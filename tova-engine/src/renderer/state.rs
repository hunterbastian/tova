use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3, Vec4};
use wgpu::util::DeviceExt;
use winit::window::Window;

use super::camera::Camera;
use super::settings::{QualityPreset, RenderSettings};
use super::vertex::Vertex;
use crate::voxel::block::Block;
use crate::voxel::chunk::{world_seed_from_env, CHUNK_SIZE, WORLD_HEIGHT};
use crate::voxel::{Chunk, VoxelMesher};

const HDR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const SHADOW_CASCADES: usize = 2;
const CASCADE_SPLITS: [f32; SHADOW_CASCADES] = [40.0, 140.0];

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct FrameUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub time: f32,
    pub exposure: f32,
    pub render_scale: f32,
    pub near_plane: f32,
    pub far_plane: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct LightingUniform {
    pub sun_direction: [f32; 3],
    pub sun_intensity: f32,
    pub sun_color: [f32; 3],
    pub ambient: f32,
    pub fog_color: [f32; 3],
    pub fog_base_density: f32,
    pub fog_height_falloff: f32,
    pub fog_enabled: f32,
    pub volumetric_strength: f32,
    pub shader_pack_enabled: f32,
    pub water_wave_strength: f32,
    pub water_specular: f32,
    pub day_phase: f32,
    pub _pad0: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ShadowUniform {
    pub light_view_proj: [[[f32; 4]; 4]; SHADOW_CASCADES],
    pub cascade_splits: [f32; 4],
    pub shadow_params: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ShadowPassUniform {
    pub light_view_proj: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct PostUniform {
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
    pub bloom_enabled: f32,
    pub volumetric_enabled: f32,
    pub color_grade_amount: f32,
    pub vignette_strength: f32,
    pub dither_strength: f32,
    pub _pad0: f32,
    pub sun_screen_pos: [f32; 2],
    pub sun_glare_strength: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub time: f32,
    pub volumetric_decay: f32,
    pub volumetric_weight: f32,
    pub volumetric_density: f32,
    pub volumetric_steps: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct BlurUniform {
    pub direction: [f32; 2],
    pub texel_size: [f32; 2],
}

pub struct ChunkMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayMode {
    None,
    Pause { god_mode: bool },
    Title,
}

struct TextureTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

struct ShadowTextures {
    _texture: wgpu::Texture,
    array_view: wgpu::TextureView,
    cascade_views: [wgpu::TextureView; SHADOW_CASCADES],
    resolution: u32,
}

struct SceneTargets {
    depth: TextureTarget,
    hdr: TextureTarget,
    bloom_half_a: TextureTarget,
    bloom_half_b: TextureTarget,
    bloom_quarter_a: TextureTarget,
    bloom_quarter_b: TextureTarget,
}

struct PostBindGroups {
    extract: wgpu::BindGroup,
    blur_half_from_a: wgpu::BindGroup,
    blur_half_from_b: wgpu::BindGroup,
    blur_quarter_from_half: wgpu::BindGroup,
    blur_quarter_from_a: wgpu::BindGroup,
    composite: wgpu::BindGroup,
}

struct PostBindGroupInputs<'a> {
    extract_layout: &'a wgpu::BindGroupLayout,
    blur_layout: &'a wgpu::BindGroupLayout,
    composite_layout: &'a wgpu::BindGroupLayout,
    sampler: &'a wgpu::Sampler,
    post_uniform_buffer: &'a wgpu::Buffer,
    blur_uniform_buffer: &'a wgpu::Buffer,
    scene: &'a SceneTargets,
}

pub struct RenderState {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub camera: Camera,

    settings: RenderSettings,
    fog_enabled: bool,
    elapsed_time: f32,
    time_of_day: f32,

    pub chunk_meshes: Vec<ChunkMesh>,

    frame_uniform: FrameUniform,
    frame_buffer: wgpu::Buffer,
    frame_bind_group: wgpu::BindGroup,

    lighting_uniform: LightingUniform,
    lighting_buffer: wgpu::Buffer,
    lighting_bind_group: wgpu::BindGroup,

    shadow_uniform: ShadowUniform,
    shadow_uniform_buffer: wgpu::Buffer,
    shadow_bind_group: wgpu::BindGroup,

    shadow_pass_uniform: ShadowPassUniform,
    shadow_pass_buffer: wgpu::Buffer,
    shadow_pass_bind_group: wgpu::BindGroup,

    post_uniform: PostUniform,
    post_uniform_buffer: wgpu::Buffer,

    blur_uniform: BlurUniform,
    blur_uniform_buffer: wgpu::Buffer,

    shadow_bind_group_layout: wgpu::BindGroupLayout,
    post_extract_bind_group_layout: wgpu::BindGroupLayout,
    post_blur_bind_group_layout: wgpu::BindGroupLayout,
    post_composite_bind_group_layout: wgpu::BindGroupLayout,

    shadow_sampler: wgpu::Sampler,
    post_sampler: wgpu::Sampler,
    empty_bind_group: wgpu::BindGroup,

    shadow_textures: ShadowTextures,
    scene_targets: SceneTargets,
    post_bind_groups: PostBindGroups,

    world_pipeline: wgpu::RenderPipeline,
    sun_pipeline: wgpu::RenderPipeline,
    shadow_pipeline: wgpu::RenderPipeline,
    overlay_pipeline: wgpu::RenderPipeline,
    post_extract_pipeline: wgpu::RenderPipeline,
    post_blur_pipeline: wgpu::RenderPipeline,
    post_composite_pipeline: wgpu::RenderPipeline,

    sun_vertex_buffer: wgpu::Buffer,
    sun_index_buffer: wgpu::Buffer,
    sun_num_indices: u32,

    overlay_vertex_buffer: wgpu::Buffer,
    overlay_index_buffer: wgpu::Buffer,
    overlay_num_indices: u32,
    overlay_mode: OverlayMode,
}

impl RenderState {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        #[cfg(target_os = "macos")]
        let primary_backends = wgpu::Backends::METAL;
        #[cfg(not(target_os = "macos"))]
        let primary_backends = wgpu::Backends::VULKAN;

        let (surface, adapter) =
            match request_surface_and_adapter(window.clone(), primary_backends).await {
                Some(ok) => ok,
                None => {
                    eprintln!(
                        "Primary backend {:?} unavailable. Falling back to all backends.",
                        primary_backends
                    );
                    request_surface_and_adapter(window, wgpu::Backends::all())
                        .await
                        .expect("Failed to find a GPU adapter with fallback backends")
                }
            };
        let adapter_info = adapter.get_info();
        println!(
            "Using {:?} backend on adapter '{}'",
            adapter_info.backend, adapter_info.name
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("tova_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

        let mut settings = RenderSettings::default();
        settings.shadow_resolution = clamp_shadow_resolution(&device, settings.shadow_resolution);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: present_mode_for_vsync(settings.vsync),
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let camera = Camera::new(size.width as f32 / size.height as f32);

        let frame_uniform = FrameUniform {
            view_proj: camera.build_view_proj().view_proj,
            camera_pos: camera.position.to_array(),
            time: 0.0,
            exposure: 1.0,
            render_scale: settings.render_scale,
            near_plane: camera.z_near,
            far_plane: camera.z_far,
        };
        let frame_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("frame_uniform_buffer"),
            contents: bytemuck::cast_slice(&[frame_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let lighting_uniform = LightingUniform {
            sun_direction: Vec3::new(0.4, 0.7, 0.2).normalize().to_array(),
            sun_intensity: 1.0,
            sun_color: [0.88, 0.84, 0.76],
            ambient: 0.27,
            fog_color: [0.60, 0.62, 0.66],
            fog_base_density: 0.012,
            fog_height_falloff: 0.016,
            fog_enabled: 1.0,
            volumetric_strength: settings.volumetric_strength(),
            shader_pack_enabled: if settings.shader_pack_enabled {
                1.0
            } else {
                0.0
            },
            water_wave_strength: 0.07,
            water_specular: 0.55,
            day_phase: 0.24,
            _pad0: 0.0,
        };
        let lighting_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lighting_uniform_buffer"),
            contents: bytemuck::cast_slice(&[lighting_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shadow_uniform = ShadowUniform {
            light_view_proj: [[[0.0; 4]; 4]; SHADOW_CASCADES],
            cascade_splits: [CASCADE_SPLITS[0], CASCADE_SPLITS[1], 0.0, 0.0],
            shadow_params: [
                settings.shadow_resolution as f32,
                settings.pcf_radius,
                settings.shadow_bias(),
                if settings.shadow_enabled { 1.0 } else { 0.0 },
            ],
        };
        let shadow_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_uniform_buffer"),
            contents: bytemuck::cast_slice(&[shadow_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shadow_pass_uniform = ShadowPassUniform {
            light_view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        };
        let shadow_pass_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_pass_uniform_buffer"),
            contents: bytemuck::cast_slice(&[shadow_pass_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let post_uniform = PostUniform {
            bloom_threshold: settings.bloom_threshold(),
            bloom_intensity: settings.bloom_intensity(),
            bloom_enabled: if settings.bloom_enabled { 1.0 } else { 0.0 },
            volumetric_enabled: if settings.volumetric_enabled {
                1.0
            } else {
                0.0
            },
            color_grade_amount: 0.24,
            vignette_strength: 0.20,
            dither_strength: 1.0 / 255.0,
            _pad0: 0.0,
            sun_screen_pos: [0.5, 0.32],
            sun_glare_strength: 0.12,
            near_plane: camera.z_near,
            far_plane: camera.z_far,
            time: 0.0,
            volumetric_decay: 0.95,
            volumetric_weight: 0.58,
            volumetric_density: 0.86,
            volumetric_steps: 12.0,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        let post_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("post_uniform_buffer"),
            contents: bytemuck::cast_slice(&[post_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let blur_uniform = BlurUniform {
            direction: [1.0, 0.0],
            texel_size: [
                1.0 / size.width.max(1) as f32,
                1.0 / size.height.max(1) as f32,
            ],
        };
        let blur_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("blur_uniform_buffer"),
            contents: bytemuck::cast_slice(&[blur_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let frame_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("frame_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let lighting_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("lighting_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let shadow_pass_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow_pass_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let post_extract_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("post_extract_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let post_blur_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("post_blur_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let post_composite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("post_composite_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let empty_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("empty_bind_group_layout"),
                entries: &[],
            });
        let empty_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("empty_bind_group"),
            layout: &empty_bind_group_layout,
            entries: &[],
        });

        let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame_bind_group"),
            layout: &frame_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_buffer.as_entire_binding(),
            }],
        });

        let lighting_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting_bind_group"),
            layout: &lighting_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lighting_buffer.as_entire_binding(),
            }],
        });

        let shadow_pass_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_pass_bind_group"),
            layout: &shadow_pass_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_pass_buffer.as_entire_binding(),
            }],
        });

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 1.0,
            ..Default::default()
        });

        let post_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shadow_textures =
            create_shadow_textures(&device, settings.shadow_resolution, Some("shadow_map"));
        let shadow_bind_group = create_shadow_bind_group(
            &device,
            &shadow_bind_group_layout,
            &shadow_textures.array_view,
            &shadow_sampler,
            &shadow_uniform_buffer,
        );

        let scene_targets =
            create_scene_targets(&device, size, settings.render_scale, Some("scene_targets"));

        let post_bind_groups = create_post_bind_groups(
            &device,
            PostBindGroupInputs {
                extract_layout: &post_extract_bind_group_layout,
                blur_layout: &post_blur_bind_group_layout,
                composite_layout: &post_composite_bind_group_layout,
                sampler: &post_sampler,
                post_uniform_buffer: &post_uniform_buffer,
                blur_uniform_buffer: &blur_uniform_buffer,
                scene: &scene_targets,
            },
        );

        let world_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("world_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/world.wgsl").into(),
            ),
        });

        let shadow_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shadow_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/shadow.wgsl").into(),
            ),
        });

        let post_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../assets/shaders/post.wgsl").into()),
        });

        let world_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world_pipeline_layout"),
            bind_group_layouts: &[
                &frame_bind_group_layout,
                &lighting_bind_group_layout,
                &shadow_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let world_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world_pipeline"),
            layout: Some(&world_layout),
            vertex: wgpu::VertexState {
                module: &world_shader,
                entry_point: Some("vs_world"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &world_shader,
                entry_point: Some("fs_world"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sun_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sun_pipeline_layout"),
            bind_group_layouts: &[&frame_bind_group_layout, &lighting_bind_group_layout],
            push_constant_ranges: &[],
        });

        let sun_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sun_pipeline"),
            layout: Some(&sun_layout),
            vertex: wgpu::VertexState {
                module: &world_shader,
                entry_point: Some("vs_world"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &world_shader,
                entry_point: Some("fs_sun"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow_pipeline_layout"),
            bind_group_layouts: &[&shadow_pass_bind_group_layout],
            push_constant_ranges: &[],
        });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow_pipeline"),
            layout: Some(&shadow_layout),
            vertex: wgpu::VertexState {
                module: &shadow_shader,
                entry_point: Some("vs_shadow"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let overlay_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("overlay_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let overlay_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("overlay_pipeline"),
            layout: Some(&overlay_layout),
            vertex: wgpu::VertexState {
                module: &world_shader,
                entry_point: Some("vs_overlay"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &world_shader,
                entry_point: Some("fs_overlay"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let post_extract_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post_extract_layout"),
            bind_group_layouts: &[&post_extract_bind_group_layout],
            push_constant_ranges: &[],
        });
        let post_extract_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("post_extract_pipeline"),
                layout: Some(&post_extract_layout),
                vertex: wgpu::VertexState {
                    module: &post_shader,
                    entry_point: Some("vs_fullscreen"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &post_shader,
                    entry_point: Some("fs_extract_bloom"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: HDR_FORMAT,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let post_blur_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post_blur_layout"),
            bind_group_layouts: &[&empty_bind_group_layout, &post_blur_bind_group_layout],
            push_constant_ranges: &[],
        });
        let post_blur_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("post_blur_pipeline"),
            layout: Some(&post_blur_layout),
            vertex: wgpu::VertexState {
                module: &post_shader,
                entry_point: Some("vs_fullscreen"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &post_shader,
                entry_point: Some("fs_blur"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: HDR_FORMAT,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let post_composite_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("post_composite_layout"),
                bind_group_layouts: &[
                    &empty_bind_group_layout,
                    &empty_bind_group_layout,
                    &post_composite_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });
        let post_composite_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("post_composite_pipeline"),
                layout: Some(&post_composite_layout),
                vertex: wgpu::VertexState {
                    module: &post_shader,
                    entry_point: Some("vs_fullscreen"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &post_shader,
                    entry_point: Some("fs_composite"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        let sun_vertices = build_sun_geometry(
            camera.position,
            Vec3::from_array(lighting_uniform.sun_direction),
            Vec3::from_array(lighting_uniform.sun_color),
        );
        let sun_indices: [u32; 6] = [0, 1, 2, 0, 2, 3];

        let sun_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sun_vertex_buffer"),
            contents: bytemuck::cast_slice(&sun_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let sun_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sun_index_buffer"),
            contents: bytemuck::cast_slice(&sun_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let (overlay_vertices, overlay_indices) =
            build_overlay_geometry(OverlayMode::Pause { god_mode: false }, 0.0);
        let overlay_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("overlay_vertex_buffer"),
            contents: bytemuck::cast_slice(&overlay_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let overlay_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("overlay_index_buffer"),
            contents: bytemuck::cast_slice(&overlay_indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let chunk_meshes = generate_world_chunks(&device);

        let mut state = Self {
            surface,
            device,
            queue,
            config,
            size,
            camera,
            settings,
            fog_enabled: true,
            elapsed_time: 0.0,
            time_of_day: 0.24,
            chunk_meshes,
            frame_uniform,
            frame_buffer,
            frame_bind_group,
            lighting_uniform,
            lighting_buffer,
            lighting_bind_group,
            shadow_uniform,
            shadow_uniform_buffer,
            shadow_bind_group,
            shadow_pass_uniform,
            shadow_pass_buffer,
            shadow_pass_bind_group,
            post_uniform,
            post_uniform_buffer,
            blur_uniform,
            blur_uniform_buffer,
            shadow_bind_group_layout,
            post_extract_bind_group_layout,
            post_blur_bind_group_layout,
            post_composite_bind_group_layout,
            shadow_sampler,
            post_sampler,
            empty_bind_group,
            shadow_textures,
            scene_targets,
            post_bind_groups,
            world_pipeline,
            sun_pipeline,
            shadow_pipeline,
            overlay_pipeline,
            post_extract_pipeline,
            post_blur_pipeline,
            post_composite_pipeline,
            sun_vertex_buffer,
            sun_index_buffer,
            sun_num_indices: sun_indices.len() as u32,
            overlay_vertex_buffer,
            overlay_index_buffer,
            overlay_num_indices: overlay_indices.len() as u32,
            overlay_mode: OverlayMode::None,
        };

        state.update(0.0);
        state
    }

    pub fn settings(&self) -> RenderSettings {
        self.settings
    }

    pub fn set_quality_preset(&mut self, preset: QualityPreset) {
        let previous = self.settings;
        let mut next = RenderSettings::from_preset(preset);
        next.vsync = self.settings.vsync;
        next.shadow_resolution = clamp_shadow_resolution(&self.device, next.shadow_resolution);
        self.settings = next;

        self.sync_surface_present_mode();

        if previous.shadow_resolution != self.settings.shadow_resolution {
            self.recreate_shadow_resources();
        }

        if (previous.render_scale - self.settings.render_scale).abs() > f32::EPSILON {
            self.recreate_scene_targets();
        }

        self.update(0.0);
    }

    pub fn set_vsync(&mut self, enabled: bool) {
        self.settings.vsync = enabled;
        self.sync_surface_present_mode();
    }

    pub fn set_shader_pack_enabled(&mut self, enabled: bool) {
        self.settings.shader_pack_enabled = enabled;
        self.update(0.0);
    }

    #[allow(dead_code)]
    pub fn update_time_of_day(&mut self, t: f32) {
        self.time_of_day = t.rem_euclid(1.0);
        self.update(0.0);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.camera.aspect = new_size.width as f32 / new_size.height as f32;
        self.recreate_scene_targets();
        self.update(0.0);
    }

    pub fn set_fog(&mut self, enabled: bool) {
        self.fog_enabled = enabled;
        self.update(0.0);
    }

    pub fn set_overlay_mode(&mut self, mode: OverlayMode) {
        self.overlay_mode = mode;
        self.write_overlay_geometry();
    }

    pub fn update(&mut self, dt: f32) {
        let dt = dt.max(0.0);
        self.elapsed_time += dt;

        if self.settings.day_cycle_enabled && self.settings.shader_pack_enabled {
            self.time_of_day = (self.time_of_day + dt * 0.006).fract();
        }

        let (sun_direction, sun_color, sun_intensity, ambient, fog_color, fog_density, glare) =
            self.compute_lighting_values();

        self.frame_uniform = FrameUniform {
            view_proj: self.camera.build_view_proj().view_proj,
            camera_pos: self.camera.position.to_array(),
            time: self.elapsed_time,
            exposure: 1.0,
            render_scale: self.settings.render_scale,
            near_plane: self.camera.z_near,
            far_plane: self.camera.z_far,
        };

        self.lighting_uniform = LightingUniform {
            sun_direction: sun_direction.to_array(),
            sun_intensity,
            sun_color: sun_color.to_array(),
            ambient,
            fog_color: fog_color.to_array(),
            fog_base_density: fog_density,
            fog_height_falloff: 0.018,
            fog_enabled: if self.fog_enabled { 1.0 } else { 0.0 },
            volumetric_strength: self.settings.volumetric_strength(),
            shader_pack_enabled: if self.settings.shader_pack_enabled {
                1.0
            } else {
                0.0
            },
            water_wave_strength: 0.08,
            water_specular: 0.62,
            day_phase: self.time_of_day,
            _pad0: 0.0,
        };

        let shadow_mats = compute_shadow_matrices(
            &self.camera,
            sun_direction,
            self.shadow_textures.resolution as f32,
        );
        self.shadow_uniform.light_view_proj = shadow_mats;
        self.shadow_uniform.cascade_splits = [CASCADE_SPLITS[0], CASCADE_SPLITS[1], 0.0, 0.0];
        self.shadow_uniform.shadow_params = [
            self.shadow_textures.resolution as f32,
            self.settings.pcf_radius,
            self.settings.shadow_bias(),
            if self.settings.shadow_enabled && self.settings.shader_pack_enabled {
                1.0
            } else {
                0.0
            },
        ];

        let sun_screen = project_sun_to_screen(
            self.frame_uniform.view_proj,
            self.camera.position,
            sun_direction,
        );

        self.post_uniform = PostUniform {
            bloom_threshold: self.settings.bloom_threshold(),
            bloom_intensity: self.settings.bloom_intensity(),
            bloom_enabled: if self.settings.bloom_enabled && self.settings.shader_pack_enabled {
                1.0
            } else {
                0.0
            },
            volumetric_enabled: if self.settings.volumetric_enabled
                && self.settings.shader_pack_enabled
            {
                1.0
            } else {
                0.0
            },
            color_grade_amount: 0.24,
            vignette_strength: 0.20,
            dither_strength: 1.0 / 255.0,
            _pad0: 0.0,
            sun_screen_pos: sun_screen.to_array(),
            sun_glare_strength: glare,
            near_plane: self.camera.z_near,
            far_plane: self.camera.z_far,
            time: self.elapsed_time,
            volumetric_decay: 0.95,
            volumetric_weight: 0.55 + self.settings.volumetric_strength() * 0.5,
            volumetric_density: 0.84,
            volumetric_steps: if self.settings.volumetric_enabled {
                14.0
            } else {
                8.0
            },
            _pad1: 0.0,
            _pad2: 0.0,
        };

        self.queue.write_buffer(
            &self.frame_buffer,
            0,
            bytemuck::cast_slice(&[self.frame_uniform]),
        );
        self.queue.write_buffer(
            &self.lighting_buffer,
            0,
            bytemuck::cast_slice(&[self.lighting_uniform]),
        );
        self.queue.write_buffer(
            &self.shadow_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.shadow_uniform]),
        );
        self.queue.write_buffer(
            &self.post_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.post_uniform]),
        );

        let sun_vertices = build_sun_geometry(self.camera.position, sun_direction, sun_color);
        self.queue.write_buffer(
            &self.sun_vertex_buffer,
            0,
            bytemuck::cast_slice(&sun_vertices),
        );

        if matches!(self.overlay_mode, OverlayMode::Title) {
            self.write_overlay_geometry();
        }
    }

    pub fn render(&mut self, draw_overlay: bool) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        self.render_shadow_pass(&mut encoder);
        self.render_world_pass(&mut encoder);
        self.render_post_passes(&mut encoder, &output_view);

        if draw_overlay {
            self.render_overlay_pass(&mut encoder, &output_view);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }

    fn render_shadow_pass(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if !self.settings.shadow_enabled || !self.settings.shader_pack_enabled {
            return;
        }

        for cascade in 0..SHADOW_CASCADES {
            self.shadow_pass_uniform.light_view_proj = self.shadow_uniform.light_view_proj[cascade];
            self.queue.write_buffer(
                &self.shadow_pass_buffer,
                0,
                bytemuck::cast_slice(&[self.shadow_pass_uniform]),
            );

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.shadow_textures.cascade_views[cascade],
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.shadow_pipeline);
            pass.set_bind_group(0, &self.shadow_pass_bind_group, &[]);

            for mesh in &self.chunk_meshes {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
        }
    }

    fn render_world_pass(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let sky = Vec3::from_array(self.lighting_uniform.fog_color);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("world_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.scene_targets.hdr.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: sky.x as f64,
                        g: sky.y as f64,
                        b: sky.z as f64,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.scene_targets.depth.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.world_pipeline);
        pass.set_bind_group(0, &self.frame_bind_group, &[]);
        pass.set_bind_group(1, &self.lighting_bind_group, &[]);
        pass.set_bind_group(2, &self.shadow_bind_group, &[]);

        for mesh in &self.chunk_meshes {
            pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
            pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
        }

        pass.set_pipeline(&self.sun_pipeline);
        pass.set_bind_group(0, &self.frame_bind_group, &[]);
        pass.set_bind_group(1, &self.lighting_bind_group, &[]);
        pass.set_vertex_buffer(0, self.sun_vertex_buffer.slice(..));
        pass.set_index_buffer(self.sun_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.sun_num_indices, 0, 0..1);
    }

    fn render_post_passes(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
    ) {
        if self.settings.bloom_enabled && self.settings.shader_pack_enabled {
            self.fullscreen_pass(
                encoder,
                "post_extract_pass",
                &self.scene_targets.bloom_half_a.view,
                wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                &self.post_extract_pipeline,
                &self.post_bind_groups.extract,
                0,
            );

            self.update_blur_uniform(
                [1.0, 0.0],
                self.scene_targets.bloom_half_a.width,
                self.scene_targets.bloom_half_a.height,
            );
            self.fullscreen_pass(
                encoder,
                "post_blur_half_h",
                &self.scene_targets.bloom_half_b.view,
                wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                &self.post_blur_pipeline,
                &self.post_bind_groups.blur_half_from_a,
                1,
            );

            self.update_blur_uniform(
                [0.0, 1.0],
                self.scene_targets.bloom_half_b.width,
                self.scene_targets.bloom_half_b.height,
            );
            self.fullscreen_pass(
                encoder,
                "post_blur_half_v",
                &self.scene_targets.bloom_half_a.view,
                wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                &self.post_blur_pipeline,
                &self.post_bind_groups.blur_half_from_b,
                1,
            );

            if self.settings.bloom_quarter_enabled {
                self.update_blur_uniform(
                    [1.0, 0.0],
                    self.scene_targets.bloom_half_a.width,
                    self.scene_targets.bloom_half_a.height,
                );
                self.fullscreen_pass(
                    encoder,
                    "post_blur_quarter_h",
                    &self.scene_targets.bloom_quarter_a.view,
                    wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    &self.post_blur_pipeline,
                    &self.post_bind_groups.blur_quarter_from_half,
                    1,
                );

                self.update_blur_uniform(
                    [0.0, 1.0],
                    self.scene_targets.bloom_quarter_a.width,
                    self.scene_targets.bloom_quarter_a.height,
                );
                self.fullscreen_pass(
                    encoder,
                    "post_blur_quarter_v",
                    &self.scene_targets.bloom_quarter_b.view,
                    wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    &self.post_blur_pipeline,
                    &self.post_bind_groups.blur_quarter_from_a,
                    1,
                );
            } else {
                self.clear_color_target(
                    encoder,
                    "post_clear_quarter",
                    &self.scene_targets.bloom_quarter_b.view,
                );
            }
        } else {
            self.clear_color_target(
                encoder,
                "post_clear_half",
                &self.scene_targets.bloom_half_a.view,
            );
            self.clear_color_target(
                encoder,
                "post_clear_quarter",
                &self.scene_targets.bloom_quarter_b.view,
            );
        }

        self.fullscreen_pass(
            encoder,
            "post_composite_pass",
            output_view,
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            &self.post_composite_pipeline,
            &self.post_bind_groups.composite,
            2,
        );
    }

    fn render_overlay_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
    ) {
        if self.overlay_mode == OverlayMode::None {
            return;
        }

        let mut overlay_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("overlay_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        overlay_pass.set_pipeline(&self.overlay_pipeline);
        overlay_pass.set_vertex_buffer(0, self.overlay_vertex_buffer.slice(..));
        overlay_pass.set_index_buffer(
            self.overlay_index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        overlay_pass.draw_indexed(0..self.overlay_num_indices, 0, 0..1);
    }

    #[allow(clippy::too_many_arguments)]
    fn fullscreen_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        label: &str,
        target_view: &wgpu::TextureView,
        load_op: wgpu::LoadOp<wgpu::Color>,
        pipeline: &wgpu::RenderPipeline,
        bind_group: &wgpu::BindGroup,
        bind_group_index: u32,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: load_op,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(pipeline);
        for index in 0..bind_group_index {
            pass.set_bind_group(index, &self.empty_bind_group, &[]);
        }
        pass.set_bind_group(bind_group_index, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    fn clear_color_target(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        label: &str,
        target_view: &wgpu::TextureView,
    ) {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(label),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }

    fn update_blur_uniform(&mut self, direction: [f32; 2], width: u32, height: u32) {
        self.blur_uniform = BlurUniform {
            direction,
            texel_size: [1.0 / width.max(1) as f32, 1.0 / height.max(1) as f32],
        };
        self.queue.write_buffer(
            &self.blur_uniform_buffer,
            0,
            bytemuck::cast_slice(&[self.blur_uniform]),
        );
    }

    fn write_overlay_geometry(&mut self) {
        let (vertices, _indices) = build_overlay_geometry(self.overlay_mode, self.elapsed_time);
        self.queue.write_buffer(
            &self.overlay_vertex_buffer,
            0,
            bytemuck::cast_slice(&vertices),
        );
    }

    fn sync_surface_present_mode(&mut self) {
        self.config.present_mode = present_mode_for_vsync(self.settings.vsync);
        self.surface.configure(&self.device, &self.config);
    }

    fn recreate_scene_targets(&mut self) {
        self.scene_targets = create_scene_targets(
            &self.device,
            self.size,
            self.settings.render_scale,
            Some("scene_targets"),
        );
        self.post_bind_groups = create_post_bind_groups(
            &self.device,
            PostBindGroupInputs {
                extract_layout: &self.post_extract_bind_group_layout,
                blur_layout: &self.post_blur_bind_group_layout,
                composite_layout: &self.post_composite_bind_group_layout,
                sampler: &self.post_sampler,
                post_uniform_buffer: &self.post_uniform_buffer,
                blur_uniform_buffer: &self.blur_uniform_buffer,
                scene: &self.scene_targets,
            },
        );
    }

    fn recreate_shadow_resources(&mut self) {
        self.shadow_textures = create_shadow_textures(
            &self.device,
            self.settings.shadow_resolution,
            Some("shadow_map"),
        );
        self.shadow_bind_group = create_shadow_bind_group(
            &self.device,
            &self.shadow_bind_group_layout,
            &self.shadow_textures.array_view,
            &self.shadow_sampler,
            &self.shadow_uniform_buffer,
        );
    }

    fn compute_lighting_values(&self) -> (Vec3, Vec3, f32, f32, Vec3, f32, f32) {
        let phase = self.time_of_day;
        let sun_direction = sun_direction_from_phase(phase);
        let sun_height = sun_direction.y;
        let daylight = ((sun_height + 0.2) / 1.2).clamp(0.0, 1.0);
        let dusk = (1.0 - daylight).powf(1.45);

        let sun_color_day = Vec3::new(0.96, 0.92, 0.84);
        let sun_color_dusk = Vec3::new(1.10, 0.66, 0.44);
        let sun_color = sun_color_day.lerp(sun_color_dusk, dusk * 0.9);

        let sun_intensity = 0.35 + daylight * 0.95;
        let ambient = 0.16 + daylight * 0.24;

        let fog_day = Vec3::new(0.60, 0.64, 0.70);
        let fog_dusk = Vec3::new(0.58, 0.50, 0.46);
        let fog_color = fog_day.lerp(fog_dusk, dusk * 0.85);

        let fog_density = 0.010 + (1.0 - daylight) * 0.006;
        let glare = 0.06 + daylight * 0.07;

        (
            sun_direction,
            sun_color,
            sun_intensity,
            ambient,
            fog_color,
            fog_density,
            glare,
        )
    }
}

async fn request_surface_and_adapter(
    window: Arc<Window>,
    backends: wgpu::Backends,
) -> Option<(wgpu::Surface<'static>, wgpu::Adapter)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends,
        ..Default::default()
    });
    let surface = match instance.create_surface(window) {
        Ok(surface) => surface,
        Err(err) => {
            eprintln!(
                "Failed to create surface with backend set {:?}: {:?}",
                backends, err
            );
            return None;
        }
    };
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await?;
    Some((surface, adapter))
}

fn create_shadow_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    shadow_array_view: &wgpu::TextureView,
    shadow_sampler: &wgpu::Sampler,
    shadow_uniform_buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("shadow_bind_group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(shadow_array_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(shadow_sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: shadow_uniform_buffer.as_entire_binding(),
            },
        ],
    })
}

fn create_shadow_textures(
    device: &wgpu::Device,
    resolution: u32,
    label_prefix: Option<&str>,
) -> ShadowTextures {
    let label = label_prefix.unwrap_or("shadow");
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(&format!("{label}_texture")),
        size: wgpu::Extent3d {
            width: resolution,
            height: resolution,
            depth_or_array_layers: SHADOW_CASCADES as u32,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let array_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some(&format!("{label}_array_view")),
        format: Some(wgpu::TextureFormat::Depth32Float),
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        aspect: wgpu::TextureAspect::DepthOnly,
        base_mip_level: 0,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(SHADOW_CASCADES as u32),
        usage: Some(wgpu::TextureUsages::TEXTURE_BINDING),
    });

    let mut cascade_views_vec = Vec::with_capacity(SHADOW_CASCADES);
    for cascade in 0..SHADOW_CASCADES {
        cascade_views_vec.push(texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(&format!("{label}_cascade_{cascade}")),
            format: Some(wgpu::TextureFormat::Depth32Float),
            dimension: Some(wgpu::TextureViewDimension::D2),
            aspect: wgpu::TextureAspect::DepthOnly,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: cascade as u32,
            array_layer_count: Some(1),
            usage: Some(
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ),
        }));
    }

    ShadowTextures {
        _texture: texture,
        array_view,
        cascade_views: [cascade_views_vec.remove(0), cascade_views_vec.remove(0)],
        resolution,
    }
}

fn create_scene_targets(
    device: &wgpu::Device,
    window_size: winit::dpi::PhysicalSize<u32>,
    render_scale: f32,
    label_prefix: Option<&str>,
) -> SceneTargets {
    let label = label_prefix.unwrap_or("scene");
    let (scaled_width, scaled_height) = scaled_dimensions(window_size, render_scale);

    let depth = create_texture_target(
        device,
        scaled_width,
        scaled_height,
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        &format!("{label}_depth"),
        Some(wgpu::TextureAspect::DepthOnly),
    );

    let hdr = create_texture_target(
        device,
        scaled_width,
        scaled_height,
        HDR_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        &format!("{label}_hdr"),
        Some(wgpu::TextureAspect::All),
    );

    let half_w = (scaled_width / 2).max(1);
    let half_h = (scaled_height / 2).max(1);
    let quarter_w = (half_w / 2).max(1);
    let quarter_h = (half_h / 2).max(1);

    let bloom_half_a = create_texture_target(
        device,
        half_w,
        half_h,
        HDR_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        &format!("{label}_bloom_half_a"),
        Some(wgpu::TextureAspect::All),
    );
    let bloom_half_b = create_texture_target(
        device,
        half_w,
        half_h,
        HDR_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        &format!("{label}_bloom_half_b"),
        Some(wgpu::TextureAspect::All),
    );

    let bloom_quarter_a = create_texture_target(
        device,
        quarter_w,
        quarter_h,
        HDR_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        &format!("{label}_bloom_quarter_a"),
        Some(wgpu::TextureAspect::All),
    );
    let bloom_quarter_b = create_texture_target(
        device,
        quarter_w,
        quarter_h,
        HDR_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        &format!("{label}_bloom_quarter_b"),
        Some(wgpu::TextureAspect::All),
    );

    SceneTargets {
        depth,
        hdr,
        bloom_half_a,
        bloom_half_b,
        bloom_quarter_a,
        bloom_quarter_b,
    }
}

fn create_post_bind_groups(
    device: &wgpu::Device,
    inputs: PostBindGroupInputs<'_>,
) -> PostBindGroups {
    let extract = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("post_extract_bind_group"),
        layout: inputs.extract_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&inputs.scene.hdr.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(inputs.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: inputs.post_uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let blur_half_from_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("post_blur_half_from_a"),
        layout: inputs.blur_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&inputs.scene.bloom_half_a.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(inputs.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: inputs.blur_uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let blur_half_from_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("post_blur_half_from_b"),
        layout: inputs.blur_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&inputs.scene.bloom_half_b.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(inputs.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: inputs.blur_uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let blur_quarter_from_half = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("post_blur_quarter_from_half"),
        layout: inputs.blur_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&inputs.scene.bloom_half_a.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(inputs.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: inputs.blur_uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let blur_quarter_from_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("post_blur_quarter_from_a"),
        layout: inputs.blur_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&inputs.scene.bloom_quarter_a.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(inputs.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: inputs.blur_uniform_buffer.as_entire_binding(),
            },
        ],
    });

    let composite = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("post_composite_bind_group"),
        layout: inputs.composite_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&inputs.scene.hdr.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&inputs.scene.bloom_half_a.view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&inputs.scene.bloom_quarter_b.view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(inputs.sampler),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&inputs.scene.depth.view),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: inputs.post_uniform_buffer.as_entire_binding(),
            },
        ],
    });

    PostBindGroups {
        extract,
        blur_half_from_a,
        blur_half_from_b,
        blur_quarter_from_half,
        blur_quarter_from_a,
        composite,
    }
}

fn create_texture_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    label: &str,
    aspect: Option<wgpu::TextureAspect>,
) -> TextureTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: Some(&format!("{label}_view")),
        format: Some(format),
        dimension: Some(wgpu::TextureViewDimension::D2),
        aspect: aspect.unwrap_or(wgpu::TextureAspect::All),
        base_mip_level: 0,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(1),
        usage: Some(usage),
    });

    TextureTarget {
        _texture: texture,
        view,
        width,
        height,
    }
}

fn clamp_shadow_resolution(device: &wgpu::Device, requested: u32) -> u32 {
    let max_dim = device.limits().max_texture_dimension_2d;
    requested.min(max_dim).max(256)
}

fn present_mode_for_vsync(vsync: bool) -> wgpu::PresentMode {
    if vsync {
        wgpu::PresentMode::AutoVsync
    } else {
        wgpu::PresentMode::AutoNoVsync
    }
}

fn scaled_dimensions(size: winit::dpi::PhysicalSize<u32>, render_scale: f32) -> (u32, u32) {
    let scale = render_scale.clamp(0.5, 1.0);
    let width = (size.width as f32 * scale).round().max(1.0) as u32;
    let height = (size.height as f32 * scale).round().max(1.0) as u32;
    (width, height)
}

fn sun_direction_from_phase(phase: f32) -> Vec3 {
    let theta = phase * std::f32::consts::TAU;
    let elevation = (theta.sin() * 0.72 + 0.12).clamp(-0.30, 0.96);
    let azimuth = theta * 0.37 + 1.15;
    let horizontal = (1.0 - elevation * elevation).max(0.05).sqrt();
    Vec3::new(
        azimuth.cos() * horizontal,
        elevation,
        azimuth.sin() * horizontal,
    )
    .normalize()
}

fn compute_shadow_matrices(
    camera: &Camera,
    sun_direction: Vec3,
    shadow_map_size: f32,
) -> [[[f32; 4]; 4]; SHADOW_CASCADES] {
    let mut matrices = [[[0.0; 4]; 4]; SHADOW_CASCADES];
    let forward = camera.forward();

    for cascade in 0..SHADOW_CASCADES {
        let near = if cascade == 0 {
            camera.z_near.max(0.1)
        } else {
            CASCADE_SPLITS[cascade - 1]
        };
        let far = CASCADE_SPLITS[cascade];

        let radius = far * 1.30;
        let center = camera.position + forward * ((near + far) * 0.5);
        let light_distance = radius * 2.4;

        let up = if sun_direction.dot(Vec3::Y).abs() > 0.95 {
            Vec3::Z
        } else {
            Vec3::Y
        };

        let mut light_pos = center - sun_direction * light_distance;

        // Snap the light camera to texel-sized steps for more stable shadows while moving.
        let texel_world = (radius * 2.0) / shadow_map_size.max(1.0);
        light_pos.x = (light_pos.x / texel_world).round() * texel_world;
        light_pos.y = (light_pos.y / texel_world).round() * texel_world;
        light_pos.z = (light_pos.z / texel_world).round() * texel_world;

        let view = Mat4::look_at_rh(light_pos, center, up);
        let proj = Mat4::orthographic_rh(
            -radius,
            radius,
            -radius,
            radius,
            -radius * 4.0,
            radius * 4.0,
        );

        matrices[cascade] = (proj * view).to_cols_array_2d();
    }

    matrices
}

fn project_sun_to_screen(view_proj: [[f32; 4]; 4], camera_pos: Vec3, sun_direction: Vec3) -> Vec2 {
    let vp = Mat4::from_cols_array_2d(&view_proj);
    let sun_point = camera_pos + sun_direction * 240.0;
    let clip = vp * Vec4::new(sun_point.x, sun_point.y, sun_point.z, 1.0);

    if clip.w.abs() <= f32::EPSILON {
        return Vec2::new(0.5, 0.5);
    }

    let ndc = clip.truncate() / clip.w;
    Vec2::new(ndc.x * 0.5 + 0.5, -ndc.y * 0.5 + 0.5).clamp(Vec2::ZERO, Vec2::ONE)
}

fn build_sun_geometry(camera_pos: Vec3, sun_dir: Vec3, sun_color: Vec3) -> [Vertex; 4] {
    let position = camera_pos + sun_dir * 220.0;
    let mut right = sun_dir.cross(Vec3::Y);
    if right.length_squared() <= f32::EPSILON {
        right = sun_dir.cross(Vec3::X);
    }
    right = right.normalize() * 14.0;
    let up = sun_dir.cross(right).normalize() * 14.0;

    let color = (sun_color * 0.95).to_array();
    let normal = [0.0, 0.0, 0.0];

    [
        Vertex {
            position: (position - right - up).to_array(),
            color,
            normal,
        },
        Vertex {
            position: (position + right - up).to_array(),
            color,
            normal,
        },
        Vertex {
            position: (position + right + up).to_array(),
            color,
            normal,
        },
        Vertex {
            position: (position - right + up).to_array(),
            color,
            normal,
        },
    ]
}

// Button bounds in NDC — used for hit testing and geometry
pub const BTN_LEFT: f32 = -0.25;
pub const BTN_RIGHT: f32 = 0.25;
pub const BTN_BOTTOM: f32 = -0.08;
pub const BTN_TOP: f32 = 0.08;

fn build_overlay_geometry(mode: OverlayMode, elapsed_time: f32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let (bg_alpha_value, bg_color) = match mode {
        OverlayMode::Title => (0.78_f32, [0.02_f32, 0.03, 0.05]),
        OverlayMode::Pause { .. } => (0.65_f32, [0.0_f32, 0.0, 0.0]),
        OverlayMode::None => (0.0_f32, [0.0_f32, 0.0, 0.0]),
    };
    let bg_alpha = [bg_alpha_value, 0.0, 0.0];
    vertices.push(Vertex {
        position: [-1.0, -1.0, 0.0],
        color: bg_color,
        normal: bg_alpha,
    });
    vertices.push(Vertex {
        position: [1.0, -1.0, 0.0],
        color: bg_color,
        normal: bg_alpha,
    });
    vertices.push(Vertex {
        position: [1.0, 1.0, 0.0],
        color: bg_color,
        normal: bg_alpha,
    });
    vertices.push(Vertex {
        position: [-1.0, 1.0, 0.0],
        color: bg_color,
        normal: bg_alpha,
    });
    indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);

    let (left, right, bottom, top, panel_color, panel_alpha) = match mode {
        OverlayMode::Pause { god_mode } => {
            let color = if god_mode {
                [0.2_f32, 0.6, 0.3]
            } else {
                [0.35_f32, 0.35, 0.35]
            };
            (BTN_LEFT, BTN_RIGHT, BTN_BOTTOM, BTN_TOP, color, 0.85_f32)
        }
        OverlayMode::Title => {
            let pulse = (elapsed_time * 1.8).sin() * 0.5 + 0.5;
            let color = [
                0.10_f32 + pulse * 0.08,
                0.15_f32 + pulse * 0.10,
                0.22_f32 + pulse * 0.14,
            ];
            (-0.58_f32, 0.58_f32, -0.24_f32, 0.24_f32, color, 0.93_f32)
        }
        OverlayMode::None => (
            BTN_LEFT,
            BTN_RIGHT,
            BTN_BOTTOM,
            BTN_TOP,
            [0.0_f32, 0.0, 0.0],
            0.0_f32,
        ),
    };
    let btn_alpha = [panel_alpha, 0.0, 0.0];
    let base = vertices.len() as u32;
    vertices.push(Vertex {
        position: [left, bottom, 0.0],
        color: panel_color,
        normal: btn_alpha,
    });
    vertices.push(Vertex {
        position: [right, bottom, 0.0],
        color: panel_color,
        normal: btn_alpha,
    });
    vertices.push(Vertex {
        position: [right, top, 0.0],
        color: panel_color,
        normal: btn_alpha,
    });
    vertices.push(Vertex {
        position: [left, top, 0.0],
        color: panel_color,
        normal: btn_alpha,
    });
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);

    (vertices, indices)
}

fn generate_world_chunks(device: &wgpu::Device) -> Vec<ChunkMesh> {
    let radius = 6_i32;
    let world_seed = world_seed_from_env();
    log::info!("Generating procedural world with seed {}", world_seed);
    let mut chunks = HashMap::new();

    for cz in -radius..radius {
        for cx in -radius..radius {
            let mut chunk = Chunk::new(cx, cz);
            chunk.generate_procedural(world_seed);
            chunks.insert((cx, cz), chunk);
        }
    }

    let mut meshes = Vec::with_capacity(chunks.len());

    for cz in -radius..radius {
        for cx in -radius..radius {
            let Some(chunk) = chunks.get(&(cx, cz)) else {
                continue;
            };

            if let Some((vertices, indices)) =
                VoxelMesher::build_with_lookup(chunk, |wx, wy, wz| {
                    sample_block_from_world_chunks(&chunks, wx, wy, wz)
                })
            {
                let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("chunk_vertex"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("chunk_index"),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });
                meshes.push(ChunkMesh {
                    vertex_buffer,
                    index_buffer,
                    num_indices: indices.len() as u32,
                });
            }
        }
    }

    meshes
}

fn sample_block_from_world_chunks(
    chunks: &HashMap<(i32, i32), Chunk>,
    wx: i32,
    wy: i32,
    wz: i32,
) -> Block {
    if wy < 0 {
        return Block::Stone;
    }
    if wy >= WORLD_HEIGHT as i32 {
        return Block::Air;
    }

    let chunk_size = CHUNK_SIZE as i32;
    let cx = wx.div_euclid(chunk_size);
    let cz = wz.div_euclid(chunk_size);
    let lx = wx.rem_euclid(chunk_size) as usize;
    let lz = wz.rem_euclid(chunk_size) as usize;

    chunks
        .get(&(cx, cz))
        .map(|chunk| chunk.get(lx, wy as usize, lz))
        .unwrap_or(Block::Air)
}

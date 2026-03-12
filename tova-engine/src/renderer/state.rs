use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use glam::{Vec3, Vec4, Mat4};
use wgpu::util::DeviceExt;
use winit::window::Window;

use super::camera::{Camera, CameraUniform};
use super::vertex::Vertex;
use crate::ui::{self, UiVertex};
use crate::voxel::chunk::{SEA_LEVEL, CHUNK_SIZE, WORLD_HEIGHT};
use crate::world::ChunkManager;

// Shared constants — shader.wgsl duplicates SKY_ZENITH and SUN_DISTANCE/SUN_SIZE
const SKY_ZENITH: wgpu::Color = wgpu::Color { r: 0.52, g: 0.50, b: 0.47, a: 1.0 };
const SUN_DISTANCE: f32 = 800.0;
const SUN_SIZE: f32 = 22.0;
const RENDER_DISTANCE: i32 = 14;
const SHADOW_MAP_SIZE: u32 = 2048;
const SHADOW_CASCADE_SIZE: f32 = 80.0;

/// Test if a chunk AABB is inside the frustum (returns false if fully outside any plane).
fn chunk_in_frustum(planes: &[Vec4; 5], cx: i32, cz: i32) -> bool {
    let min_x = (cx * CHUNK_SIZE as i32) as f32;
    let min_z = (cz * CHUNK_SIZE as i32) as f32;
    let max_x = min_x + CHUNK_SIZE as f32;
    let max_z = min_z + CHUNK_SIZE as f32;
    let min_y = 0.0_f32;
    let max_y = WORLD_HEIGHT as f32;

    for p in planes {
        // Find the corner of the AABB most aligned with the plane normal (p-vertex)
        let px = if p.x > 0.0 { max_x } else { min_x };
        let py = if p.y > 0.0 { max_y } else { min_y };
        let pz = if p.z > 0.0 { max_z } else { min_z };
        if p.x * px + p.y * py + p.z * pz + p.w < 0.0 {
            return false; // entire AABB is outside this plane
        }
    }
    true
}

/// Strip sRGB from a texture format so intermediates stay in linear space.
/// The final output to the swapchain will be the only sRGB conversion.
fn linear_format(fmt: wgpu::TextureFormat) -> wgpu::TextureFormat {
    match fmt {
        wgpu::TextureFormat::Bgra8UnormSrgb => wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba8UnormSrgb => wgpu::TextureFormat::Rgba8Unorm,
        other => other,
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ShadowUniform {
    pub light_vp: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct TaaUniform {
    pub prev_view_proj: [[f32; 4]; 4],
    pub inv_view_proj: [[f32; 4]; 4],
    pub jitter: [f32; 2],
    pub feedback: f32,
    pub _pad: f32,
}

// Halton(2,3) sequence for sub-pixel jitter — low-discrepancy, covers the pixel well
const TAA_JITTER_SEQUENCE: [[f32; 2]; 8] = [
    [0.5, 0.333], [0.25, 0.667], [0.75, 0.111], [0.125, 0.444],
    [0.625, 0.778], [0.375, 0.222], [0.875, 0.556], [0.0625, 0.889],
];

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SunUniform {
    pub direction: [f32; 3],
    pub _pad: f32,
    pub color: [f32; 3],
    pub ambient: f32,
}

/// 2D vertex for crosshair overlay — includes outline flag.
/// MenuVertex is now UiVertex from the ui module.
type MenuVertex = UiVertex;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct HudUniform {
    yaw: f32,
    stamina: f32,
    god_mode: f32,
    aspect: f32,
    time: f32,
    _pad: [f32; 3],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct WeatherUniform {
    pub weather_type: f32,
    pub intensity: f32,
    pub time: f32,
    pub fog_mult: f32,
    pub sky_darken: f32,
    pub wind_x: f32,
    pub wind_z: f32,
    pub wind_strength: f32,
    pub wind_gust: f32,      // gust intensity 0..1
    pub wind_turbulence: f32, // high-freq jitter 0..1
    pub _pad_wind: [f32; 2],
    // Sky colors from time-of-day (vec4-aligned)
    pub sky_zenith: [f32; 4],
    pub sky_horizon: [f32; 4],
    pub sky_horizon_sun: [f32; 4],
    pub sky_nadir: [f32; 4],
}

pub struct RenderState {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub render_pipeline: wgpu::RenderPipeline,
    pub camera: Camera,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub sun_bind_group: wgpu::BindGroup,
    pub chunk_manager: ChunkManager,
    pub depth_texture: wgpu::Texture,
    // Sky dome
    sky_pipeline: wgpu::RenderPipeline,
    // Sun disc
    pub sun_pixel_pipeline: wgpu::RenderPipeline,
    pub sun_pixel_vertex_buffer: wgpu::Buffer,
    pub sun_pixel_index_buffer: wgpu::Buffer,
    pub sun_pixel_camera_buffer: wgpu::Buffer,
    pub sun_pixel_camera_bind_group: wgpu::BindGroup,
    // Crosshair (fullscreen SDF)
    crosshair_pipeline: wgpu::RenderPipeline,
    crosshair_uniform_buffer: wgpu::Buffer,
    crosshair_bind_group: wgpu::BindGroup,
    // Ocean plane — grid mesh at sea level, follows camera
    ocean_pipeline: wgpu::RenderPipeline,
    ocean_vertex_buffer: wgpu::Buffer,
    ocean_index_buffer: wgpu::Buffer,
    ocean_depth_copy: wgpu::Texture,
    ocean_depth_bind_group: wgpu::BindGroup,
    ocean_depth_bind_group_layout: wgpu::BindGroupLayout,
    // Weather overlay
    weather_pipeline: wgpu::RenderPipeline,
    weather_buffer: wgpu::Buffer,
    weather_bind_group: wgpu::BindGroup,
    weather_clear_color: wgpu::Color,
    sky_zenith_base: [f64; 3],
    weather_time: f32,
    // Shadow mapping
    shadow_texture: wgpu::Texture,
    shadow_pipeline: wgpu::RenderPipeline,
    shadow_camera_buffer: wgpu::Buffer,
    shadow_camera_bind_group: wgpu::BindGroup,
    shadow_bind_group: wgpu::BindGroup,
    shadow_uniform_buffer: wgpu::Buffer,
    sun_direction: Vec3,
    // Volumetric lighting
    volumetric_pipeline: wgpu::RenderPipeline,
    volumetric_bind_group: wgpu::BindGroup,
    volumetric_bind_group_layout: wgpu::BindGroupLayout,
    sun_buffer: wgpu::Buffer,
    // Bloom
    scene_texture: wgpu::Texture,
    scene_view: wgpu::TextureView,
    bloom_textures: [wgpu::Texture; 2], // ping-pong for blur
    bloom_views: [wgpu::TextureView; 2],
    bloom_extract_pipeline: wgpu::RenderPipeline,
    bloom_blur_h_pipeline: wgpu::RenderPipeline,
    bloom_blur_v_pipeline: wgpu::RenderPipeline,
    bloom_composite_pipeline: wgpu::RenderPipeline,
    bloom_extract_bind_group: wgpu::BindGroup,
    bloom_blur_bind_groups: [wgpu::BindGroup; 2], // [read 0 → write 1, read 1 → write 0]
    bloom_composite_bind_group: wgpu::BindGroup,
    bloom_sampler: wgpu::Sampler,
    bloom_bind_group_layout: wgpu::BindGroupLayout,
    bloom_composite_bind_group_layout: wgpu::BindGroupLayout,
    // SSAO
    ssao_pipeline: wgpu::RenderPipeline,
    ssao_bind_group: wgpu::BindGroup,
    ssao_bind_group_layout: wgpu::BindGroupLayout,
    // TAA
    taa_pipeline: wgpu::RenderPipeline,
    taa_bind_group: wgpu::BindGroup,
    taa_bind_group_layout: wgpu::BindGroupLayout,
    taa_uniform_buffer: wgpu::Buffer,
    taa_history_texture: wgpu::Texture,
    taa_history_view: wgpu::TextureView,
    taa_resolve_texture: wgpu::Texture,
    taa_resolve_view: wgpu::TextureView,
    taa_sampler: wgpu::Sampler,
    taa_frame_index: u32,
    prev_view_proj: [[f32; 4]; 4],
    // HUD overlay (vignette, compass, stamina, god mode)
    hud_pipeline: wgpu::RenderPipeline,
    hud_buffer: wgpu::Buffer,
    hud_bind_group: wgpu::BindGroup,
    // Pause menu
    menu_pipeline: wgpu::RenderPipeline,
    // UI state
    pub paused: bool,
    pub god_mode: bool,
    pub command_text: Option<String>,
    pub hud_vertices: Option<Vec<crate::ui::UiVertex>>,
    // Menu page: 0=main, 1=settings
    pub menu_page: u8,
    pub fov_setting: u32,
    pub render_dist_setting: u32,
    pub sensitivity_label: String,
    pub mouse_uv: [f32; 2],
}

impl RenderState {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let required_limits = if cfg!(target_arch = "wasm32") {
            wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits())
        } else {
            wgpu::Limits {
                max_bind_groups: 5, // need group 4 for ocean depth
                ..wgpu::Limits::default()
            }
        };

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("tova_device"),
                required_features: wgpu::Features::empty(),
                required_limits,
                ..Default::default()
            }, None)
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Linear format for intermediate render targets (no sRGB encode/decode per pass)
        let scene_format = linear_format(config.format);

        // Camera
        let camera = Camera::new(size.width as f32 / size.height as f32);
        let camera_uniform = camera.build_view_proj();
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
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

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Sun uniform — dim, cold, filtered through heavy overcast
        let sun_dir = Vec3::new(0.4, 0.25, 0.2).normalize();
        let sun_uniform = SunUniform {
            direction: sun_dir.to_array(),
            _pad: 0.0,
            color: [0.52, 0.50, 0.55],
            ambient: 0.28,
        };
        let sun_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sun_buffer"),
            contents: bytemuck::cast_slice(&[sun_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sun_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("sun_bind_group_layout"),
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

        let sun_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sun_bind_group"),
            layout: &sun_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sun_buffer.as_entire_binding(),
            }],
        });

        // Weather uniform
        let weather_uniform = WeatherUniform {
            weather_type: 0.0,
            intensity: 0.0,
            time: 0.0,
            fog_mult: 1.0,
            sky_darken: 0.0,
            wind_x: 1.0,
            wind_z: 0.3,
            wind_strength: 0.0,
            wind_gust: 0.0,
            wind_turbulence: 0.0,
            _pad_wind: [0.0; 2],
            sky_zenith: [0.28, 0.26, 0.32, 0.0],
            sky_horizon: [0.34, 0.32, 0.38, 0.0],
            sky_horizon_sun: [0.40, 0.38, 0.42, 0.0],
            sky_nadir: [0.22, 0.20, 0.25, 0.0],
        };
        let weather_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("weather_buffer"),
            contents: bytemuck::cast_slice(&[weather_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let weather_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("weather_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let weather_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("weather_bind_group"),
            layout: &weather_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: weather_buffer.as_entire_binding(),
            }],
        });

        // ─── Shadow mapping ─────────────────────────────────
        let shadow_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow_map"),
            size: wgpu::Extent3d {
                width: SHADOW_MAP_SIZE,
                height: SHADOW_MAP_SIZE,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let shadow_texture_view = shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow_sampler"),
            compare: Some(wgpu::CompareFunction::Less),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let light_vp = compute_light_vp(sun_dir, Vec3::ZERO);
        let shadow_uniform = ShadowUniform {
            light_vp: light_vp.to_cols_array_2d(),
        };
        let shadow_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_uniform_buffer"),
            contents: bytemuck::cast_slice(&[shadow_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Shadow camera buffer — light VP stored as CameraUniform for depth pass
        let shadow_cam_uniform = CameraUniform {
            view_proj: light_vp.to_cols_array_2d(),
            inv_view_proj: light_vp.inverse().to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 0.0],
            _pad: 0.0,
        };
        let shadow_camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadow_camera_buffer"),
            contents: bytemuck::cast_slice(&[shadow_cam_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shadow_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shadow_camera_buffer.as_entire_binding(),
            }],
        });

        let shadow_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("shadow_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                ],
            });

        let shadow_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("shadow_bind_group"),
            layout: &shadow_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: shadow_uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
            ],
        });

        // World shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/shader.wgsl").into(),
            ),
        });

        // Main world pipeline (camera + sun + weather bind groups)
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &sun_bind_group_layout,
                &weather_bind_group_layout,
                &shadow_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let depth_stencil = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("world_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: scene_format,
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
            depth_stencil: Some(depth_stencil.clone()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Sun disc camera
        let sun_pixel_camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sun_pixel_camera_buffer"),
            contents: bytemuck::cast_slice(&[camera.build_view_proj()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sun_pixel_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sun_pixel_camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sun_pixel_camera_buffer.as_entire_binding(),
            }],
        });

        // Sky dome pipeline — fullscreen triangle, renders at depth=1.0
        let sky_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sky"),
                buffers: &[], // no vertex buffers — uses vertex_index
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sky"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: scene_format,
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
                depth_compare: wgpu::CompareFunction::LessEqual, // renders at z=1.0
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Sun pixel pipeline
        let sun_pixel_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sun_pixel_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sun"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: scene_format,
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

        // Sun disc geometry
        let sun_pos = sun_dir * SUN_DISTANCE;
        let sun_color = [0.65_f32, 0.62, 0.55];
        let sun_normal = [0.0_f32, 0.0, 0.0];

        let right = sun_dir.cross(Vec3::Y).normalize() * SUN_SIZE;
        let up = sun_dir.cross(right).normalize() * SUN_SIZE;
        let p = sun_pos;

        let sun_verts = vec![
            Vertex { position: (p - right - up).to_array(), color: sun_color, normal: sun_normal },
            Vertex { position: (p + right - up).to_array(), color: sun_color, normal: sun_normal },
            Vertex { position: (p + right + up).to_array(), color: sun_color, normal: sun_normal },
            Vertex { position: (p - right + up).to_array(), color: sun_color, normal: sun_normal },
        ];
        let sun_idx: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        let sun_pixel_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sun_pixel_vertex_buffer"),
            contents: bytemuck::cast_slice(&sun_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let sun_pixel_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sun_pixel_index_buffer"),
            contents: bytemuck::cast_slice(&sun_idx),
            usage: wgpu::BufferUsages::INDEX,
        });

        // ─── Crosshair pipeline ─────────────────────────────
        let crosshair_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("crosshair_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/crosshair.wgsl").into(),
            ),
        });

        // Crosshair uniform: just aspect ratio for SDF fullscreen pass
        let crosshair_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("crosshair_uniform"),
            contents: bytemuck::cast_slice(&[config.width as f32 / config.height as f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let crosshair_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("crosshair_bgl"),
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

        let crosshair_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crosshair_bg"),
            layout: &crosshair_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: crosshair_uniform_buffer.as_entire_binding(),
            }],
        });

        let crosshair_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("crosshair_pipeline_layout"),
                bind_group_layouts: &[&crosshair_bind_group_layout],
                push_constant_ranges: &[],
            });

        let crosshair_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("crosshair_pipeline"),
            layout: Some(&crosshair_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &crosshair_shader,
                entry_point: Some("vs_crosshair"),
                buffers: &[], // fullscreen triangle — no vertex buffers
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &crosshair_shader,
                entry_point: Some("fs_crosshair"),
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

        // ─── Pause menu pipeline ────────────────────────────────
        let menu_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("menu_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/menu.wgsl").into(),
            ),
        });

        let menu_vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<MenuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        };

        let menu_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("menu_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let menu_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("menu_pipeline"),
            layout: Some(&menu_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &menu_shader,
                entry_point: Some("vs_menu"),
                buffers: &[menu_vertex_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &menu_shader,
                entry_point: Some("fs_menu"),
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

        // ─── HUD pipeline (vignette, compass, stamina, god mode) ─
        let hud_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("hud_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/hud.wgsl").into(),
            ),
        });

        let hud_uniform = HudUniform {
            yaw: 0.0,
            stamina: 1.0,
            god_mode: 0.0,
            aspect: config.width as f32 / config.height as f32,
            time: 0.0,
            _pad: [0.0; 3],
        };
        let hud_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("hud_buffer"),
            contents: bytemuck::cast_slice(&[hud_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let hud_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hud_bind_group_layout"),
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

        let hud_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hud_bind_group"),
            layout: &hud_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: hud_buffer.as_entire_binding(),
            }],
        });

        let hud_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hud_pipeline_layout"),
            bind_group_layouts: &[&hud_bind_group_layout],
            push_constant_ranges: &[],
        });

        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud_pipeline"),
            layout: Some(&hud_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &hud_shader,
                entry_point: Some("vs_hud"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &hud_shader,
                entry_point: Some("fs_hud"),
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

        // Weather overlay pipeline — fullscreen triangle, no vertex buffer
        let weather_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("weather_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/weather.wgsl").into(),
            ),
        });

        let weather_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("weather_pipeline_layout"),
                bind_group_layouts: &[&weather_bind_group_layout],
                push_constant_ranges: &[],
            });

        let weather_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("weather_pipeline"),
            layout: Some(&weather_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &weather_shader,
                entry_point: Some("vs_weather"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &weather_shader,
                entry_point: Some("fs_weather"),
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

        // Depth buffer
        let depth_texture = create_depth_texture(&device, &config);

        // Ocean depth copy texture — stores terrain depth for ocean transparency
        let ocean_depth_copy = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ocean_depth_copy"),
            size: wgpu::Extent3d { width: config.width, height: config.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let ocean_depth_copy_view = ocean_depth_copy.create_view(&wgpu::TextureViewDescriptor::default());
        let ocean_depth_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("ocean_depth_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let ocean_depth_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ocean_depth_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
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
            ],
        });
        let ocean_depth_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ocean_depth_bind_group"),
            layout: &ocean_depth_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&ocean_depth_copy_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&ocean_depth_sampler) },
            ],
        });

        let ocean_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ocean_pipeline_layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                &sun_bind_group_layout,
                &weather_bind_group_layout,
                &shadow_bind_group_layout,
                &ocean_depth_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        // Ocean pipeline — vertex displacement, alpha blend, depth test but no depth write
        let ocean_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ocean_pipeline"),
            layout: Some(&ocean_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_ocean"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_ocean"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: scene_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // render from above and below
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false, // don't write depth — terrain already in buffer
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Shadow depth pipeline — vertex only, renders into shadow map
        let shadow_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("shadow_pipeline_layout"),
                bind_group_layouts: &[&camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow_pipeline"),
            layout: Some(&shadow_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: None,
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front), // front-face cull reduces shadow acne
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

        // ─── Volumetric lighting pipeline ──────────────────────
        let volumetric_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("volumetric_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/volumetric.wgsl").into(),
            ),
        });

        // Depth texture view for volumetric sampling
        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let volumetric_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("volumetric_bind_group_layout"),
                entries: &[
                    // camera uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // sun uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // shadow uniform (light VP)
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
                    // shadow map depth texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // shadow comparison sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                        count: None,
                    },
                    // scene depth texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 5,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        let volumetric_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("volumetric_bind_group"),
            layout: &volumetric_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: sun_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: shadow_uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&shadow_texture_view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&shadow_sampler) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&depth_texture_view) },
            ],
        });

        let volumetric_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("volumetric_pipeline_layout"),
                bind_group_layouts: &[&volumetric_bind_group_layout],
                push_constant_ranges: &[],
            });

        let volumetric_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("volumetric_pipeline"),
            layout: Some(&volumetric_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &volumetric_shader,
                entry_point: Some("vs_volumetric"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &volumetric_shader,
                entry_point: Some("fs_volumetric"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: scene_format,
                    // Additive blending — light scattering adds to the scene
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
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

        // ─── Bloom pipeline ──────────────────────────────────
        let bloom_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/bloom.wgsl").into(),
            ),
        });

        let bloom_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Scene render target — world renders here instead of swapchain
        let (scene_texture, scene_view) = create_scene_texture(&device, &config);

        // Quarter-res bloom ping-pong textures (cheaper blur, wider spread)
        let bloom_w = config.width / 4;
        let bloom_h = config.height / 4;
        let bloom_textures = [
            create_bloom_texture(&device, bloom_w, bloom_h, "bloom_0"),
            create_bloom_texture(&device, bloom_w, bloom_h, "bloom_1"),
        ];
        let bloom_views = [
            bloom_textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
            bloom_textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
        ];

        // Bind group layouts
        let bloom_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bloom_bind_group_layout"),
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
                ],
            });

        let bloom_composite_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("bloom_composite_bind_group_layout"),
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
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        // Bind groups
        let bloom_extract_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_extract_bg"),
            layout: &bloom_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&scene_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&bloom_sampler) },
            ],
        });

        let bloom_blur_bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom_blur_bg_0"),
                layout: &bloom_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&bloom_views[0]) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&bloom_sampler) },
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom_blur_bg_1"),
                layout: &bloom_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&bloom_views[1]) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&bloom_sampler) },
                ],
            }),
        ];

        let bloom_composite_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_composite_bg"),
            layout: &bloom_composite_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&scene_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&bloom_sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&bloom_views[0]) },
            ],
        });

        // Pipelines
        let bloom_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("bloom_pipeline_layout"),
                bind_group_layouts: &[&bloom_bind_group_layout],
                push_constant_ranges: &[],
            });

        let bloom_composite_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("bloom_composite_pipeline_layout"),
                bind_group_layouts: &[&bloom_composite_bind_group_layout],
                push_constant_ranges: &[],
            });

        let make_bloom_pipeline = |label: &str, entry: &str, layout: &wgpu::PipelineLayout, format: wgpu::TextureFormat| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &bloom_shader,
                    entry_point: Some("vs_bloom"),
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &bloom_shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
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
            })
        };

        let bloom_format = wgpu::TextureFormat::Rgba16Float;
        let bloom_extract_pipeline = make_bloom_pipeline(
            "bloom_extract", "fs_bloom_extract", &bloom_pipeline_layout, bloom_format,
        );
        let bloom_blur_h_pipeline = make_bloom_pipeline(
            "bloom_blur_h", "fs_bloom_blur", &bloom_pipeline_layout, bloom_format,
        );
        let bloom_blur_v_pipeline = make_bloom_pipeline(
            "bloom_blur_v", "fs_bloom_blur_v", &bloom_pipeline_layout, bloom_format,
        );
        let bloom_composite_pipeline = make_bloom_pipeline(
            "bloom_composite", "fs_bloom_composite", &bloom_composite_pipeline_layout, scene_format,
        );

        // ─── SSAO pipeline ─────────────────────────────────────
        let ssao_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ssao_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/ssao.wgsl").into(),
            ),
        });

        let ssao_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ssao_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                ],
            });

        let ssao_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ssao_bind_group"),
            layout: &ssao_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: camera_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&depth_texture_view) },
            ],
        });

        let ssao_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ssao_pipeline_layout"),
                bind_group_layouts: &[&ssao_bind_group_layout],
                push_constant_ranges: &[],
            });

        let ssao_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ssao_pipeline"),
            layout: Some(&ssao_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ssao_shader,
                entry_point: Some("vs_ssao"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ssao_shader,
                entry_point: Some("fs_ssao"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: scene_format,
                    // Multiply blend — AO darkens the scene
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Dst,
                            dst_factor: wgpu::BlendFactor::Zero,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::Zero,
                            dst_factor: wgpu::BlendFactor::One,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
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

        // ─── TAA pipeline ──────────────────────────────────────
        let taa_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("taa_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/taa.wgsl").into(),
            ),
        });

        let taa_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("taa_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let identity_mat = Mat4::IDENTITY.to_cols_array_2d();
        let taa_uniform = TaaUniform {
            prev_view_proj: identity_mat,
            inv_view_proj: identity_mat,
            jitter: [0.0, 0.0],
            feedback: 0.8,
            _pad: 0.0,
        };
        let taa_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("taa_uniform_buffer"),
            contents: bytemuck::cast_slice(&[taa_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // History + resolve textures (same size as scene)
        let (taa_history_texture, taa_history_view) = create_taa_texture(&device, &config, "taa_history");
        let (taa_resolve_texture, taa_resolve_view) = create_taa_texture(&device, &config, "taa_resolve");

        let taa_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("taa_bind_group_layout"),
                entries: &[
                    // current frame
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
                    // history
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
                    // depth
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Depth,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // sampler
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // taa uniform
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
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

        let taa_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("taa_bind_group"),
            layout: &taa_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&taa_resolve_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&taa_history_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&depth_texture_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&taa_sampler) },
                wgpu::BindGroupEntry { binding: 4, resource: taa_uniform_buffer.as_entire_binding() },
            ],
        });

        let taa_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("taa_pipeline_layout"),
                bind_group_layouts: &[&taa_bind_group_layout],
                push_constant_ranges: &[],
            });

        let taa_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("taa_pipeline"),
            layout: Some(&taa_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &taa_shader,
                entry_point: Some("vs_taa"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &taa_shader,
                entry_point: Some("fs_taa"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
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

        // Ocean mesh — grid at SEA_LEVEL, updated each frame to follow camera
        let (ocean_verts, ocean_idxs) = build_ocean_geometry(0.0, 0.0);
        let ocean_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ocean_vertex_buffer"),
            contents: bytemuck::cast_slice(&ocean_verts),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let ocean_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ocean_index_buffer"),
            contents: bytemuck::cast_slice(&ocean_idxs),
            usage: wgpu::BufferUsages::INDEX,
        });

        // Generate world via ChunkManager
        let mut chunk_manager = ChunkManager::new(RENDER_DISTANCE);
        chunk_manager.generate_initial(&device);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            camera,
            camera_buffer,
            camera_bind_group,
            sun_bind_group,
            chunk_manager,
            depth_texture,
            sky_pipeline,
            sun_pixel_pipeline,
            sun_pixel_vertex_buffer,
            sun_pixel_index_buffer,
            sun_pixel_camera_buffer,
            sun_pixel_camera_bind_group,
            crosshair_pipeline,
            crosshair_uniform_buffer,
            crosshair_bind_group,
            ocean_pipeline,
            ocean_vertex_buffer,
            ocean_index_buffer,
            ocean_depth_copy,
            ocean_depth_bind_group,
            ocean_depth_bind_group_layout,
            weather_pipeline,
            weather_buffer,
            weather_bind_group,
            weather_clear_color: SKY_ZENITH,
            sky_zenith_base: [SKY_ZENITH.r, SKY_ZENITH.g, SKY_ZENITH.b],
            weather_time: 0.0,
            shadow_texture,
            shadow_pipeline,
            shadow_camera_buffer,
            shadow_camera_bind_group,
            shadow_bind_group,
            shadow_uniform_buffer,
            sun_direction: sun_dir,
            volumetric_pipeline,
            volumetric_bind_group,
            volumetric_bind_group_layout,
            sun_buffer,
            scene_texture,
            scene_view,
            bloom_textures,
            bloom_views,
            bloom_extract_pipeline,
            bloom_blur_h_pipeline,
            bloom_blur_v_pipeline,
            bloom_composite_pipeline,
            bloom_extract_bind_group,
            bloom_blur_bind_groups,
            bloom_composite_bind_group,
            bloom_sampler,
            bloom_bind_group_layout,
            bloom_composite_bind_group_layout,
            ssao_pipeline,
            ssao_bind_group,
            ssao_bind_group_layout,
            taa_pipeline,
            taa_bind_group,
            taa_bind_group_layout,
            taa_uniform_buffer,
            taa_history_texture,
            taa_history_view,
            taa_resolve_texture,
            taa_resolve_view,
            taa_sampler,
            taa_frame_index: 0,
            prev_view_proj: identity_mat,
            hud_pipeline,
            hud_buffer,
            hud_bind_group,
            menu_pipeline,
            paused: false,
            god_mode: false,
            command_text: None,
            hud_vertices: None,
            menu_page: 0,
            fov_setting: 1,
            render_dist_setting: 2,
            sensitivity_label: "MEDIUM".into(),
            mouse_uv: [0.5, 0.5],
        }
    }

    /// Update render distance at runtime.
    pub fn set_render_distance(&mut self, dist: i32) {
        self.chunk_manager.set_render_distance(dist);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.camera.aspect = new_size.width as f32 / new_size.height as f32;
            self.depth_texture = create_depth_texture(&self.device, &self.config);

            // Recreate volumetric bind group — it references the depth texture
            let depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let shadow_view = self.shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let shadow_sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("shadow_sampler_resize"),
                compare: Some(wgpu::CompareFunction::Less),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            // Recreate bloom textures and bind groups
            let (scene_tex, scene_v) = create_scene_texture(&self.device, &self.config);
            self.scene_texture = scene_tex;
            self.scene_view = scene_v;

            let bloom_w = self.config.width / 4;
            let bloom_h = self.config.height / 4;
            self.bloom_textures = [
                create_bloom_texture(&self.device, bloom_w, bloom_h, "bloom_0"),
                create_bloom_texture(&self.device, bloom_w, bloom_h, "bloom_1"),
            ];
            self.bloom_views = [
                self.bloom_textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
                self.bloom_textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
            ];

            self.bloom_extract_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom_extract_bg"),
                layout: &self.bloom_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.scene_view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.bloom_sampler) },
                ],
            });
            self.bloom_blur_bind_groups = [
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("bloom_blur_bg_0"),
                    layout: &self.bloom_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.bloom_views[0]) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.bloom_sampler) },
                    ],
                }),
                self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("bloom_blur_bg_1"),
                    layout: &self.bloom_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.bloom_views[1]) },
                        wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.bloom_sampler) },
                    ],
                }),
            ];
            self.bloom_composite_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("bloom_composite_bg"),
                layout: &self.bloom_composite_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.scene_view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.bloom_sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&self.bloom_views[0]) },
                ],
            });

            // Recreate TAA textures and bind groups
            let (taa_hist_tex, taa_hist_view) = create_taa_texture(&self.device, &self.config, "taa_history");
            let (taa_res_tex, taa_res_view) = create_taa_texture(&self.device, &self.config, "taa_resolve");
            self.taa_history_texture = taa_hist_tex;
            self.taa_history_view = taa_hist_view;
            self.taa_resolve_texture = taa_res_tex;
            self.taa_resolve_view = taa_res_view;

            self.taa_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("taa_bind_group"),
                layout: &self.taa_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&self.taa_resolve_view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.taa_history_view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&depth_view) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.taa_sampler) },
                    wgpu::BindGroupEntry { binding: 4, resource: self.taa_uniform_buffer.as_entire_binding() },
                ],
            });

            // Recreate SSAO bind group
            self.ssao_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ssao_bind_group"),
                layout: &self.ssao_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.camera_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&depth_view) },
                ],
            });

            self.volumetric_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("volumetric_bind_group"),
                layout: &self.volumetric_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.camera_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: self.sun_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: self.shadow_uniform_buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&shadow_view) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&shadow_sampler) },
                    wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&depth_view) },
                ],
            });

            // Recreate ocean depth copy texture and bind group
            self.ocean_depth_copy = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("ocean_depth_copy"),
                size: wgpu::Extent3d { width: new_size.width, height: new_size.height, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let ocean_depth_copy_view = self.ocean_depth_copy.create_view(&wgpu::TextureViewDescriptor::default());
            let ocean_depth_sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("ocean_depth_sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            });
            self.ocean_depth_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ocean_depth_bind_group"),
                layout: &self.ocean_depth_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&ocean_depth_copy_view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&ocean_depth_sampler) },
                ],
            });

            // Update crosshair aspect uniform (SDF handles the rest)
            self.queue.write_buffer(
                &self.crosshair_uniform_buffer,
                0,
                bytemuck::cast_slice(&[new_size.width as f32 / new_size.height as f32]),
            );
        }
    }

    pub fn update_camera(&mut self) {
        let unjittered = self.camera.build_view_proj();

        // Apply TAA jitter to the projection
        let jitter_idx = (self.taa_frame_index % 8) as usize;
        let jitter = TAA_JITTER_SEQUENCE[jitter_idx];
        // Convert jitter from [0,1] to [-0.5, 0.5] pixel offset in NDC
        let jitter_ndc = [
            (jitter[0] - 0.5) / self.size.width as f32,
            (jitter[1] - 0.5) / self.size.height as f32,
        ];

        // Apply jitter to view_proj matrix (translate in clip space)
        let mut jittered_vp = unjittered;
        jittered_vp.view_proj[3][0] += jitter_ndc[0] * 2.0;
        jittered_vp.view_proj[3][1] += jitter_ndc[1] * 2.0;

        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[jittered_vp]));
        self.queue.write_buffer(
            &self.sun_pixel_camera_buffer,
            0,
            bytemuck::cast_slice(&[jittered_vp]),
        );

        // Update TAA uniform
        let taa_uniform = TaaUniform {
            prev_view_proj: self.prev_view_proj,
            inv_view_proj: unjittered.inv_view_proj,
            jitter: jitter_ndc,
            feedback: 0.8,
            _pad: 0.0,
        };
        self.queue.write_buffer(
            &self.taa_uniform_buffer,
            0,
            bytemuck::cast_slice(&[taa_uniform]),
        );

        // Store current VP for next frame's reprojection
        self.prev_view_proj = unjittered.view_proj;
        self.taa_frame_index += 1;

        // Update shadow map light VP to follow camera
        let light_vp = compute_light_vp(self.sun_direction, self.camera.position);
        let shadow_uniform = ShadowUniform {
            light_vp: light_vp.to_cols_array_2d(),
        };
        self.queue.write_buffer(
            &self.shadow_uniform_buffer,
            0,
            bytemuck::cast_slice(&[shadow_uniform]),
        );
        let shadow_cam = CameraUniform {
            view_proj: light_vp.to_cols_array_2d(),
            inv_view_proj: light_vp.inverse().to_cols_array_2d(),
            camera_pos: [0.0, 0.0, 0.0],
            _pad: 0.0,
        };
        self.queue.write_buffer(
            &self.shadow_camera_buffer,
            0,
            bytemuck::cast_slice(&[shadow_cam]),
        );
    }


    /// Reposition the ocean plane to follow the camera.
    pub fn update_ocean(&mut self, cam_x: f32, cam_z: f32) {
        let (verts, _) = build_ocean_geometry(cam_x, cam_z);
        self.queue.write_buffer(
            &self.ocean_vertex_buffer,
            0,
            bytemuck::cast_slice(&verts),
        );
    }

    /// Update HUD uniform (called every frame).
    pub fn update_hud(&mut self, yaw: f32, stamina: f32, god_mode: bool, time: f32) {
        let hud = HudUniform {
            yaw,
            stamina,
            god_mode: if god_mode { 1.0 } else { 0.0 },
            aspect: self.camera.aspect,
            time,
            _pad: [0.0; 3],
        };
        self.queue.write_buffer(&self.hud_buffer, 0, bytemuck::cast_slice(&[hud]));
    }

    /// Update weather uniform and clear color.
    /// Update sun direction, color, and ambient from game time.
    pub fn update_sun(&mut self, direction: Vec3, color: [f32; 3], ambient: f32) {
        self.sun_direction = direction;
        let uniform = SunUniform {
            direction: direction.to_array(),
            _pad: 0.0,
            color,
            ambient,
        };
        self.queue.write_buffer(&self.sun_buffer, 0, bytemuck::cast_slice(&[uniform]));

        // Update shadow light VP to follow new sun direction
        let light_vp = compute_light_vp(direction, self.camera.position);
        let shadow_uniform = ShadowUniform {
            light_vp: light_vp.to_cols_array_2d(),
        };
        self.queue.write_buffer(&self.shadow_uniform_buffer, 0, bytemuck::cast_slice(&[shadow_uniform]));
    }

    /// Update sky zenith base color from game time.
    /// Weather darkening is applied on top of this in update_weather.
    pub fn set_sky_zenith(&mut self, rgb: [f32; 3]) {
        self.sky_zenith_base = [rgb[0] as f64, rgb[1] as f64, rgb[2] as f64];
    }

    pub fn update_weather(
        &mut self,
        weather_type: f32,
        intensity: f32,
        time: f32,
        fog_mult: f32,
        sky_darken: f32,
        wind_x: f32,
        wind_z: f32,
        wind_strength: f32,
        wind_gust: f32,
        wind_turbulence: f32,
        sky_zenith: [f32; 3],
        sky_horizon: [f32; 3],
        sky_horizon_sun: [f32; 3],
        sky_nadir: [f32; 3],
    ) {
        let uniform = WeatherUniform {
            weather_type,
            intensity,
            time,
            fog_mult,
            sky_darken,
            wind_x,
            wind_z,
            wind_strength,
            wind_gust,
            wind_turbulence,
            _pad_wind: [0.0; 2],
            sky_zenith: [sky_zenith[0], sky_zenith[1], sky_zenith[2], 0.0],
            sky_horizon: [sky_horizon[0], sky_horizon[1], sky_horizon[2], 0.0],
            sky_horizon_sun: [sky_horizon_sun[0], sky_horizon_sun[1], sky_horizon_sun[2], 0.0],
            sky_nadir: [sky_nadir[0], sky_nadir[1], sky_nadir[2], 0.0],
        };
        self.queue.write_buffer(&self.weather_buffer, 0, bytemuck::cast_slice(&[uniform]));
        self.weather_time = time;

        // Darken clear color to match shader sky changes (applied on top of time-based zenith)
        self.weather_clear_color = wgpu::Color {
            r: sky_zenith[0] as f64 * (1.0 - sky_darken as f64),
            g: sky_zenith[1] as f64 * (1.0 - sky_darken as f64),
            b: sky_zenith[2] as f64 * (1.0 - sky_darken as f64),
            a: 1.0,
        };
    }

    /// Stream chunks around the player's chunk position.
    pub fn update_chunks(&mut self, player_cx: i32, player_cz: i32) {
        self.chunk_manager.update(player_cx, player_cz, &self.device);
    }

    /// Capture the current frame to a PNG file.
    pub fn screenshot(&self, path: &str) {
        eprintln!("Taking screenshot to {}...", path);
        let width = self.config.width;
        let height = self.config.height;

        // Create a texture we can copy the swapchain to and then map
        let bytes_per_row = ((width * 4 + 255) / 256) * 256; // aligned to 256
        let buffer_size = (bytes_per_row * height) as u64;

        let screenshot_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("screenshot_buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // We'll capture from the taa_resolve texture (the final composited frame)
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("screenshot_encoder"),
        });

        eprintln!("  size: {}x{}, format: {:?}", width, height, self.config.format);

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.taa_resolve_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &screenshot_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map the buffer and save
        let buffer_slice = screenshot_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.poll(wgpu::Maintain::Wait);

        match rx.recv() {
            Ok(Ok(())) => {
                let data = buffer_slice.get_mapped_range();
                // Remove row padding, handle BGRA → RGBA swap
                let mut pixels = Vec::with_capacity((width * height * 4) as usize);
                for row in 0..height {
                    let start = (row * bytes_per_row) as usize;
                    let end = start + (width * 4) as usize;
                    for chunk in data[start..end].chunks_exact(4) {
                        // BGRA → RGBA + linear→sRGB gamma (intermediate textures are linear)
                        let r = ((chunk[2] as f32 / 255.0).powf(1.0 / 2.2) * 255.0) as u8;
                        let g = ((chunk[1] as f32 / 255.0).powf(1.0 / 2.2) * 255.0) as u8;
                        let b = ((chunk[0] as f32 / 255.0).powf(1.0 / 2.2) * 255.0) as u8;
                        pixels.extend_from_slice(&[r, g, b, chunk[3]]);
                    }
                }
                drop(data);
                screenshot_buffer.unmap();

                if let Some(img) = image::RgbaImage::from_raw(width, height, pixels) {
                    match img.save(path) {
                        Ok(_) => eprintln!("Screenshot saved to {}", path),
                        Err(e) => eprintln!("Failed to save screenshot: {}", e),
                    }
                } else {
                    eprintln!("Failed to create image from raw pixels");
                }
            }
            Ok(Err(e)) => eprintln!("Buffer map failed: {:?}", e),
            Err(e) => eprintln!("Channel recv failed: {:?}", e),
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Frustum culling — compute once, reuse for all passes
        let frustum = self.camera.frustum_planes();
        let cam_cx = (self.camera.position.x / CHUNK_SIZE as f32).floor() as i32;
        let cam_cz = (self.camera.position.z / CHUNK_SIZE as f32).floor() as i32;

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        // Shadow depth pass — only chunks within shadow cascade distance
        {
            let shadow_view = self.shadow_texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("shadow_pass"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &shadow_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rp.set_pipeline(&self.shadow_pipeline);
            rp.set_bind_group(0, &self.shadow_camera_bind_group, &[]);

            // Only render chunks within shadow cascade range (~8 chunks)
            let shadow_chunks = (SHADOW_CASCADE_SIZE / CHUNK_SIZE as f32).ceil() as i32 + 1;
            for mesh in self.chunk_manager.meshes() {
                if (mesh.cx - cam_cx).abs() <= shadow_chunks
                    && (mesh.cz - cam_cz).abs() <= shadow_chunks
                {
                    rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    rp.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    rp.draw_indexed(0..mesh.num_indices, 0, 0..1);
                }
            }

            for mesh in self.chunk_manager.props.meshes() {
                rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                rp.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
        }

        // World pass — render to scene texture (not swapchain) for bloom
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.weather_clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Draw sky dome (fullscreen tri at depth=1.0, behind everything)
            rp.set_pipeline(&self.sky_pipeline);
            rp.set_bind_group(0, &self.camera_bind_group, &[]);
            rp.set_bind_group(1, &self.sun_bind_group, &[]);
            rp.set_bind_group(2, &self.weather_bind_group, &[]);
            rp.set_bind_group(3, &self.shadow_bind_group, &[]);
            rp.draw(0..3, 0..1);

            // Draw voxel terrain first (writes depth for ocean transparency)
            rp.set_pipeline(&self.render_pipeline);
            rp.set_bind_group(0, &self.camera_bind_group, &[]);
            rp.set_bind_group(1, &self.sun_bind_group, &[]);
            rp.set_bind_group(2, &self.weather_bind_group, &[]);
            rp.set_bind_group(3, &self.shadow_bind_group, &[]);

            for mesh in self.chunk_manager.meshes() {
                if chunk_in_frustum(&frustum, mesh.cx, mesh.cz) {
                    rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    rp.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    rp.draw_indexed(0..mesh.num_indices, 0, 0..1);
                }
            }

            // Draw prop meshes (trees, rocks, etc.)
            for mesh in self.chunk_manager.props.meshes() {
                rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                rp.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }

            // Draw LOD terrain (distant horizons)
            for mesh in self.chunk_manager.lod.meshes() {
                rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                rp.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }

            // Draw sun disc
            rp.set_pipeline(&self.sun_pixel_pipeline);
            rp.set_bind_group(0, &self.sun_pixel_camera_bind_group, &[]);
            rp.set_bind_group(1, &self.sun_bind_group, &[]);
            rp.set_bind_group(2, &self.weather_bind_group, &[]);
            rp.set_bind_group(3, &self.shadow_bind_group, &[]);
            rp.set_vertex_buffer(0, self.sun_pixel_vertex_buffer.slice(..));
            rp.set_index_buffer(self.sun_pixel_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(0..6, 0, 0..1);
        }

        // Copy terrain depth for ocean transparency
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.depth_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.ocean_depth_copy,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
        );

        // Ocean pass — depth-based transparency, foam, shoreline blend
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ocean_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rp.set_pipeline(&self.ocean_pipeline);
            rp.set_bind_group(0, &self.camera_bind_group, &[]);
            rp.set_bind_group(1, &self.sun_bind_group, &[]);
            rp.set_bind_group(2, &self.weather_bind_group, &[]);
            rp.set_bind_group(3, &self.shadow_bind_group, &[]);
            rp.set_bind_group(4, &self.ocean_depth_bind_group, &[]);
            rp.set_vertex_buffer(0, self.ocean_vertex_buffer.slice(..));
            rp.set_index_buffer(self.ocean_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(0..OCEAN_INDICES as u32, 0, 0..1);
        }

        // Volumetric lighting pass — additive god rays (also to scene texture)
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("volumetric_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_view,
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

            rp.set_pipeline(&self.volumetric_pipeline);
            rp.set_bind_group(0, &self.volumetric_bind_group, &[]);
            rp.draw(0..3, 0..1);
        }

        // SSAO pass — multiply-blend occlusion onto scene texture
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ssao_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.scene_view,
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
            rp.set_pipeline(&self.ssao_pipeline);
            rp.set_bind_group(0, &self.ssao_bind_group, &[]);
            rp.draw(0..3, 0..1);
        }

        // Bloom: extract bright pixels from scene → bloom_textures[0]
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_extract"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_views[0],
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
            rp.set_pipeline(&self.bloom_extract_pipeline);
            rp.set_bind_group(0, &self.bloom_extract_bind_group, &[]);
            rp.draw(0..3, 0..1);
        }

        // Bloom: horizontal blur → bloom_textures[1]
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_blur_h"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_views[1],
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
            rp.set_pipeline(&self.bloom_blur_h_pipeline);
            rp.set_bind_group(0, &self.bloom_blur_bind_groups[0], &[]);
            rp.draw(0..3, 0..1);
        }

        // Bloom: vertical blur → bloom_textures[0]
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_blur_v"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_views[0],
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
            rp.set_pipeline(&self.bloom_blur_v_pipeline);
            rp.set_bind_group(0, &self.bloom_blur_bind_groups[1], &[]);
            rp.draw(0..3, 0..1);
        }

        // Bloom: second blur pass (H→V) for wider, smoother spread
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_blur_h2"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_views[1],
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
            rp.set_pipeline(&self.bloom_blur_h_pipeline);
            rp.set_bind_group(0, &self.bloom_blur_bind_groups[0], &[]);
            rp.draw(0..3, 0..1);
        }
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_blur_v2"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.bloom_views[0],
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
            rp.set_pipeline(&self.bloom_blur_v_pipeline);
            rp.set_bind_group(0, &self.bloom_blur_bind_groups[1], &[]);
            rp.draw(0..3, 0..1);
        }

        // Bloom: composite scene + bloom → TAA resolve texture
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bloom_composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.taa_resolve_view,
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
            rp.set_pipeline(&self.bloom_composite_pipeline);
            rp.set_bind_group(0, &self.bloom_composite_bind_group, &[]);
            rp.draw(0..3, 0..1);
        }

        // TAA resolve — blend current + history → swapchain
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("taa_resolve"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            rp.set_pipeline(&self.taa_pipeline);
            rp.set_bind_group(0, &self.taa_bind_group, &[]);
            rp.draw(0..3, 0..1);
        }

        // Copy resolved frame to history for next frame
        let copy_size = wgpu::Extent3d {
            width: self.config.width,
            height: self.config.height,
            depth_or_array_layers: 1,
        };
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.taa_resolve_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.taa_history_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            copy_size,
        );

        // Weather overlay pass (rain/snow particles)
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("weather_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

            rp.set_pipeline(&self.weather_pipeline);
            rp.set_bind_group(0, &self.weather_bind_group, &[]);
            rp.draw(0..3, 0..1);
        }

        // HUD overlay (vignette, compass, stamina, god mode indicator)
        if !self.paused {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hud_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

            rp.set_pipeline(&self.hud_pipeline);
            rp.set_bind_group(0, &self.hud_bind_group, &[]);
            rp.draw(0..3, 0..1);
        }

        // Compass direction labels (vertex text, overlays the HUD shader compass bar)
        if !self.paused {
            let aspect = self.config.width as f32 / self.config.height as f32;
            let compass_verts = build_compass_text_vertices(
                self.camera.yaw, aspect,
            );
            if !compass_verts.is_empty() {
                let compass_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("compass_text_vb"),
                    contents: bytemuck::cast_slice(&compass_verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });
                let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("compass_text_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
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
                rp.set_pipeline(&self.menu_pipeline);
                rp.set_vertex_buffer(0, compass_buf.slice(..));
                rp.draw(0..compass_verts.len() as u32, 0..1);
            }
        }

        // Crosshair overlay pass (no depth) — only when playing
        if !self.paused {
            // Update aspect ratio uniform before the pass
            self.queue.write_buffer(
                &self.crosshair_uniform_buffer,
                0,
                bytemuck::cast_slice(&[self.camera.aspect]),
            );
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("crosshair_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

            rp.set_pipeline(&self.crosshair_pipeline);
            rp.set_bind_group(0, &self.crosshair_bind_group, &[]);
            rp.draw(0..3, 0..1); // fullscreen triangle
        }

        // HUD overlay (FPS, coords, compass, time)
        if !self.paused {
            if let Some(ref hud_verts) = self.hud_vertices {
                if !hud_verts.is_empty() {
                    let hud_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("hud_vb"),
                        contents: bytemuck::cast_slice(hud_verts),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

                    let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("hud_pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
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

                    rp.set_pipeline(&self.menu_pipeline);
                    rp.set_vertex_buffer(0, hud_buffer.slice(..));
                    rp.draw(0..hud_verts.len() as u32, 0..1);
                }
            }
        }

        // Pause menu overlay
        if self.paused {
            let aspect = self.config.width as f32 / self.config.height as f32;
            let menu_verts = if self.menu_page == 0 {
                build_pause_menu_vertices(self.god_mode, aspect, self.mouse_uv)
            } else {
                build_settings_menu_vertices(self.fov_setting, self.render_dist_setting, &self.sensitivity_label, aspect, self.mouse_uv)
            };
            let menu_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("pause_menu_vb"),
                contents: bytemuck::cast_slice(&menu_verts),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pause_menu_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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

            rp.set_pipeline(&self.menu_pipeline);
            rp.set_vertex_buffer(0, menu_buffer.slice(..));
            rp.draw(0..menu_verts.len() as u32, 0..1);
        }

        // Command palette overlay
        if let Some(ref text) = self.command_text {
            let aspect = self.config.width as f32 / self.config.height as f32;
            let cmd_verts = crate::command::build_command_bar(text, self.weather_time, aspect);
            if !cmd_verts.is_empty() {
                let cmd_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("command_bar_vb"),
                    contents: bytemuck::cast_slice(&cmd_verts),
                    usage: wgpu::BufferUsages::VERTEX,
                });

                let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("command_bar_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
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

                rp.set_pipeline(&self.menu_pipeline);
                rp.set_vertex_buffer(0, cmd_buffer.slice(..));
                rp.draw(0..cmd_verts.len() as u32, 0..1);
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

fn create_depth_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth_texture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}

/// Ocean grid resolution and extent.
const OCEAN_HALF: f32 = 220.0;
const OCEAN_CELLS: usize = 55; // 55x55 grid = ~8-block cells
pub const OCEAN_VERTS: usize = (OCEAN_CELLS + 1) * (OCEAN_CELLS + 1);
pub const OCEAN_INDICES: usize = OCEAN_CELLS * OCEAN_CELLS * 6;

/// Simple hash for deterministic per-cell variation.
fn ocean_hash(x: f32, z: f32) -> f32 {
    let ix = (x * 0.7 + 31.7) as i32;
    let iz = (z * 0.7 + 17.3) as i32;
    let h = ((ix.wrapping_mul(374761393) ^ iz.wrapping_mul(668265263)).wrapping_add(1376312589))
        .wrapping_mul(1103515245);
    ((h >> 16) & 0xFFFF) as f32 / 65535.0 // 0..1
}

/// Build an ocean grid centered on (cx, cz) at SEA_LEVEL.
/// Grid has per-vertex color and normal variation for visual depth.
fn build_ocean_geometry(cx: f32, cz: f32) -> (Vec<Vertex>, Vec<u32>) {
    let y_base = SEA_LEVEL as f32 - 0.15;
    let cell = (OCEAN_HALF * 2.0) / OCEAN_CELLS as f32;

    let base_deep = [0.22_f32, 0.26, 0.29];   // deep water — dark, cold
    let base_mid = [0.26_f32, 0.30, 0.33];     // mid water
    let base_shallow = [0.30_f32, 0.34, 0.35];  // shallow — slightly lighter

    let n1 = OCEAN_CELLS + 1;
    let mut verts = Vec::with_capacity(n1 * n1);

    for iz in 0..n1 {
        for ix in 0..n1 {
            let wx = cx - OCEAN_HALF + ix as f32 * cell;
            let wz = cz - OCEAN_HALF + iz as f32 * cell;

            // Distance from camera for depth-based color
            let dx = wx - cx;
            let dz = wz - cz;
            let dist = (dx * dx + dz * dz).sqrt();
            let depth_t = (dist / OCEAN_HALF).min(1.0);

            // Blend shallow → mid → deep based on distance
            let color = if depth_t < 0.3 {
                let t = depth_t / 0.3;
                [
                    base_shallow[0] + (base_mid[0] - base_shallow[0]) * t,
                    base_shallow[1] + (base_mid[1] - base_shallow[1]) * t,
                    base_shallow[2] + (base_mid[2] - base_shallow[2]) * t,
                ]
            } else {
                let t = ((depth_t - 0.3) / 0.7).min(1.0);
                [
                    base_mid[0] + (base_deep[0] - base_mid[0]) * t,
                    base_mid[1] + (base_deep[1] - base_mid[1]) * t,
                    base_mid[2] + (base_deep[2] - base_mid[2]) * t,
                ]
            };

            // Per-cell hash for subtle variation
            let h = ocean_hash(wx, wz);
            let color = [
                (color[0] + (h - 0.5) * 0.03).clamp(0.18, 0.38),
                (color[1] + (h - 0.5) * 0.03).clamp(0.22, 0.38),
                (color[2] + (h - 0.5) * 0.02).clamp(0.24, 0.40),
            ];

            // Subtle height variation — very gentle undulation
            let h2 = ocean_hash(wx + 100.0, wz + 100.0);
            let y = y_base + (h2 - 0.5) * 0.15;

            // Slight normal tilt for light variation across the surface
            let nx = (ocean_hash(wx + 50.0, wz) - 0.5) * 0.15;
            let nz = (ocean_hash(wx, wz + 50.0) - 0.5) * 0.15;
            let ny = (1.0 - nx * nx - nz * nz).max(0.0).sqrt();

            verts.push(Vertex {
                position: [wx, y, wz],
                color,
                normal: [nx, ny, nz],
            });
        }
    }

    let mut idxs = Vec::with_capacity(OCEAN_CELLS * OCEAN_CELLS * 6);
    for iz in 0..OCEAN_CELLS {
        for ix in 0..OCEAN_CELLS {
            let tl = (iz * n1 + ix) as u32;
            let tr = tl + 1;
            let bl = tl + n1 as u32;
            let br = bl + 1;
            idxs.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    (verts, idxs)
}

fn create_scene_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene_texture"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: linear_format(config.format),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn create_bloom_texture(device: &wgpu::Device, width: u32, height: u32, label: &str) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn create_taa_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    label: &str,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: linear_format(config.format),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

/// Compute orthographic light-space VP matrix for shadow mapping.
/// Centers the shadow frustum on the camera, looking along the sun direction.
/// Build pause menu overlay quads: dark background + god mode button + resume button.
/// Shared menu panel: dark overlay + bordered panel + corner accents.
fn menu_panel(v: &mut Vec<MenuVertex>) {
    ui::push_quad(v, -1.0, -1.0, 1.0, 1.0, [0.0, 0.0, 0.0, 0.55]);
    ui::push_quad(v, -0.34, -0.32, 0.34, 0.28, [0.08, 0.07, 0.06, 0.92]);
    let b: [f32; 4] = [0.55, 0.42, 0.22, 0.65];
    let bw = 0.004_f32;
    ui::push_quad(v, -0.34, 0.28 - bw, 0.34, 0.28, b);
    ui::push_quad(v, -0.34, -0.32, 0.34, -0.32 + bw, b);
    ui::push_quad(v, -0.34, -0.32, -0.34 + bw, 0.28, b);
    ui::push_quad(v, 0.34 - bw, -0.32, 0.34, 0.28, b);
    let c: [f32; 4] = [0.65, 0.52, 0.28, 0.8];
    let cs = 0.018_f32;
    ui::push_quad(v, -0.34 - cs, 0.28 - cs, -0.34 + cs, 0.28 + cs, c);
    ui::push_quad(v, 0.34 - cs, 0.28 - cs, 0.34 + cs, 0.28 + cs, c);
    ui::push_quad(v, -0.34 - cs, -0.32 - cs, -0.34 + cs, -0.32 + cs, c);
    ui::push_quad(v, 0.34 - cs, -0.32 - cs, 0.34 + cs, -0.32 + cs, c);
}

/// Check if mouse UV is over a button (NDC y0..y1, x -0.28..0.28).
fn is_hovered(mouse_uv: [f32; 2], y0: f32, y1: f32) -> bool {
    // Convert NDC y to UV: uv_y = (1 - ndc_y) / 2
    let uv_top = (1.0 - y1) / 2.0;
    let uv_bot = (1.0 - y0) / 2.0;
    // Button x range in UV: ndc -0.28..0.28 → uv 0.36..0.64
    let uv_left = (1.0 + (-0.28)) / 2.0;
    let uv_right = (1.0 + 0.28) / 2.0;
    mouse_uv[0] >= uv_left && mouse_uv[0] <= uv_right
        && mouse_uv[1] >= uv_top && mouse_uv[1] <= uv_bot
}

/// Brighten a color for hover effect.
fn hover_brighten(mut c: [f32; 4]) -> [f32; 4] {
    c[0] = (c[0] + 0.06).min(1.0);
    c[1] = (c[1] + 0.05).min(1.0);
    c[2] = (c[2] + 0.04).min(1.0);
    c
}

/// Draw a menu button with text centered inside.
fn menu_button(v: &mut Vec<MenuVertex>, y0: f32, y1: f32, text: &str, scale: f32, ax: f32, bg: [f32; 4], border: [f32; 4], text_color: [f32; 4]) {
    let bw = 0.004_f32;
    ui::push_quad(v, -0.28, y0, 0.28, y1, bg);
    ui::push_quad(v, -0.28, y1 - bw, 0.28, y1, border);
    ui::push_quad(v, -0.28, y0, 0.28, y0 + bw, border);
    ui::push_quad(v, -0.28, y0, -0.28 + bw, y1, border);
    ui::push_quad(v, 0.28 - bw, y0, 0.28, y1, border);
    let tw = ui::text_width(text, scale, ax);
    let tx = -tw / 2.0;
    let th = ui::GLYPH_H as f32 * ui::PIXEL_SIZE * scale;
    let ty = y0 + ((y1 - y0) - th) / 2.0;
    ui::render_text(v, text, tx, ty, scale, ax, text_color);
}

fn build_pause_menu_vertices(god_mode: bool, aspect: f32, mouse_uv: [f32; 2]) -> Vec<MenuVertex> {
    let mut v = Vec::new();
    let ax = 1.0 / aspect;

    menu_panel(&mut v);

    // Title
    let title = "PAUSED";
    let tw = ui::text_width(title, 2.2, ax);
    ui::render_text(&mut v, title, -tw / 2.0, 0.20, 2.2, ax, [0.75, 0.65, 0.42, 0.9]);
    ui::push_quad(&mut v, -0.26, 0.16, 0.26, 0.165, [0.40, 0.33, 0.20, 0.45]);

    // God mode button: y [0.06, 0.14]
    let god_label = if god_mode { "GOD MODE  ON" } else { "GOD MODE  OFF" };
    let god_hovered = is_hovered(mouse_uv, 0.06, 0.14);
    let god_bg = if god_mode { [0.15, 0.24, 0.12, 0.85] } else { [0.14, 0.12, 0.10, 0.85] };
    let god_border = if god_mode { [0.45, 0.65, 0.35, 0.5] } else { [0.40, 0.33, 0.22, 0.35] };
    let god_text = if god_mode { [0.55, 0.85, 0.45, 0.9] } else { [0.65, 0.58, 0.42, 0.75] };
    let god_bg_f = if god_hovered { hover_brighten(god_bg) } else { god_bg };
    menu_button(&mut v, 0.06, 0.14, god_label, 1.5, ax, god_bg_f, god_border, god_text);

    // Settings button: y [-0.04, 0.04]
    let btn_bg = [0.14, 0.12, 0.10, 0.85];
    let btn_border = [0.40, 0.33, 0.22, 0.35];
    let btn_text = [0.65, 0.58, 0.42, 0.75];
    let set_bg = if is_hovered(mouse_uv, -0.04, 0.04) { hover_brighten(btn_bg) } else { btn_bg };
    menu_button(&mut v, -0.04, 0.04, "SETTINGS", 1.5, ax, set_bg, btn_border, btn_text);

    // Resume button: y [-0.14, -0.06]
    let res_bg = if is_hovered(mouse_uv, -0.14, -0.06) { hover_brighten(btn_bg) } else { btn_bg };
    menu_button(&mut v, -0.14, -0.06, "RESUME", 1.5, ax, res_bg, btn_border, btn_text);

    // Hint
    let hint = "ESC TO CLOSE";
    let hw = ui::text_width(hint, 1.0, ax);
    ui::render_text(&mut v, hint, -hw / 2.0, -0.28, 1.0, ax, [0.45, 0.40, 0.32, 0.45]);

    v
}

fn build_settings_menu_vertices(fov: u32, render_dist: u32, sens_label: &str, aspect: f32, mouse_uv: [f32; 2]) -> Vec<MenuVertex> {
    let mut v = Vec::new();
    let ax = 1.0 / aspect;
    let fov_options = [60, 70, 80, 90];
    let dist_options = [8, 10, 14, 18];

    menu_panel(&mut v);

    // Title
    let title = "SETTINGS";
    let tw = ui::text_width(title, 2.2, ax);
    ui::render_text(&mut v, title, -tw / 2.0, 0.20, 2.2, ax, [0.75, 0.65, 0.42, 0.9]);
    ui::push_quad(&mut v, -0.26, 0.16, 0.26, 0.165, [0.40, 0.33, 0.20, 0.45]);

    let row_bg = [0.14, 0.12, 0.10, 0.85];
    let row_border = [0.40, 0.33, 0.22, 0.35];
    let value_color = [0.82, 0.75, 0.52, 0.9];

    // FOV row: y [0.08, 0.14]
    let fov_val = format!("FOV  {}", fov_options[fov as usize]);
    let fov_bg = if is_hovered(mouse_uv, 0.08, 0.14) { hover_brighten(row_bg) } else { row_bg };
    menu_button(&mut v, 0.08, 0.14, &fov_val, 1.4, ax, fov_bg, row_border, value_color);

    // Render distance row: y [0.00, 0.06]
    let dist_val = format!("RENDER DIST  {}", dist_options[render_dist as usize]);
    let dist_bg = if is_hovered(mouse_uv, 0.00, 0.06) { hover_brighten(row_bg) } else { row_bg };
    menu_button(&mut v, 0.00, 0.06, &dist_val, 1.4, ax, dist_bg, row_border, value_color);

    // Sensitivity row: y [-0.08, -0.02]
    let sens_val = format!("SENSITIVITY  {}", sens_label);
    let sens_bg = if is_hovered(mouse_uv, -0.08, -0.02) { hover_brighten(row_bg) } else { row_bg };
    menu_button(&mut v, -0.08, -0.02, &sens_val, 1.4, ax, sens_bg, row_border, value_color);

    // Back button: y [-0.18, -0.10]
    let btn_text = [0.65, 0.58, 0.42, 0.75];
    let back_bg = if is_hovered(mouse_uv, -0.18, -0.10) { hover_brighten(row_bg) } else { row_bg };
    menu_button(&mut v, -0.18, -0.10, "BACK", 1.5, ax, back_bg, row_border, btn_text);

    // Hint
    let hint = "CLICK TO CYCLE";
    let hw = ui::text_width(hint, 1.0, ax);
    ui::render_text(&mut v, hint, -hw / 2.0, -0.28, 1.0, ax, [0.45, 0.40, 0.32, 0.45]);

    v
}

/// Build compass direction labels as vertex quads.
fn build_compass_text_vertices(yaw: f32, aspect: f32) -> Vec<MenuVertex> {
    let mut v = Vec::new();
    let ax = 1.0 / aspect;
    let pi = std::f32::consts::PI;
    let fov_span = pi * 0.667;

    let cardinals: [(f32, &str); 8] = [
        (0.0, "N"), (pi * 0.25, "NE"), (pi * 0.5, "E"), (pi * 0.75, "SE"),
        (pi, "S"), (-pi * 0.75, "SW"), (-pi * 0.5, "W"), (-pi * 0.25, "NW"),
    ];

    let text_y_ndc = 0.92; // just below the compass bar (NDC)
    let cardinal_scale = 1.4;
    let inter_scale = 1.0;

    for (angle, label) in &cardinals {
        let mut diff = *angle - yaw;
        diff = ((diff + pi) % (2.0 * pi)) - pi;
        if diff < -pi { diff += 2.0 * pi; }

        let screen_x = 0.5 + diff / fov_span;
        if screen_x > 0.08 && screen_x < 0.92 {
            let ndc_x = screen_x * 2.0 - 1.0;
            let is_cardinal = label.len() == 1;
            // Edge fade — labels smoothly disappear near screen edges
            let edge_fade = ((screen_x - 0.08) / 0.08).min(1.0).max(0.0)
                          * ((0.92 - screen_x) / 0.08).min(1.0).max(0.0);
            let (mut color, scale) = if is_cardinal {
                ([0.82, 0.72, 0.48, 0.85], cardinal_scale)
            } else {
                ([0.50, 0.45, 0.35, 0.50], inter_scale)
            };
            color[3] *= edge_fade;

            // Center the label on its tick
            let tw = ui::text_width(label, scale, ax);
            let text_x = ndc_x - tw / 2.0;
            let text_h = ui::GLYPH_H as f32 * ui::PIXEL_SIZE * scale;
            ui::render_text(&mut v, label, text_x, text_y_ndc - text_h, scale, ax, color);
        }
    }

    v
}

fn compute_light_vp(sun_dir: Vec3, camera_pos: Vec3) -> Mat4 {
    let half = SHADOW_CASCADE_SIZE;

    // Build light view matrix centered on origin to get stable axes
    let light_view = Mat4::look_at_rh(sun_dir * 400.0, Vec3::ZERO, Vec3::Y);

    // Project camera position into light space
    let cam_light = light_view.transform_point3(camera_pos);

    // Snap to shadow texel grid to prevent shimmer
    let texel_size = (half * 2.0) / SHADOW_MAP_SIZE as f32;
    let snapped_x = (cam_light.x / texel_size).floor() * texel_size;
    let snapped_y = (cam_light.y / texel_size).floor() * texel_size;

    // Rebuild with snapped offset
    let offset = Vec3::new(snapped_x - cam_light.x, snapped_y - cam_light.y, 0.0);
    let light_pos = sun_dir * 400.0 + camera_pos;
    let snap_view = Mat4::look_at_rh(light_pos, camera_pos, Vec3::Y);
    let proj = Mat4::orthographic_rh(-half, half, -half, half, 0.1, 800.0);

    // Apply texel snap as a translation in light clip space
    let snap_mat = Mat4::from_translation(Vec3::new(
        offset.x / half,
        offset.y / half,
        0.0,
    ));
    snap_mat * proj * snap_view
}

use std::collections::BTreeMap;
use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::window::Window;

use crate::camera::{Camera, CameraUniform};
use crate::geometry::Vertex;
use crate::hud::{build_mesh as build_hud_mesh, HudView};
use crate::voxel::{VoxelMesher, VoxelWorld};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

struct DepthTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

pub struct GraphicsState {
    pub size: winit::dpi::PhysicalSize<u32>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    world_pipeline: wgpu::RenderPipeline,
    hud_pipeline: wgpu::RenderPipeline,
    depth_target: DepthTarget,
    meshes: BTreeMap<(i32, i32), GpuMesh>,
}

impl GraphicsState {
    pub async fn new(window: Arc<Window>, world: &VoxelWorld) -> Self {
        let size = window.inner_size();
        #[cfg(target_arch = "wasm32")]
        let primary_backends = wgpu::Backends::BROWSER_WEBGPU;
        #[cfg(target_os = "macos")]
        let primary_backends = wgpu::Backends::METAL;
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "macos")))]
        let primary_backends = wgpu::Backends::VULKAN;

        let (surface, adapter) =
            if let Some(ok) = request_surface_and_adapter(window.clone(), primary_backends).await {
                ok
            } else {
                request_surface_and_adapter(window, wgpu::Backends::all())
                    .await
                    .expect("failed to find GPU adapter")
            };
        let adapter_info = adapter.get_info();
        log::info!(
            "Using {:?} backend on adapter '{}'",
            adapter_info.backend,
            adapter_info.name,
        );

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("tova_rebuilt_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("failed to create logical device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|format| format.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        #[cfg(target_arch = "wasm32")]
        let alpha_mode = surface_caps
            .alpha_modes
            .iter()
            .copied()
            .find(|mode| *mode == wgpu::CompositeAlphaMode::Opaque)
            .unwrap_or(surface_caps.alpha_modes[0]);
        #[cfg(not(target_arch = "wasm32"))]
        let alpha_mode = surface_caps.alpha_modes[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_uniform_buffer"),
            contents: bytemuck::cast_slice(&[CameraUniform::default()]),
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rebuilt_world_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../assets/shaders/rebuilt_world.wgsl").into(),
            ),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("world_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        let world_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rebuilt_world_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let hud_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("hud_pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let hud_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("hud_pipeline"),
            layout: Some(&hud_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_hud"),
                buffers: &[Vertex::layout()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_hud"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let depth_target = create_depth_target(&device, config.width, config.height);
        let meshes = build_world_meshes(&device, world);

        Self {
            size,
            surface,
            device,
            queue,
            config,
            camera_buffer,
            camera_bind_group,
            world_pipeline,
            hud_pipeline,
            depth_target,
            meshes,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            self.size = new_size;
            return;
        }

        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.depth_target = create_depth_target(&self.device, new_size.width, new_size.height);
    }

    pub fn rebuild_world(&mut self, world: &VoxelWorld) {
        self.meshes = build_world_meshes(&self.device, world);
    }

    pub fn rebuild_chunks(&mut self, world: &VoxelWorld, chunk_coords: &[(i32, i32)]) {
        for &coords in chunk_coords {
            if let Some(mesh) = build_chunk_mesh(&self.device, world, coords) {
                self.meshes.insert(coords, mesh);
            } else {
                self.meshes.remove(&coords);
            }
        }
    }

    pub fn render(&mut self, camera: &Camera, hud: HudView<'_>) -> Result<(), wgpu::SurfaceError> {
        if self.config.width == 0 || self.config.height == 0 {
            return Ok(());
        }

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera.uniform()]),
        );

        let output = self.surface.get_current_texture()?;
        let output_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.09,
                            g: 0.10,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_target.view,
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
            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            for mesh in self.meshes.values() {
                pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }

        let (hud_vertices, hud_indices) = build_hud_mesh(hud);
        if !hud_indices.is_empty() {
            let hud_vertex_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("hud_vertex_buffer"),
                        contents: bytemuck::cast_slice(&hud_vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
            let hud_index_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("hud_index_buffer"),
                        contents: bytemuck::cast_slice(&hud_indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });

            let mut hud_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("hud_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &output_view,
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
            hud_pass.set_pipeline(&self.hud_pipeline);
            hud_pass.set_vertex_buffer(0, hud_vertex_buffer.slice(..));
            hud_pass.set_index_buffer(hud_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            hud_pass.draw_indexed(0..hud_indices.len() as u32, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
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
    let surface = instance.create_surface(window).ok()?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await?;
    Some((surface, adapter))
}

fn create_depth_target(device: &wgpu::Device, width: u32, height: u32) -> DepthTarget {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth_texture"),
        size: wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    DepthTarget {
        _texture: texture,
        view,
    }
}

fn build_world_meshes(device: &wgpu::Device, world: &VoxelWorld) -> BTreeMap<(i32, i32), GpuMesh> {
    let mut coords: Vec<_> = world.chunks().keys().copied().collect();
    coords.sort_unstable_by_key(|(cx, cz)| (*cz, *cx));

    let mut meshes = BTreeMap::new();
    for coords in coords {
        if let Some(mesh) = build_chunk_mesh(device, world, coords) {
            meshes.insert(coords, mesh);
        }
    }

    meshes
}

fn build_chunk_mesh(
    device: &wgpu::Device,
    world: &VoxelWorld,
    coords: (i32, i32),
) -> Option<GpuMesh> {
    let chunk = world.chunks().get(&coords)?;
    let (vertices, indices) =
        VoxelMesher::build_with_lookup(chunk, |wx, wy, wz| world.sample_xyz(wx, wy, wz))?;

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("chunk_vertex_buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("chunk_index_buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    Some(GpuMesh {
        vertex_buffer,
        index_buffer,
        index_count: indices.len() as u32,
    })
}

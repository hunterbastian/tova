use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
use winit::window::Window;

use super::camera::Camera;
use super::vertex::{self, Vertex};
use crate::voxel::{Chunk, VoxelMesher};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SunUniform {
    pub direction: [f32; 3],
    pub _pad: f32,
    pub color: [f32; 3],
    pub ambient: f32,
}

pub struct ChunkMesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
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
    pub sun_buffer: wgpu::Buffer,
    pub sun_bind_group: wgpu::BindGroup,
    pub chunk_meshes: Vec<ChunkMesh>,
    // Sword
    pub sword_pipeline: wgpu::RenderPipeline,
    pub sword_vertex_buffer: wgpu::Buffer,
    pub sword_index_buffer: wgpu::Buffer,
    pub sword_num_indices: u32,
    pub sword_camera_buffer: wgpu::Buffer,
    pub sword_camera_bind_group: wgpu::BindGroup,
    // Sun pixel position (screen-space)
    pub sun_pixel_pipeline: wgpu::RenderPipeline,
    pub sun_pixel_vertex_buffer: wgpu::Buffer,
    pub sun_pixel_index_buffer: wgpu::Buffer,
    pub sun_pixel_camera_buffer: wgpu::Buffer,
    pub sun_pixel_camera_bind_group: wgpu::BindGroup,
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

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("tova_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
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
                    visibility: wgpu::ShaderStages::VERTEX,
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

        // Sun uniform
        let sun_uniform = SunUniform {
            direction: [0.5, 0.8, 0.3], // angled sunlight
            _pad: 0.0,
            color: [1.0, 0.95, 0.85],   // warm white
            ambient: 0.25,
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

        // Shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/shader.wgsl").into(),
            ),
        });

        // Main world pipeline (vs_main + fs_main)
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &sun_bind_group_layout],
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
                    format: config.format,
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

        // Sword pipeline (vs_sword + fs_sword) — separate camera bind group, clears depth
        let sword_camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sword_camera_buffer"),
            contents: bytemuck::cast_slice(&[camera.build_view_proj()]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sword_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sword_camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: sword_camera_buffer.as_entire_binding(),
            }],
        });

        let sword_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sword_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &sun_bind_group_layout],
            push_constant_ranges: &[],
        });

        let sword_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sword_pipeline"),
            layout: Some(&sword_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sword"),
                buffers: &[Vertex::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sword"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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

        // Sword geometry
        let (sword_verts, sword_idx) = vertex::create_sword();
        let sword_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sword_vertex_buffer"),
            contents: bytemuck::cast_slice(&sword_verts),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let sword_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("sword_index_buffer"),
            contents: bytemuck::cast_slice(&sword_idx),
            usage: wgpu::BufferUsages::INDEX,
        });
        let sword_num_indices = sword_idx.len() as u32;

        // Sun pixel — a small yellow quad rendered in screen space
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

        // Sun pixel pipeline — no depth test, rendered on top
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
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
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

        // Sun pixel geometry — a small quad far away in the sun direction
        let sun_dir = Vec3::new(0.5, 0.8, 0.3).normalize();
        let sun_pos = sun_dir * 500.0; // far away
        let sun_size = 2.0_f32;
        let sun_color = [1.0_f32, 0.95, 0.7];
        let sun_normal = [0.0_f32, 0.0, 0.0]; // unlit — emissive

        let right = sun_dir.cross(Vec3::Y).normalize() * sun_size;
        let up = sun_dir.cross(right).normalize() * sun_size;
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

        // Generate voxel chunks
        let chunk_meshes = generate_world_chunks(&device);

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
            sun_buffer,
            sun_bind_group,
            chunk_meshes,
            sword_pipeline,
            sword_vertex_buffer,
            sword_index_buffer,
            sword_num_indices,
            sword_camera_buffer,
            sword_camera_bind_group,
            sun_pixel_pipeline,
            sun_pixel_vertex_buffer,
            sun_pixel_index_buffer,
            sun_pixel_camera_buffer,
            sun_pixel_camera_bind_group,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.camera.aspect = new_size.width as f32 / new_size.height as f32;
        }
    }

    pub fn update_camera(&mut self) {
        let uniform = self.camera.build_view_proj();
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));

        // Sword camera: fixed position in lower-right, looking forward
        let sword_view = Mat4::look_at_rh(
            Vec3::new(0.25, -0.2, 0.0),  // offset to lower-right
            Vec3::new(0.25, -0.2, -1.0),  // looking forward
            Vec3::Y,
        );
        let sword_proj = Mat4::perspective_rh(45.0_f32.to_radians(), self.camera.aspect, 0.01, 10.0);
        let sword_vp = sword_proj * sword_view;
        let sword_uniform = super::camera::CameraUniform {
            view_proj: sword_vp.to_cols_array_2d(),
        };
        self.queue.write_buffer(
            &self.sword_camera_buffer,
            0,
            bytemuck::cast_slice(&[sword_uniform]),
        );

        // Sun pixel uses the world camera
        self.queue.write_buffer(
            &self.sun_pixel_camera_buffer,
            0,
            bytemuck::cast_slice(&[uniform]),
        );
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_texture = create_depth_texture(&self.device, &self.config);
        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        // World pass — voxel chunks + sun pixel
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.53,
                            g: 0.72,
                            b: 0.90,
                            a: 1.0,
                        }),
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

            // Draw voxel chunks
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.sun_bind_group, &[]);

            for chunk_mesh in &self.chunk_meshes {
                render_pass.set_vertex_buffer(0, chunk_mesh.vertex_buffer.slice(..));
                render_pass.set_index_buffer(chunk_mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.draw_indexed(0..chunk_mesh.num_indices, 0, 0..1);
            }

            // Draw sun pixel
            render_pass.set_pipeline(&self.sun_pixel_pipeline);
            render_pass.set_bind_group(0, &self.sun_pixel_camera_bind_group, &[]);
            render_pass.set_bind_group(1, &self.sun_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.sun_pixel_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.sun_pixel_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..6, 0, 0..1);
        }

        // Sword pass — clears depth so sword is always on top
        {
            let mut sword_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sword_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load, // keep world render
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0), // clear depth for sword
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            sword_pass.set_pipeline(&self.sword_pipeline);
            sword_pass.set_bind_group(0, &self.sword_camera_bind_group, &[]);
            sword_pass.set_bind_group(1, &self.sun_bind_group, &[]);
            sword_pass.set_vertex_buffer(0, self.sword_vertex_buffer.slice(..));
            sword_pass.set_index_buffer(self.sword_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            sword_pass.draw_indexed(0..self.sword_num_indices, 0, 0..1);
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

/// Generate a grid of voxel chunks and mesh them.
fn generate_world_chunks(device: &wgpu::Device) -> Vec<ChunkMesh> {
    let mut meshes = Vec::new();
    let radius = 4_i32;

    for cz in -radius..radius {
        for cx in -radius..radius {
            let mut chunk = Chunk::new(cx, cz);
            chunk.generate_test();

            if let Some((vertices, indices)) = VoxelMesher::build(&chunk) {
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

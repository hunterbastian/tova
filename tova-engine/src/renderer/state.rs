use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use wgpu::util::DeviceExt;
use winit::window::Window;

use super::camera::Camera;
use super::vertex::Vertex;
use crate::world::ChunkManager;

// Shared constants — shader.wgsl duplicates SKY_ZENITH and SUN_DISTANCE/SUN_SIZE
const SKY_ZENITH: wgpu::Color = wgpu::Color { r: 0.52, g: 0.50, b: 0.47, a: 1.0 };
const SUN_DISTANCE: f32 = 800.0;
const SUN_SIZE: f32 = 22.0;
const RENDER_DISTANCE: i32 = 8;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct SunUniform {
    pub direction: [f32; 3],
    pub _pad: f32,
    pub color: [f32; 3],
    pub ambient: f32,
}

/// Simple 2D vertex for the crosshair overlay.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CrosshairVertex {
    position: [f32; 2],
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
    // Sun disc
    pub sun_pixel_pipeline: wgpu::RenderPipeline,
    pub sun_pixel_vertex_buffer: wgpu::Buffer,
    pub sun_pixel_index_buffer: wgpu::Buffer,
    pub sun_pixel_camera_buffer: wgpu::Buffer,
    pub sun_pixel_camera_bind_group: wgpu::BindGroup,
    // Crosshair
    crosshair_pipeline: wgpu::RenderPipeline,
    crosshair_vertex_buffer: wgpu::Buffer,
    crosshair_num_vertices: u32,
    // First-person feet
    feet_vertex_buffer: wgpu::Buffer,
    feet_index_buffer: wgpu::Buffer,
    feet_num_indices: u32,
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
            wgpu::Limits::default()
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

        // Sun uniform — low, hazy, diffused through overcast
        let sun_dir = Vec3::new(0.3, 0.5, 0.15).normalize();
        let sun_uniform = SunUniform {
            direction: sun_dir.to_array(),
            _pad: 0.0,
            color: [0.70, 0.65, 0.55],
            ambient: 0.40,
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

        // World shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../assets/shaders/shader.wgsl").into(),
            ),
        });

        // Main world pipeline
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

        let crosshair_vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<CrosshairVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        };

        let crosshair_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("crosshair_pipeline_layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let crosshair_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("crosshair_pipeline"),
            layout: Some(&crosshair_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &crosshair_shader,
                entry_point: Some("vs_crosshair"),
                buffers: &[crosshair_vertex_layout],
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

        let crosshair_vertices = build_crosshair_vertices();
        let crosshair_num_vertices = crosshair_vertices.len() as u32;
        let crosshair_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("crosshair_vertex_buffer"),
            contents: bytemuck::cast_slice(&crosshair_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Depth buffer
        let depth_texture = create_depth_texture(&device, &config);

        // First-person feet — pre-allocate buffers (updated each frame)
        let (feet_verts, feet_idxs) = build_feet_geometry(Vec3::ZERO, 0.0, 0.0);
        let feet_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("feet_vertex_buffer"),
            contents: bytemuck::cast_slice(&feet_verts),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        let feet_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("feet_index_buffer"),
            contents: bytemuck::cast_slice(&feet_idxs),
            usage: wgpu::BufferUsages::INDEX,
        });
        let feet_num_indices = feet_idxs.len() as u32;

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
            sun_pixel_pipeline,
            sun_pixel_vertex_buffer,
            sun_pixel_index_buffer,
            sun_pixel_camera_buffer,
            sun_pixel_camera_bind_group,
            crosshair_pipeline,
            crosshair_vertex_buffer,
            crosshair_num_vertices,
            feet_vertex_buffer,
            feet_index_buffer,
            feet_num_indices,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.camera.aspect = new_size.width as f32 / new_size.height as f32;
            self.depth_texture = create_depth_texture(&self.device, &self.config);
        }
    }

    pub fn update_camera(&mut self) {
        let uniform = self.camera.build_view_proj();
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[uniform]));
        self.queue.write_buffer(
            &self.sun_pixel_camera_buffer,
            0,
            bytemuck::cast_slice(&[uniform]),
        );
    }

    /// Rebuild the feet vertex buffer to match the player's current pose.
    pub fn update_feet(&mut self, pos: Vec3, yaw: f32, walk_cycle: f32) {
        let (verts, _) = build_feet_geometry(pos, yaw, walk_cycle);
        self.queue.write_buffer(
            &self.feet_vertex_buffer,
            0,
            bytemuck::cast_slice(&verts),
        );
    }

    /// Stream chunks around the player's chunk position.
    pub fn update_chunks(&mut self, player_cx: i32, player_cz: i32) {
        self.chunk_manager.update(player_cx, player_cz, &self.device);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_view = self.depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render_encoder"),
            });

        // World pass
        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(SKY_ZENITH),
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
            rp.set_pipeline(&self.render_pipeline);
            rp.set_bind_group(0, &self.camera_bind_group, &[]);
            rp.set_bind_group(1, &self.sun_bind_group, &[]);

            for mesh in self.chunk_manager.meshes() {
                rp.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                rp.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                rp.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }

            // Draw first-person feet
            rp.set_vertex_buffer(0, self.feet_vertex_buffer.slice(..));
            rp.set_index_buffer(self.feet_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(0..self.feet_num_indices, 0, 0..1);

            // Draw sun disc
            rp.set_pipeline(&self.sun_pixel_pipeline);
            rp.set_bind_group(0, &self.sun_pixel_camera_bind_group, &[]);
            rp.set_bind_group(1, &self.sun_bind_group, &[]);
            rp.set_vertex_buffer(0, self.sun_pixel_vertex_buffer.slice(..));
            rp.set_index_buffer(self.sun_pixel_index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(0..6, 0, 0..1);
        }

        // Crosshair overlay pass (no depth)
        {
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
            rp.set_vertex_buffer(0, self.crosshair_vertex_buffer.slice(..));
            rp.draw(0..self.crosshair_num_vertices, 0..1);
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

/// Build two boot-shaped boxes at the player's feet, oriented by yaw.
/// Walk cycle (0..1) animates the feet forward/back in alternation.
fn build_feet_geometry(pos: Vec3, yaw: f32, walk_cycle: f32) -> (Vec<Vertex>, Vec<u32>) {
    let boot_color = [0.28, 0.22, 0.16]; // dark Morrowind leather

    let fwd = Vec3::new(yaw.sin(), 0.0, -yaw.cos());
    let right = Vec3::new(yaw.cos(), 0.0, yaw.sin());

    // Walk bob — alternate feet forward/back with sine wave
    let bob = (walk_cycle * std::f32::consts::TAU).sin() * 0.08;

    // Boot positions relative to feet
    let left_center = pos + Vec3::Y * 0.06 - right * 0.11 + fwd * (0.15 - bob);
    let right_center = pos + Vec3::Y * 0.06 + right * 0.11 + fwd * (0.15 + bob);

    let mut verts = Vec::with_capacity(48);
    let mut idxs = Vec::with_capacity(72);

    for center in [left_center, right_center] {
        push_boot_box(&mut verts, &mut idxs, center, fwd, right, boot_color);
    }

    (verts, idxs)
}

/// Push a single boot box (6 faces, 24 vertices, 36 indices).
fn push_boot_box(
    verts: &mut Vec<Vertex>,
    idxs: &mut Vec<u32>,
    center: Vec3,
    fwd: Vec3,
    right: Vec3,
    color: [f32; 3],
) {
    let hw = 0.07;  // half-width
    let hh = 0.055; // half-height
    let hl = 0.13;  // half-length (along forward)
    let up = Vec3::Y;

    // 8 corners of the oriented box
    let c = [
        center - right * hw - up * hh - fwd * hl, // 0: left-bottom-back
        center + right * hw - up * hh - fwd * hl, // 1: right-bottom-back
        center + right * hw - up * hh + fwd * hl, // 2: right-bottom-front
        center - right * hw - up * hh + fwd * hl, // 3: left-bottom-front
        center - right * hw + up * hh - fwd * hl, // 4: left-top-back
        center + right * hw + up * hh - fwd * hl, // 5: right-top-back
        center + right * hw + up * hh + fwd * hl, // 6: right-top-front
        center - right * hw + up * hh + fwd * hl, // 7: left-top-front
    ];

    // Each face: 4 corner indices (CCW from outside), normal direction
    let faces: &[([usize; 4], Vec3)] = &[
        ([2, 3, 7, 6], fwd),         // front
        ([0, 1, 5, 4], -fwd),        // back
        ([1, 2, 6, 5], right),       // right side
        ([3, 0, 4, 7], -right),      // left side
        ([7, 4, 5, 6], up),          // top
        ([0, 3, 2, 1], -up),         // bottom
    ];

    for (corners, normal) in faces {
        let base = verts.len() as u32;
        let n = normal.to_array();
        for &ci in corners {
            verts.push(Vertex {
                position: c[ci].to_array(),
                color,
                normal: n,
            });
        }
        idxs.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}

/// Build crosshair vertices — small + shape in NDC.
fn build_crosshair_vertices() -> Vec<CrosshairVertex> {
    let t = 0.0015; // half-thickness
    let l = 0.012;  // half-length

    // Horizontal bar
    let mut verts = vec![
        CrosshairVertex { position: [-l, -t] },
        CrosshairVertex { position: [ l, -t] },
        CrosshairVertex { position: [ l,  t] },
        CrosshairVertex { position: [-l, -t] },
        CrosshairVertex { position: [ l,  t] },
        CrosshairVertex { position: [-l,  t] },
    ];

    // Vertical bar
    verts.extend_from_slice(&[
        CrosshairVertex { position: [-t, -l] },
        CrosshairVertex { position: [ t, -l] },
        CrosshairVertex { position: [ t,  l] },
        CrosshairVertex { position: [-t, -l] },
        CrosshairVertex { position: [ t,  l] },
        CrosshairVertex { position: [-t,  l] },
    ]);

    verts
}

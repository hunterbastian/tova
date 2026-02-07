use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// A ground grid for spatial reference — flat colored quads.
pub fn create_ground_grid(size: i32, spacing: f32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let color_a = [0.18, 0.22, 0.15]; // dark green
    let color_b = [0.22, 0.27, 0.18]; // slightly lighter green

    for x in -size..size {
        for z in -size..size {
            let base = vertices.len() as u32;
            let color = if (x + z).rem_euclid(2) == 0 { color_a } else { color_b };
            let fx = x as f32 * spacing;
            let fz = z as f32 * spacing;

            vertices.push(Vertex { position: [fx, 0.0, fz], color });
            vertices.push(Vertex { position: [fx + spacing, 0.0, fz], color });
            vertices.push(Vertex { position: [fx + spacing, 0.0, fz + spacing], color });
            vertices.push(Vertex { position: [fx, 0.0, fz + spacing], color });

            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    (vertices, indices)
}

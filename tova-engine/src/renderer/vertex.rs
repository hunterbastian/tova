use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub normal: [f32; 3],
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
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// Create a small sword mesh for first-person view.
/// The sword points forward and is positioned in the lower-right of the screen.
pub fn create_sword() -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let silver = [0.75, 0.75, 0.78];  // blade
    let dark_silver = [0.55, 0.55, 0.58]; // blade edge shade
    let brown = [0.35, 0.22, 0.10];   // grip
    let gold = [0.85, 0.70, 0.20];    // crossguard

    // Blade — thin rectangular prism pointing forward (-Z)
    // Blade dimensions: 0.02 wide, 0.5 long, 0.01 thick
    let bw = 0.015_f32; // half width
    let bl = 0.5_f32;   // length
    let bt = 0.005_f32; // half thickness

    // Blade faces (6 faces of a box)
    let blade_verts: [(f32, f32, f32); 8] = [
        (-bw, -bt, 0.0),   // 0: back-left-bottom
        ( bw, -bt, 0.0),   // 1: back-right-bottom
        ( bw,  bt, 0.0),   // 2: back-right-top
        (-bw,  bt, 0.0),   // 3: back-left-top
        (-bw, -bt, -bl),   // 4: front-left-bottom (tip end)
        ( bw, -bt, -bl),   // 5: front-right-bottom
        ( bw,  bt, -bl),   // 6: front-right-top
        (-bw,  bt, -bl),   // 7: front-left-top
    ];

    // Top face (+Y)
    add_quad(&mut vertices, &mut indices, silver, [0.0, 1.0, 0.0],
        blade_verts[3], blade_verts[2], blade_verts[6], blade_verts[7]);
    // Bottom face (-Y)
    add_quad(&mut vertices, &mut indices, dark_silver, [0.0, -1.0, 0.0],
        blade_verts[4], blade_verts[5], blade_verts[1], blade_verts[0]);
    // Right face (+X)
    add_quad(&mut vertices, &mut indices, dark_silver, [1.0, 0.0, 0.0],
        blade_verts[1], blade_verts[5], blade_verts[6], blade_verts[2]);
    // Left face (-X)
    add_quad(&mut vertices, &mut indices, dark_silver, [-1.0, 0.0, 0.0],
        blade_verts[4], blade_verts[0], blade_verts[3], blade_verts[7]);
    // Back face (+Z) — pommel end
    add_quad(&mut vertices, &mut indices, silver, [0.0, 0.0, 1.0],
        blade_verts[0], blade_verts[1], blade_verts[2], blade_verts[3]);
    // Front face (-Z) — tip
    add_quad(&mut vertices, &mut indices, silver, [0.0, 0.0, -1.0],
        blade_verts[7], blade_verts[6], blade_verts[5], blade_verts[4]);

    // Crossguard — wider, short box
    let cw = 0.05_f32;  // half width
    let cl = 0.02_f32;  // half length (along Z)
    let ct = 0.008_f32; // half thickness

    let cross_verts: [(f32, f32, f32); 8] = [
        (-cw, -ct, -cl),
        ( cw, -ct, -cl),
        ( cw,  ct, -cl),
        (-cw,  ct, -cl),
        (-cw, -ct,  cl),
        ( cw, -ct,  cl),
        ( cw,  ct,  cl),
        (-cw,  ct,  cl),
    ];

    // All 6 faces of crossguard
    add_quad(&mut vertices, &mut indices, gold, [0.0, 1.0, 0.0],
        cross_verts[3], cross_verts[2], cross_verts[6], cross_verts[7]);
    add_quad(&mut vertices, &mut indices, gold, [0.0, -1.0, 0.0],
        cross_verts[4], cross_verts[5], cross_verts[1], cross_verts[0]);
    add_quad(&mut vertices, &mut indices, gold, [1.0, 0.0, 0.0],
        cross_verts[1], cross_verts[5], cross_verts[6], cross_verts[2]);
    add_quad(&mut vertices, &mut indices, gold, [-1.0, 0.0, 0.0],
        cross_verts[4], cross_verts[0], cross_verts[3], cross_verts[7]);
    add_quad(&mut vertices, &mut indices, gold, [0.0, 0.0, -1.0],
        cross_verts[3], cross_verts[2], cross_verts[1], cross_verts[0]);
    add_quad(&mut vertices, &mut indices, gold, [0.0, 0.0, 1.0],
        cross_verts[4], cross_verts[5], cross_verts[6], cross_verts[7]);

    // Grip — small cylinder approximated as a box
    let gw = 0.01_f32;
    let gl = 0.12_f32;  // grip length
    let gt = 0.01_f32;

    let grip_verts: [(f32, f32, f32); 8] = [
        (-gw, -gt, 0.0),
        ( gw, -gt, 0.0),
        ( gw,  gt, 0.0),
        (-gw,  gt, 0.0),
        (-gw, -gt, gl),
        ( gw, -gt, gl),
        ( gw,  gt, gl),
        (-gw,  gt, gl),
    ];

    add_quad(&mut vertices, &mut indices, brown, [0.0, 1.0, 0.0],
        grip_verts[3], grip_verts[2], grip_verts[6], grip_verts[7]);
    add_quad(&mut vertices, &mut indices, brown, [0.0, -1.0, 0.0],
        grip_verts[4], grip_verts[5], grip_verts[1], grip_verts[0]);
    add_quad(&mut vertices, &mut indices, brown, [1.0, 0.0, 0.0],
        grip_verts[1], grip_verts[5], grip_verts[6], grip_verts[2]);
    add_quad(&mut vertices, &mut indices, brown, [-1.0, 0.0, 0.0],
        grip_verts[4], grip_verts[0], grip_verts[3], grip_verts[7]);

    (vertices, indices)
}

fn add_quad(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    color: [f32; 3],
    normal: [f32; 3],
    a: (f32, f32, f32),
    b: (f32, f32, f32),
    c: (f32, f32, f32),
    d: (f32, f32, f32),
) {
    let base = vertices.len() as u32;
    for p in [a, b, c, d] {
        vertices.push(Vertex {
            position: [p.0, p.1, p.2],
            color,
            normal,
        });
    }
    indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
}

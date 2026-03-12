use glam::Vec3;
use crate::renderer::Vertex;

/// Wildflower palettes — muted, natural colors
const FLOWER_COLORS: [[f32; 3]; 5] = [
    [0.55, 0.45, 0.55],  // Thistle purple
    [0.60, 0.55, 0.35],  // Goldenrod
    [0.50, 0.50, 0.58],  // Lavender
    [0.55, 0.40, 0.35],  // Rusty red
    [0.58, 0.55, 0.45],  // Pale yellow
];

/// Generate a small wildflower — a stem with a tiny bloom on top.
pub fn generate_flower(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    let height = 0.18 + (seed % 5) as f32 * 0.04;
    let flower_color = FLOWER_COLORS[(seed % 5) as usize];

    // Stem — thin green line (two crossed quads like grass but smaller)
    let stem_color = [0.28, 0.34, 0.22];
    let stem_width = 0.02;

    // Slight random lean
    let lean = Vec3::new(
        ((seed % 7) as f32 - 3.0) * 0.015,
        0.0,
        ((seed % 11) as f32 - 5.0) * 0.012,
    );
    let top = base + Vec3::Y * height + lean;

    // One stem quad
    let angle = (seed % 100) as f32 * 0.063;
    let dir = Vec3::new(angle.cos(), 0.0, angle.sin());
    let n = Vec3::new(-angle.sin(), 0.3, angle.cos()).normalize();

    let vi = verts.len() as u32;
    verts.push(Vertex { position: (base - dir * stem_width).to_array(), color: stem_color, normal: n.to_array() });
    verts.push(Vertex { position: (base + dir * stem_width).to_array(), color: stem_color, normal: n.to_array() });
    verts.push(Vertex { position: (top + dir * stem_width * 0.5).to_array(), color: stem_color, normal: n.to_array() });
    verts.push(Vertex { position: (top - dir * stem_width * 0.5).to_array(), color: stem_color, normal: n.to_array() });
    idxs.extend_from_slice(&[vi, vi + 1, vi + 2, vi, vi + 2, vi + 3]);
    // Back face
    let vi2 = verts.len() as u32;
    let nb = (-n).to_array();
    verts.push(Vertex { position: (base - dir * stem_width).to_array(), color: stem_color, normal: nb });
    verts.push(Vertex { position: (base + dir * stem_width).to_array(), color: stem_color, normal: nb });
    verts.push(Vertex { position: (top + dir * stem_width * 0.5).to_array(), color: stem_color, normal: nb });
    verts.push(Vertex { position: (top - dir * stem_width * 0.5).to_array(), color: stem_color, normal: nb });
    idxs.extend_from_slice(&[vi2, vi2 + 2, vi2 + 1, vi2, vi2 + 3, vi2 + 2]);

    // Bloom — 3-4 tiny petals (small triangles radiating from top)
    let num_petals = 3 + (seed % 2) as usize;
    let petal_size = 0.06 + (seed % 3) as f32 * 0.015;

    for p in 0..num_petals {
        let petal_angle = (p as f32 / num_petals as f32) * std::f32::consts::TAU
            + (seed % 50) as f32 * 0.1;
        let petal_dir = Vec3::new(petal_angle.cos(), 0.0, petal_angle.sin());
        let petal_tip = top + petal_dir * petal_size + Vec3::Y * 0.01;
        let petal_left = top + Vec3::new(
            (petal_angle + 0.5).cos() * petal_size * 0.3,
            0.02,
            (petal_angle + 0.5).sin() * petal_size * 0.3,
        );

        let pn = Vec3::Y.to_array();
        let pvi = verts.len() as u32;
        verts.push(Vertex { position: top.to_array(), color: flower_color, normal: pn });
        verts.push(Vertex { position: petal_tip.to_array(), color: flower_color, normal: pn });
        verts.push(Vertex { position: petal_left.to_array(), color: flower_color, normal: pn });
        idxs.extend_from_slice(&[pvi, pvi + 1, pvi + 2]);

        // Back face
        let pvi2 = verts.len() as u32;
        let pnb = [0.0, -1.0, 0.0];
        verts.push(Vertex { position: top.to_array(), color: flower_color, normal: pnb });
        verts.push(Vertex { position: petal_tip.to_array(), color: flower_color, normal: pnb });
        verts.push(Vertex { position: petal_left.to_array(), color: flower_color, normal: pnb });
        idxs.extend_from_slice(&[pvi2, pvi2 + 2, pvi2 + 1]);
    }

    (verts, idxs)
}

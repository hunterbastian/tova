use glam::Vec3;
use crate::renderer::Vertex;

/// Deterministic hash for variation.
fn ruin_hash(seed: u32) -> u32 {
    seed.wrapping_mul(2654435761)
}

/// Dark stone color palette for ruins.
fn ruin_color(seed: u32) -> [f32; 3] {
    let tint = seed % 4;
    match tint {
        0 => [0.16 + (seed % 3) as f32 * 0.01, 0.15 + (seed % 4) as f32 * 0.008, 0.18],
        1 => [0.20, 0.18 + (seed % 3) as f32 * 0.006, 0.22],
        2 => [0.22, 0.20, 0.24 + (seed % 3) as f32 * 0.006],
        _ => [0.18, 0.17, 0.20 + (seed % 4) as f32 * 0.008],
    }
}

/// Helper: build a deformed box and return (verts, idxs).
/// `corners` are the 4 XZ corners, extruded up to `height` with `steps` subdivisions.
/// `top_offsets` allows each corner's top to be at a different height for crumbling edges.
fn build_box(
    base: Vec3,
    corners: &[Vec3; 4],
    top_heights: &[f32; 4],
    steps: usize,
    color: [f32; 3],
    seed: u32,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    let base_vi = 0u32;

    // For each step, emit 4 corner vertices
    for step in 0..=steps {
        let t = step as f32 / steps as f32;

        for (ci, corner) in corners.iter().enumerate() {
            let top_h = top_heights[ci];
            let y = base.y + t * top_h;

            // Weathering deformation
            let ds = seed.wrapping_add(step as u32 * 13 + ci as u32 * 37);
            let deform_x = ((ruin_hash(ds) % 60) as f32 - 30.0) * 0.003;
            let deform_z = ((ruin_hash(ds.wrapping_add(5)) % 60) as f32 - 30.0) * 0.003;

            // Stones slightly offset for aged look
            let offset_x = ((ruin_hash(ds.wrapping_add(11)) % 40) as f32 - 20.0) * 0.004;
            let offset_z = ((ruin_hash(ds.wrapping_add(17)) % 40) as f32 - 20.0) * 0.004;

            let pos = Vec3::new(
                base.x + corner.x + deform_x + offset_x,
                y,
                base.z + corner.z + deform_z + offset_z,
            );

            // Normal faces outward from center
            let normal = Vec3::new(corner.x, 0.0, corner.z).normalize();

            // Darker at base, lichen patches
            let shade = 0.55 + t * 0.35 + 0.1;
            let lichen = if t < 0.4 && ci % 2 == 0 { 0.02 } else { 0.0 };
            let var = (ruin_hash(ds.wrapping_add(23)) % 20) as f32 * 0.003;
            let c = [
                (color[0] + var) * shade,
                (color[1] + lichen + var) * shade,
                (color[2] + var) * shade,
            ];

            verts.push(Vertex {
                position: pos.to_array(),
                color: c,
                normal: normal.to_array(),
            });
        }
    }

    // Index the 4 quad faces
    for step in 0..steps {
        for face in 0..4u32 {
            let next_face = (face + 1) % 4;
            let bl = base_vi + step as u32 * 4 + face;
            let br = base_vi + step as u32 * 4 + next_face;
            let tl = base_vi + (step as u32 + 1) * 4 + face;
            let tr = base_vi + (step as u32 + 1) * 4 + next_face;
            idxs.extend_from_slice(&[bl, tl, br]);
            idxs.extend_from_slice(&[br, tl, tr]);
        }
    }

    // Top face
    let top_start = base_vi + steps as u32 * 4;
    idxs.extend_from_slice(&[top_start, top_start + 1, top_start + 2]);
    idxs.extend_from_slice(&[top_start, top_start + 2, top_start + 3]);

    (verts, idxs)
}

/// Helper: build a cylinder (column shape).
fn build_cylinder(
    center: Vec3,
    height: f32,
    radius_bottom: f32,
    radius_top: f32,
    rings: usize,
    segments: usize,
    color: [f32; 3],
    seed: u32,
    lean_x: f32,
    lean_z: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();
    let base_vi = 0u32;

    for ring in 0..=rings {
        let t = ring as f32 / rings as f32;
        let y = center.y + t * height;
        let r = radius_bottom + (radius_top - radius_bottom) * t;

        let cx = center.x + lean_x * t * height;
        let cz = center.z + lean_z * t * height;

        for seg in 0..=segments {
            let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;

            let ds = seed.wrapping_add(ring as u32 * 19 + seg as u32 * 43);
            let deform = 0.85 + (ruin_hash(ds) % 100) as f32 * 0.003;
            let actual_r = r * deform;

            let nx = angle.cos();
            let nz = angle.sin();
            let pos = Vec3::new(cx + nx * actual_r, y, cz + nz * actual_r);
            let normal = Vec3::new(nx, 0.1, nz).normalize();

            let shade = 0.55 + t * 0.35 + 0.1;
            let lichen = if t < 0.3 && nz < -0.2 { 0.025 } else { 0.0 };
            let var = (ruin_hash(ds.wrapping_add(11)) % 20) as f32 * 0.003;
            let c = [
                (color[0] + var) * shade,
                (color[1] + lichen + var) * shade,
                (color[2] + var) * shade,
            ];

            verts.push(Vertex {
                position: pos.to_array(),
                color: c,
                normal: normal.to_array(),
            });
        }
    }

    for ring in 0..rings {
        for seg in 0..segments {
            let current = base_vi + (ring * (segments + 1) + seg) as u32;
            let next = current + (segments + 1) as u32;
            idxs.extend_from_slice(&[current, next, current + 1]);
            idxs.extend_from_slice(&[current + 1, next, next + 1]);
        }
    }

    (verts, idxs)
}

fn append_local(
    all_verts: &mut Vec<Vertex>,
    all_idxs: &mut Vec<u32>,
    new_verts: &[Vertex],
    new_idxs: &[u32],
) {
    let offset = all_verts.len() as u32;
    all_verts.extend_from_slice(new_verts);
    for idx in new_idxs {
        all_idxs.push(idx + offset);
    }
}

/// Generate a broken stone wall section — crumbling ancient ruin.
pub fn generate_wall_fragment(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let color = ruin_color(seed);

    // Dimensions
    let height = 1.5 + (seed % 7) as f32 * 0.25; // 1.5 - 3.0
    let half_width = 1.0 + (seed % 5) as f32 * 0.2; // half of 2.0 - 4.0
    let half_thick = 0.2 + (seed % 3) as f32 * 0.05; // half of 0.4 - 0.6

    // Rotation for variety
    let rot_angle = (seed % 360) as f32 * std::f32::consts::PI / 180.0;
    let cos_r = rot_angle.cos();
    let sin_r = rot_angle.sin();

    // Crumbling top: each corner gets a different height
    let h0 = height * (0.6 + (ruin_hash(seed) % 40) as f32 * 0.01);
    let h1 = height * (0.7 + (ruin_hash(seed.wrapping_add(1)) % 30) as f32 * 0.01);
    let h2 = height * (0.5 + (ruin_hash(seed.wrapping_add(2)) % 50) as f32 * 0.01);
    let h3 = height * (0.8 + (ruin_hash(seed.wrapping_add(3)) % 20) as f32 * 0.01);

    // Rotate corners
    let raw_corners = [
        Vec3::new(-half_width, 0.0, -half_thick),
        Vec3::new(half_width, 0.0, -half_thick),
        Vec3::new(half_width, 0.0, half_thick),
        Vec3::new(-half_width, 0.0, half_thick),
    ];

    let corners: [Vec3; 4] = [
        Vec3::new(
            raw_corners[0].x * cos_r - raw_corners[0].z * sin_r,
            0.0,
            raw_corners[0].x * sin_r + raw_corners[0].z * cos_r,
        ),
        Vec3::new(
            raw_corners[1].x * cos_r - raw_corners[1].z * sin_r,
            0.0,
            raw_corners[1].x * sin_r + raw_corners[1].z * cos_r,
        ),
        Vec3::new(
            raw_corners[2].x * cos_r - raw_corners[2].z * sin_r,
            0.0,
            raw_corners[2].x * sin_r + raw_corners[2].z * cos_r,
        ),
        Vec3::new(
            raw_corners[3].x * cos_r - raw_corners[3].z * sin_r,
            0.0,
            raw_corners[3].x * sin_r + raw_corners[3].z * cos_r,
        ),
    ];

    let top_heights = [h0, h1, h2, h3];

    build_box(base, &corners, &top_heights, 5, color, seed)
}

/// Generate a half-buried cylindrical column — ancient ruin pillar.
pub fn generate_column(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut all_verts = Vec::new();
    let mut all_idxs = Vec::new();

    let color = ruin_color(seed);

    // Dimensions
    let height = 1.5 + (seed % 5) as f32 * 0.5; // 1.5 - 3.5
    let radius = 0.25 + (seed % 4) as f32 * 0.05; // 0.25 - 0.4
    let radius_top = radius * (0.85 + (seed % 3) as f32 * 0.03); // slight taper

    // Slight lean
    let lean_x = ((ruin_hash(seed) % 80) as f32 - 40.0) * 0.003;
    let lean_z = ((ruin_hash(seed.wrapping_add(7)) % 80) as f32 - 40.0) * 0.003;

    // Optional square base pedestal
    let has_pedestal = ruin_hash(seed.wrapping_add(33)) % 3 != 0; // 2/3 chance
    if has_pedestal {
        let ped_size = radius * 1.6;
        let ped_h = 0.1;
        let corners = [
            Vec3::new(-ped_size, 0.0, -ped_size),
            Vec3::new(ped_size, 0.0, -ped_size),
            Vec3::new(ped_size, 0.0, ped_size),
            Vec3::new(-ped_size, 0.0, ped_size),
        ];
        let top_heights = [ped_h; 4];
        let (v, i) = build_box(base, &corners, &top_heights, 1, color, seed.wrapping_add(50));
        append_local(&mut all_verts, &mut all_idxs, &v, &i);
    }

    // Column body
    let col_base_y = if has_pedestal { base.y + 0.1 } else { base.y };
    let col_base = Vec3::new(base.x, col_base_y, base.z);
    let (v, i) = build_cylinder(col_base, height, radius, radius_top, 6, 6, color, seed, lean_x, lean_z);
    append_local(&mut all_verts, &mut all_idxs, &v, &i);

    // Cap the top — broken or flat
    let is_broken = ruin_hash(seed.wrapping_add(55)) % 2 == 0;
    if !is_broken {
        // Flat cap — fan from center
        let top_cx = base.x + lean_x * height;
        let top_cz = base.z + lean_z * height;
        let top_y = col_base_y + height;
        let cap_vi = all_verts.len() as u32;
        all_verts.push(Vertex {
            position: [top_cx, top_y, top_cz],
            color: [color[0] * 0.9, color[1] * 0.9, color[2] * 0.9],
            normal: [0.0, 1.0, 0.0],
        });
        let segments = 6usize;
        // Add ring vertices for cap
        for seg in 0..=segments {
            let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;
            let ds = seed.wrapping_add(seg as u32 * 47);
            let deform = 0.85 + (ruin_hash(ds) % 100) as f32 * 0.003;
            let r = radius_top * deform;
            let pos = Vec3::new(top_cx + angle.cos() * r, top_y, top_cz + angle.sin() * r);
            all_verts.push(Vertex {
                position: pos.to_array(),
                color: [color[0] * 0.85, color[1] * 0.85, color[2] * 0.85],
                normal: [0.0, 1.0, 0.0],
            });
        }
        let ring_start = cap_vi + 1;
        for seg in 0..segments {
            let a = ring_start + seg as u32;
            let b = ring_start + seg as u32 + 1;
            all_idxs.extend_from_slice(&[cap_vi, a, b]);
        }
    }

    (all_verts, all_idxs)
}

/// Generate an arch fragment — two short columns with a connecting lintel.
pub fn generate_arch_fragment(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut all_verts = Vec::new();
    let mut all_idxs = Vec::new();

    let color = ruin_color(seed);

    // Dimensions
    let col_height_l = 1.0 + (seed % 5) as f32 * 0.2; // 1.0 - 1.8 left column
    // Right column can be shorter (partially collapsed)
    let collapsed = ruin_hash(seed.wrapping_add(77)) % 3 == 0; // 1/3 chance
    let col_height_r = if collapsed {
        col_height_l * (0.4 + (seed % 3) as f32 * 0.1)
    } else {
        col_height_l * (0.85 + (seed % 3) as f32 * 0.05)
    };

    let gap = 0.75 + (seed % 4) as f32 * 0.25; // half of 1.5 - 2.5
    let col_radius = 0.2 + (seed % 3) as f32 * 0.03;

    // Rotation for variety
    let rot_angle = (seed % 360) as f32 * std::f32::consts::PI / 180.0;
    let cos_r = rot_angle.cos();
    let sin_r = rot_angle.sin();

    // Left column position
    let left_offset = Vec3::new(-gap * cos_r, 0.0, -gap * sin_r);
    let left_base = base + left_offset;
    let lean_l_x = ((ruin_hash(seed.wrapping_add(1)) % 40) as f32 - 20.0) * 0.002;
    let lean_l_z = ((ruin_hash(seed.wrapping_add(2)) % 40) as f32 - 20.0) * 0.002;
    let (v, i) = build_cylinder(left_base, col_height_l, col_radius, col_radius * 0.9, 4, 6, color, seed.wrapping_add(10), lean_l_x, lean_l_z);
    append_local(&mut all_verts, &mut all_idxs, &v, &i);

    // Right column position
    let right_offset = Vec3::new(gap * cos_r, 0.0, gap * sin_r);
    let right_base = base + right_offset;
    let lean_r_x = ((ruin_hash(seed.wrapping_add(3)) % 40) as f32 - 20.0) * 0.002;
    let lean_r_z = ((ruin_hash(seed.wrapping_add(4)) % 40) as f32 - 20.0) * 0.002;
    let (v, i) = build_cylinder(right_base, col_height_r, col_radius, col_radius * 0.9, 4, 6, color, seed.wrapping_add(20), lean_r_x, lean_r_z);
    append_local(&mut all_verts, &mut all_idxs, &v, &i);

    // Lintel across the top (only if not collapsed too much)
    if col_height_r > col_height_l * 0.5 {
        let lintel_y = col_height_r.min(col_height_l); // rest on shorter column
        let lintel_half_len = gap + col_radius;
        let lintel_half_h = 0.12 + (seed % 3) as f32 * 0.03;
        let lintel_half_d = col_radius * 0.8;

        // Lintel as a box rotated to match the arch
        let corners = [
            Vec3::new(-lintel_half_len * cos_r - (-lintel_half_d) * sin_r, 0.0,
                      -lintel_half_len * sin_r + (-lintel_half_d) * cos_r),
            Vec3::new(lintel_half_len * cos_r - (-lintel_half_d) * sin_r, 0.0,
                      lintel_half_len * sin_r + (-lintel_half_d) * cos_r),
            Vec3::new(lintel_half_len * cos_r - lintel_half_d * sin_r, 0.0,
                      lintel_half_len * sin_r + lintel_half_d * cos_r),
            Vec3::new(-lintel_half_len * cos_r - lintel_half_d * sin_r, 0.0,
                      -lintel_half_len * sin_r + lintel_half_d * cos_r),
        ];

        // Cracked lintel — middle sags or has gap
        let crack = ruin_hash(seed.wrapping_add(88)) % 2 == 0;
        let h_vals = if crack {
            // Slightly different heights to suggest cracking
            [
                lintel_half_h * 2.0,
                lintel_half_h * 2.0 * 0.7,
                lintel_half_h * 2.0 * 0.8,
                lintel_half_h * 2.0,
            ]
        } else {
            [lintel_half_h * 2.0; 4]
        };

        let lintel_base = Vec3::new(base.x, base.y + lintel_y, base.z);
        let (v, i) = build_box(lintel_base, &corners, &h_vals, 2, color, seed.wrapping_add(30));
        append_local(&mut all_verts, &mut all_idxs, &v, &i);
    }

    (all_verts, all_idxs)
}

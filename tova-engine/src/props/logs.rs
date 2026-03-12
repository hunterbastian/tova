use glam::Vec3;
use crate::renderer::Vertex;

/// Deterministic hash for variation.
fn log_hash(seed: u32) -> u32 {
    seed.wrapping_mul(2654435761)
}

/// Generate a fallen log — decaying wood lying on the ground, mossy.
pub fn generate_fallen_log(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    // Dimensions
    let length = 2.0 + (seed % 7) as f32 * 0.5; // 2.0 - 5.0
    let radius = 0.2 + (seed % 5) as f32 * 0.05; // 0.2 - 0.4

    // Random rotation around Y axis
    let rot_angle = (seed % 360) as f32 * std::f32::consts::PI / 180.0;
    let cos_r = rot_angle.cos();
    let sin_r = rot_angle.sin();

    // Base dark brown-grey color
    let base_color = [
        0.18 + (seed % 4) as f32 * 0.01,
        0.15 + (seed % 3) as f32 * 0.008,
        0.12 + (seed % 5) as f32 * 0.006,
    ];

    let segments_around = 6usize;
    let segments_along = 4usize;
    let base_vi = verts.len() as u32;

    // Broken end: the far end has smaller, more irregular radius
    let broken_end = log_hash(seed.wrapping_add(99)) % 2 == 0;

    for along in 0..=segments_along {
        let t = along as f32 / segments_along as f32;

        // Position along the log's local axis (before rotation)
        let local_x = (t - 0.5) * length;

        // Sagging: log dips in the middle
        let sag = -(t * (1.0 - t)) * 0.15 * length;

        // Radius variation along length
        let mut r = radius;
        if broken_end && t > 0.7 {
            // Taper and deform the broken end
            let break_t = (t - 0.7) / 0.3;
            r *= 1.0 - break_t * 0.5;
        }

        for seg in 0..=segments_around {
            let angle = (seg as f32 / segments_around as f32) * std::f32::consts::TAU;

            // Normal in cylinder local space (Y is up when lying flat)
            let ny_local = angle.cos();
            let nz_local = angle.sin();

            // Deform for organic shape
            let deform_seed = seed.wrapping_add(along as u32 * 17 + seg as u32 * 41);
            let deform = 0.85 + (log_hash(deform_seed) % 100) as f32 * 0.003;

            // Extra irregularity at broken end
            let break_deform = if broken_end && t > 0.7 {
                0.7 + (log_hash(deform_seed.wrapping_add(7)) % 100) as f32 * 0.006
            } else {
                1.0
            };

            let actual_r = r * deform * break_deform;

            // Position in log-local space: log lies along X, cross-section in Y/Z
            let ly = actual_r * ny_local + sag;
            let lz = actual_r * nz_local;

            // Lift the log so it sits on the ground (center at radius height)
            let world_y = base.y + radius + ly;

            // Rotate around Y axis
            let rx = local_x * cos_r - lz * sin_r;
            let rz = local_x * sin_r + lz * cos_r;

            let pos = Vec3::new(base.x + rx, world_y, base.z + rz);

            // Normal: rotate the cylinder normal
            let normal_local = Vec3::new(0.0, ny_local, nz_local).normalize();
            let normal = Vec3::new(
                -normal_local.z * sin_r,
                normal_local.y,
                normal_local.z * cos_r,
            ).normalize();

            // Color: mossy on top, darker underneath
            let mut c = base_color;

            // Shade based on vertical facing
            let shade = if normal.y > 0.3 {
                // Top-facing: mossy green tint
                c[1] += 0.06 + (log_hash(deform_seed.wrapping_add(13)) % 30) as f32 * 0.002;
                c[0] += 0.01;
                0.85
            } else if normal.y < -0.2 {
                // Underside: darker
                0.55
            } else {
                // Sides
                0.7
            };

            // Per-vertex color variation
            let var = (log_hash(deform_seed.wrapping_add(23)) % 30) as f32 * 0.002;
            let final_color = [
                (c[0] + var) * shade,
                (c[1] + var) * shade,
                (c[2] + var) * shade,
            ];

            verts.push(Vertex {
                position: pos.to_array(),
                color: final_color,
                normal: normal.to_array(),
            });
        }
    }

    // Index the rings into triangles
    for along in 0..segments_along {
        for seg in 0..segments_around {
            let current = base_vi + (along * (segments_around + 1) + seg) as u32;
            let next = current + (segments_around + 1) as u32;
            idxs.extend_from_slice(&[current, next, current + 1]);
            idxs.extend_from_slice(&[current + 1, next, next + 1]);
        }
    }

    // Cap the near end (t=0) with a fan
    let cap0_center_vi = verts.len() as u32;
    let cap0_pos = Vec3::new(
        base.x + (-0.5 * length) * cos_r,
        base.y + radius,
        base.z + (-0.5 * length) * sin_r,
    );
    verts.push(Vertex {
        position: cap0_pos.to_array(),
        color: [base_color[0] * 0.6, base_color[1] * 0.6, base_color[2] * 0.6],
        normal: [(-cos_r), 0.0, (-sin_r)],
    });
    let ring0_start = base_vi;
    for seg in 0..segments_around {
        let a = ring0_start + seg as u32;
        let b = ring0_start + seg as u32 + 1;
        idxs.extend_from_slice(&[cap0_center_vi, b, a]);
    }

    // Cap the far end (t=1) — if broken, skip cap for jagged look
    if !broken_end {
        let cap1_center_vi = verts.len() as u32;
        let cap1_pos = Vec3::new(
            base.x + (0.5 * length) * cos_r,
            base.y + radius,
            base.z + (0.5 * length) * sin_r,
        );
        verts.push(Vertex {
            position: cap1_pos.to_array(),
            color: [base_color[0] * 0.6, base_color[1] * 0.6, base_color[2] * 0.6],
            normal: [cos_r, 0.0, sin_r],
        });
        let ring_last_start = base_vi + (segments_along * (segments_around + 1)) as u32;
        for seg in 0..segments_around {
            let a = ring_last_start + seg as u32;
            let b = ring_last_start + seg as u32 + 1;
            idxs.extend_from_slice(&[cap1_center_vi, a, b]);
        }
    }

    (verts, idxs)
}

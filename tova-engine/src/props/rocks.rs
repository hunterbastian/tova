use glam::Vec3;
use crate::renderer::Vertex;

/// Deterministic hash for variation.
fn rock_hash(seed: u32) -> u32 {
    seed.wrapping_mul(2654435761)
}

/// Generate a boulder — irregular, low-poly rock sitting on the ground.
pub fn generate_boulder(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    // Size variation
    let scale = 0.4 + (seed % 6) as f32 * 0.15;
    let squash_y = 0.6 + (seed % 4) as f32 * 0.1;

    // Rock color — dark fantasy palette: deep grey, cold stone, dark moss
    let tint = (seed % 4) as f32;
    let color = if tint < 1.0 {
        // Dark basalt
        [0.22 + (seed % 5) as f32 * 0.008, 0.21 + (seed % 4) as f32 * 0.006, 0.24]
    } else if tint < 2.0 {
        // Cold dark grey
        [0.26 + (seed % 4) as f32 * 0.01, 0.24, 0.22 + (seed % 3) as f32 * 0.008]
    } else if tint < 3.0 {
        // Dark mossy — sickly green-grey
        [0.22, 0.26 + (seed % 4) as f32 * 0.008, 0.23]
    } else {
        // Near-black weathered stone
        [0.18 + (seed % 5) as f32 * 0.006, 0.17, 0.20]
    };

    // Deformed sphere — low resolution for chunky, natural look
    let lat_steps = 4usize;
    let lon_steps = 6usize;
    let center = base + Vec3::Y * (scale * squash_y * 0.4); // half-buried
    let base_vi = verts.len() as u32;

    for lat in 0..=lat_steps {
        let theta = (lat as f32 / lat_steps as f32) * std::f32::consts::PI;
        for lon in 0..=lon_steps {
            let phi = (lon as f32 / lon_steps as f32) * std::f32::consts::TAU;

            let nx = theta.sin() * phi.cos();
            let ny = theta.cos();
            let nz = theta.sin() * phi.sin();
            let normal = Vec3::new(nx, ny, nz);

            // Heavy deformation for chunky rock shape
            let deform_seed = seed.wrapping_add(lat as u32 * 13 + lon as u32 * 37);
            let deform = 0.7 + (deform_seed.wrapping_mul(2654435761) % 100) as f32 * 0.006;
            let r = scale * deform;

            let pos = center + Vec3::new(
                normal.x * r,
                normal.y * r * squash_y,
                normal.z * r,
            );

            // Darken bottom, lighter top
            let shade = 0.65 + ny * 0.2 + 0.15;
            let c = [color[0] * shade, color[1] * shade, color[2] * shade];

            verts.push(Vertex {
                position: pos.to_array(),
                color: c,
                normal: normal.to_array(),
            });
        }
    }

    for lat in 0..lat_steps {
        for lon in 0..lon_steps {
            let current = base_vi + (lat * (lon_steps + 1) + lon) as u32;
            let next = current + (lon_steps + 1) as u32;

            if lat != 0 {
                idxs.extend_from_slice(&[current, next, current + 1]);
            }
            if lat != lat_steps - 1 {
                idxs.extend_from_slice(&[current + 1, next, next + 1]);
            }
        }
    }

    (verts, idxs)
}

/// Generate a small rock cluster — 2-3 small stones grouped together.
pub fn generate_rock_cluster(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut all_verts = Vec::new();
    let mut all_idxs = Vec::new();

    let count = 2 + (seed % 2) as usize;
    for i in 0..count {
        let s = seed.wrapping_add(i as u32 * 97);
        let offset = Vec3::new(
            ((s >> 4) % 40) as f32 * 0.01 - 0.2,
            0.0,
            ((s >> 8) % 40) as f32 * 0.01 - 0.2,
        );

        // Small stones
        let stone_base = base + offset;
        let scale = 0.15 + (s % 4) as f32 * 0.05;
        let squash = 0.5 + (s % 3) as f32 * 0.15;

        let color = [
            0.22 + (s % 5) as f32 * 0.01,
            0.20 + (s % 4) as f32 * 0.008,
            0.23 + (s % 3) as f32 * 0.008,
        ];

        let center = stone_base + Vec3::Y * (scale * squash * 0.35);
        let lat_steps = 3usize;
        let lon_steps = 5usize;
        let base_vi = all_verts.len() as u32;

        for lat in 0..=lat_steps {
            let theta = (lat as f32 / lat_steps as f32) * std::f32::consts::PI;
            for lon in 0..=lon_steps {
                let phi = (lon as f32 / lon_steps as f32) * std::f32::consts::TAU;

                let nx = theta.sin() * phi.cos();
                let ny = theta.cos();
                let nz = theta.sin() * phi.sin();
                let normal = Vec3::new(nx, ny, nz);

                let ds = s.wrapping_add(lat as u32 * 11 + lon as u32 * 29);
                let deform = 0.75 + (ds.wrapping_mul(2654435761) % 100) as f32 * 0.005;
                let r = scale * deform;

                let pos = center + Vec3::new(normal.x * r, normal.y * r * squash, normal.z * r);
                let shade = 0.7 + ny * 0.15 + 0.15;
                let c = [color[0] * shade, color[1] * shade, color[2] * shade];

                all_verts.push(Vertex {
                    position: pos.to_array(),
                    color: c,
                    normal: normal.to_array(),
                });
            }
        }

        for lat in 0..lat_steps {
            for lon in 0..lon_steps {
                let current = base_vi + (lat * (lon_steps + 1) + lon) as u32;
                let next = current + (lon_steps + 1) as u32;
                if lat != 0 {
                    all_idxs.extend_from_slice(&[current, next, current + 1]);
                }
                if lat != lat_steps - 1 {
                    all_idxs.extend_from_slice(&[current + 1, next, next + 1]);
                }
            }
        }
    }

    (all_verts, all_idxs)
}

/// Generate a rock spire — tall, jagged stone column jutting from the ground.
/// Dark fantasy staple: natural stone pillars, weathered and imposing.
pub fn generate_rock_spire(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    // Height and width variation
    let height = 2.5 + (seed % 8) as f32 * 0.6; // 2.5 - 6.7 blocks tall
    let base_radius = 0.35 + (seed % 4) as f32 * 0.08;
    let taper = 0.15 + (seed % 3) as f32 * 0.05; // top radius fraction

    // Slight lean for organic feel
    let lean_x = ((rock_hash(seed) % 100) as f32 - 50.0) * 0.004;
    let lean_z = ((rock_hash(seed.wrapping_add(7)) % 100) as f32 - 50.0) * 0.004;

    // Dark stone color
    let tint = seed % 3;
    let base_color = match tint {
        0 => [0.20, 0.19, 0.24], // dark blue-grey
        1 => [0.18, 0.20, 0.22], // cold dark
        _ => [0.24, 0.22, 0.20], // dark warm grey
    };

    let rings = 6usize;
    let segments = 6usize;
    let base_vi = verts.len() as u32;

    for ring in 0..=rings {
        let t = ring as f32 / rings as f32;
        let y = base.y + t * height;
        let radius = base_radius * (1.0 - t * (1.0 - taper));

        // Lean accumulates with height
        let cx = base.x + lean_x * t * height;
        let cz = base.z + lean_z * t * height;

        for seg in 0..=segments {
            let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;

            // Deform each vertex for jagged, weathered look
            let deform_seed = seed.wrapping_add(ring as u32 * 17 + seg as u32 * 41);
            let deform = 0.7 + (rock_hash(deform_seed) % 100) as f32 * 0.006;
            // Extra jaggedness near the top
            let top_jag = if t > 0.7 {
                0.6 + (rock_hash(deform_seed.wrapping_add(3)) % 100) as f32 * 0.008
            } else {
                1.0
            };
            let r = radius * deform * top_jag;

            let nx = angle.cos();
            let nz = angle.sin();
            let pos = Vec3::new(cx + nx * r, y, cz + nz * r);
            let normal = Vec3::new(nx, 0.15, nz).normalize();

            // Darker at base, lighter at top (sky exposure)
            let shade = 0.6 + t * 0.3 + 0.1;
            // Slight moss tint on north-facing side
            let moss = if nz < -0.3 && t < 0.6 { 0.03 } else { 0.0 };
            let c = [
                base_color[0] * shade,
                (base_color[1] + moss) * shade,
                base_color[2] * shade,
            ];

            verts.push(Vertex {
                position: pos.to_array(),
                color: c,
                normal: normal.to_array(),
            });
        }
    }

    // Index the rings into triangles
    for ring in 0..rings {
        for seg in 0..segments {
            let current = base_vi + (ring * (segments + 1) + seg) as u32;
            let next = current + (segments + 1) as u32;
            idxs.extend_from_slice(&[current, next, current + 1]);
            idxs.extend_from_slice(&[current + 1, next, next + 1]);
        }
    }

    // Cap the top with a rough point
    let top_center_y = base.y + height * 1.05; // slightly beyond last ring
    let top_cx = base.x + lean_x * height;
    let top_cz = base.z + lean_z * height;
    let tip_vi = verts.len() as u32;
    let shade = 0.95;
    verts.push(Vertex {
        position: [top_cx, top_center_y, top_cz],
        color: [base_color[0] * shade, base_color[1] * shade, base_color[2] * shade],
        normal: [0.0, 1.0, 0.0],
    });
    let last_ring_start = base_vi + (rings * (segments + 1)) as u32;
    for seg in 0..segments {
        let a = last_ring_start + seg as u32;
        let b = last_ring_start + seg as u32 + 1;
        idxs.extend_from_slice(&[a, tip_vi, b]);
    }

    (verts, idxs)
}

/// Generate a standing stone — smooth ancient monolith, slightly leaning.
/// Menhir-like: rectangular slab, weathered edges, feels placed deliberately.
pub fn generate_standing_stone(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    // Slab dimensions
    let height = 1.8 + (seed % 6) as f32 * 0.5; // 1.8 - 4.3
    let width = 0.3 + (seed % 3) as f32 * 0.1;
    let depth = 0.15 + (seed % 2) as f32 * 0.06;

    // Slight lean and rotation
    let lean_x = ((rock_hash(seed) % 80) as f32 - 40.0) * 0.003;
    let lean_z = ((rock_hash(seed.wrapping_add(13)) % 80) as f32 - 40.0) * 0.003;
    let rot_angle = (seed % 360) as f32 * std::f32::consts::PI / 180.0;
    let cos_r = rot_angle.cos();
    let sin_r = rot_angle.sin();

    // Ancient dark stone — nearly black
    let tint = seed % 3;
    let base_color = match tint {
        0 => [0.16, 0.15, 0.18], // dark slate
        1 => [0.18, 0.17, 0.16], // dark granite
        _ => [0.14, 0.16, 0.18], // blue-black
    };

    // Build as a deformed box: 4 vertical edges, top and bottom faces
    let steps = 4usize; // vertical subdivisions
    let base_vi = verts.len() as u32;

    // 4 corners at each height step
    let corners = [
        Vec3::new(-width, 0.0, -depth),
        Vec3::new(width, 0.0, -depth),
        Vec3::new(width, 0.0, depth),
        Vec3::new(-width, 0.0, depth),
    ];
    let normals = [
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(-1.0, 0.0, 0.0),
    ];

    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let y = base.y + t * height;
        let lean_offset_x = lean_x * t * height;
        let lean_offset_z = lean_z * t * height;

        // Slight taper toward top
        let taper = 1.0 - t * 0.15;
        // Weathering deformation
        let weather = (rock_hash(seed.wrapping_add(step as u32 * 23)) % 100) as f32 * 0.001;

        for (ci, corner) in corners.iter().enumerate() {
            let deform = (rock_hash(seed.wrapping_add(step as u32 * 13 + ci as u32 * 7)) % 100) as f32 * 0.002;
            let cx = corner.x * taper + deform;
            let cz = corner.z * taper + weather;

            // Rotate around Y
            let rx = cx * cos_r - cz * sin_r;
            let rz = cx * sin_r + cz * cos_r;

            let pos = Vec3::new(
                base.x + rx + lean_offset_x,
                y,
                base.z + rz + lean_offset_z,
            );

            let n = Vec3::new(
                normals[ci].x * cos_r - normals[ci].z * sin_r,
                normals[ci].y,
                normals[ci].x * sin_r + normals[ci].z * cos_r,
            ).normalize();

            // Lichen / moss patches near base
            let shade = 0.55 + t * 0.35 + 0.1;
            let lichen = if t < 0.4 && ci % 2 == 0 { 0.02 } else { 0.0 };
            let c = [
                base_color[0] * shade,
                (base_color[1] + lichen) * shade,
                base_color[2] * shade,
            ];

            verts.push(Vertex {
                position: pos.to_array(),
                color: c,
                normal: n.to_array(),
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

/// Generate rubble — scattered angular stone fragments. Debris at cliff bases.
pub fn generate_rubble(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut all_verts = Vec::new();
    let mut all_idxs = Vec::new();

    let count = 3 + (seed % 4) as usize; // 3-6 fragments

    for i in 0..count {
        let s = seed.wrapping_add(i as u32 * 71);
        let offset = Vec3::new(
            ((rock_hash(s) % 60) as f32 - 30.0) * 0.015,
            0.0,
            ((rock_hash(s.wrapping_add(5)) % 60) as f32 - 30.0) * 0.015,
        );

        let pos = base + offset;
        let size = 0.08 + (s % 5) as f32 * 0.04; // small angular chunks

        // Dark stone fragment
        let shade_var = (rock_hash(s.wrapping_add(11)) % 40) as f32 * 0.005;
        let color = [0.19 + shade_var, 0.18 + shade_var, 0.21 + shade_var];

        // Simple angular shape: irregular tetrahedron-ish
        let base_vi = all_verts.len() as u32;
        let h = size * (0.5 + (s % 3) as f32 * 0.3);

        // Random rotation
        let rot = (s % 360) as f32 * std::f32::consts::PI / 180.0;
        let cr = rot.cos();
        let sr = rot.sin();

        // 5 vertices: 4 base corners + top
        let corners = [
            Vec3::new(-size * cr, 0.0, -size * sr),
            Vec3::new(size * cr, 0.0, -size * sr * 0.7),
            Vec3::new(size * cr * 0.6, 0.0, size * sr),
            Vec3::new(-size * cr * 0.8, 0.0, size * sr * 0.6),
        ];

        // Deformed top point
        let top_off_x = ((rock_hash(s.wrapping_add(3)) % 40) as f32 - 20.0) * 0.004;
        let top_off_z = ((rock_hash(s.wrapping_add(9)) % 40) as f32 - 20.0) * 0.004;
        let top = Vec3::new(top_off_x, h, top_off_z);

        // Emit triangles from each base edge to top
        for ci in 0..4usize {
            let next = (ci + 1) % 4;
            let a = pos + corners[ci];
            let b = pos + corners[next];
            let c = pos + top;

            let edge1 = b - a;
            let edge2 = c - a;
            let normal = edge1.cross(edge2).normalize();

            let shade_top = 0.85;
            let shade_bot = 0.6;

            let vi = all_verts.len() as u32;
            all_verts.push(Vertex {
                position: a.to_array(),
                color: [color[0] * shade_bot, color[1] * shade_bot, color[2] * shade_bot],
                normal: normal.to_array(),
            });
            all_verts.push(Vertex {
                position: b.to_array(),
                color: [color[0] * shade_bot, color[1] * shade_bot, color[2] * shade_bot],
                normal: normal.to_array(),
            });
            all_verts.push(Vertex {
                position: c.to_array(),
                color: [color[0] * shade_top, color[1] * shade_top, color[2] * shade_top],
                normal: normal.to_array(),
            });
            all_idxs.extend_from_slice(&[vi, vi + 1, vi + 2]);
        }

        // Bottom face
        let vi = all_verts.len() as u32;
        for corner in &corners {
            let p = pos + *corner;
            all_verts.push(Vertex {
                position: p.to_array(),
                color: [color[0] * 0.5, color[1] * 0.5, color[2] * 0.5],
                normal: [0.0, -1.0, 0.0],
            });
        }
        all_idxs.extend_from_slice(&[vi, vi + 2, vi + 1]);
        all_idxs.extend_from_slice(&[vi, vi + 3, vi + 2]);
    }

    (all_verts, all_idxs)
}

/// Generate a giant rock spire — towering jagged stone formation.
/// Massive, imposing pillars that dominate the landscape. Multiple
/// columns fused together with heavy deformation for an organic, ancient feel.
pub fn generate_giant_spire(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut all_verts = Vec::new();
    let mut all_idxs = Vec::new();

    // Main spire + 1-2 smaller companion spires clustered together
    let companion_count = 1 + (seed % 2) as usize;

    for spire_idx in 0..=companion_count {
        let s = seed.wrapping_add(spire_idx as u32 * 137);

        // Main spire is tallest, companions are shorter
        let (height, base_radius) = if spire_idx == 0 {
            let h = 12.0 + (s % 10) as f32 * 1.5; // 12 - 25.5 blocks tall
            let r = 1.2 + (s % 4) as f32 * 0.3;
            (h, r)
        } else {
            let h = 5.0 + (s % 8) as f32 * 1.0;
            let r = 0.6 + (s % 3) as f32 * 0.2;
            (h, r)
        };

        // Companion offset from center
        let offset = if spire_idx == 0 {
            Vec3::ZERO
        } else {
            let angle = (rock_hash(s) % 360) as f32 * std::f32::consts::PI / 180.0;
            let dist = 1.5 + (s % 3) as f32 * 0.5;
            Vec3::new(angle.cos() * dist, 0.0, angle.sin() * dist)
        };

        let spire_base = base + offset;
        let taper = 0.08 + (s % 4) as f32 * 0.03;

        // Giants lean more dramatically
        let lean_x = ((rock_hash(s) % 100) as f32 - 50.0) * 0.002;
        let lean_z = ((rock_hash(s.wrapping_add(7)) % 100) as f32 - 50.0) * 0.002;

        // Dark, imposing stone color
        let tint = s % 4;
        let base_color = match tint {
            0 => [0.16, 0.15, 0.20], // dark blue-slate
            1 => [0.14, 0.14, 0.17], // near-black
            2 => [0.18, 0.16, 0.15], // dark charcoal
            _ => [0.15, 0.17, 0.20], // cold dark blue
        };

        let rings = 10usize;
        let segments = 8usize;
        let base_vi = all_verts.len() as u32;

        for ring in 0..=rings {
            let t = ring as f32 / rings as f32;
            let y = spire_base.y + t * height;

            // Non-linear taper — wider base shelf, then narrows dramatically
            let taper_curve = if t < 0.15 {
                1.0 - t * 0.3
            } else {
                let tt = (t - 0.15) / 0.85;
                0.955 * (1.0 - tt.powf(0.7) * (1.0 - taper))
            };
            let radius = base_radius * taper_curve;

            let cx = spire_base.x + lean_x * t * height;
            let cz = spire_base.z + lean_z * t * height;

            for seg in 0..=segments {
                let angle = (seg as f32 / segments as f32) * std::f32::consts::TAU;

                // Heavy deformation — craggy, weathered surface
                let ds = s.wrapping_add(ring as u32 * 23 + seg as u32 * 53);
                let deform = 0.65 + (rock_hash(ds) % 100) as f32 * 0.007;

                // Vertical ridges
                let ridge = ((angle * 3.0 + s as f32 * 0.1).sin() * 0.15 + 1.0).max(0.85);

                // Extra jaggedness near top
                let top_jag = if t > 0.75 {
                    let jag_seed = rock_hash(ds.wrapping_add(17));
                    0.5 + (jag_seed % 100) as f32 * 0.01
                } else {
                    1.0
                };

                let r = radius * deform * ridge * top_jag;
                let nx = angle.cos();
                let nz = angle.sin();
                let pos = Vec3::new(cx + nx * r, y, cz + nz * r);
                let normal = Vec3::new(nx, 0.1, nz).normalize();

                let shade = 0.45 + t * 0.4 + 0.15;
                // Vertical dark streaks (rain stains)
                let streak = ((angle * 5.0 + s as f32).sin() * 0.5 + 0.5) * 0.06;
                // Moss near base on sheltered side
                let moss = if t < 0.3 && nz < -0.2 { 0.025 } else { 0.0 };

                let c = [
                    (base_color[0] - streak).max(0.0) * shade,
                    (base_color[1] + moss - streak * 0.5).max(0.0) * shade,
                    (base_color[2] - streak * 0.3).max(0.0) * shade,
                ];

                all_verts.push(Vertex {
                    position: pos.to_array(),
                    color: c,
                    normal: normal.to_array(),
                });
            }
        }

        // Index rings
        for ring in 0..rings {
            for seg in 0..segments {
                let current = base_vi + (ring * (segments + 1) + seg) as u32;
                let next = current + (segments + 1) as u32;
                all_idxs.extend_from_slice(&[current, next, current + 1]);
                all_idxs.extend_from_slice(&[current + 1, next, next + 1]);
            }
        }

        // Jagged top cap — fan to a rough peak point
        let top_ring_start = base_vi + (rings * (segments + 1)) as u32;
        let peak_vi = all_verts.len() as u32;
        let peak_shade = 0.9;
        // Offset peak slightly for asymmetry
        let peak_off = ((rock_hash(s.wrapping_add(99)) % 60) as f32 - 30.0) * 0.01;
        all_verts.push(Vertex {
            position: [
                spire_base.x + lean_x * height + peak_off,
                spire_base.y + height * 1.04,
                spire_base.z + lean_z * height - peak_off,
            ],
            color: [base_color[0] * peak_shade, base_color[1] * peak_shade, base_color[2] * peak_shade],
            normal: [0.0, 1.0, 0.0],
        });
        for seg in 0..segments {
            let a = top_ring_start + seg as u32;
            let b = top_ring_start + (seg as u32 + 1) % (segments as u32 + 1);
            all_idxs.extend_from_slice(&[a, peak_vi, b]);
        }
    }

    (all_verts, all_idxs)
}

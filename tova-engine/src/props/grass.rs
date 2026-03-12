use glam::Vec3;
use crate::renderer::Vertex;

/// Deterministic hash for variation.
fn blade_hash(seed: u32, idx: u32) -> u32 {
    let mut h = seed.wrapping_mul(374761393).wrapping_add(idx.wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

/// Map hash to 0..1 float.
fn hash_f(seed: u32, idx: u32) -> f32 {
    (blade_hash(seed, idx) & 0xFFFF) as f32 / 65535.0
}

/// The Isle-inspired grass palettes — warm amber meadow mixed with deep green.
/// Each palette has [root, mid, tip] colors.
const PALETTES: [[[f32; 3]; 3]; 5] = [
    // Deep green meadow
    [[0.22, 0.28, 0.14], [0.30, 0.38, 0.20], [0.38, 0.44, 0.26]],
    // Warm golden-green
    [[0.28, 0.30, 0.16], [0.40, 0.42, 0.24], [0.52, 0.48, 0.30]],
    // Amber/dried — The Isle's signature warm grass
    [[0.32, 0.28, 0.16], [0.46, 0.40, 0.24], [0.56, 0.48, 0.28]],
    // Rich dark green
    [[0.18, 0.26, 0.12], [0.26, 0.34, 0.18], [0.32, 0.40, 0.22]],
    // Yellow-green transitional
    [[0.30, 0.32, 0.18], [0.44, 0.44, 0.26], [0.50, 0.46, 0.28]],
];

/// Generate a dense grass clump — The Isle style.
/// Tall, swaying, dense meadow grass with amber/green variation.
pub fn generate_grass(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    // 3-4 micro-tufts scattered tightly — creates dense coverage
    let num_tufts = 3 + (seed % 2) as usize;

    for t in 0..num_tufts {
        let ts = seed.wrapping_add(t as u32 * 9973);
        let ox = (hash_f(ts, 100) - 0.5) * 0.4;
        let oz = (hash_f(ts, 101) - 0.5) * 0.4;
        let tuft_base = base + Vec3::new(ox, 0.0, oz);

        // 6-10 blades per tuft — dense like The Isle
        let num_blades = 6 + (ts % 5) as usize;

        // Palette: nearby grass tends toward same palette but with per-blade variation
        let palette_idx = (ts % 5) as usize;
        let palette = &PALETTES[palette_idx];

        generate_tuft(
            &mut verts,
            &mut idxs,
            tuft_base,
            ts,
            num_blades,
            palette,
        );
    }

    (verts, idxs)
}

/// Generate a single tuft of curved, tapered grass blades.
fn generate_tuft(
    verts: &mut Vec<Vertex>,
    idxs: &mut Vec<u32>,
    base: Vec3,
    seed: u32,
    num_blades: usize,
    palette: &[[f32; 3]; 3],
) {
    const SEGMENTS: usize = 4; // more segments = smoother curve

    for b in 0..num_blades {
        let bs = seed.wrapping_add(b as u32 * 7919);

        // ─── Blade shape ─────────────────────────────────────
        // Height: tall blades, 0.35 - 0.75 (knee to waist)
        let height = 0.35 + hash_f(bs, 0) * 0.40;

        // Width at base tapers to sharp tip
        let base_width = 0.04 + hash_f(bs, 1) * 0.05; // 0.04 - 0.09

        // Rotation around Y — evenly spread with heavy jitter
        let angle = (b as f32 / num_blades as f32) * std::f32::consts::TAU
            + (hash_f(bs, 2) - 0.5) * 1.2;

        // Lean: each blade arcs outward in a random direction
        let lean_angle = hash_f(bs, 3) * std::f32::consts::TAU;
        let lean_amount = 0.08 + hash_f(bs, 4) * 0.28; // stronger lean for wilder grass

        // ─── Blade color ─────────────────────────────────────
        // Per-blade color variation — mix between palette colors
        let cv = (hash_f(bs, 5) - 0.5) * 0.05;
        // Some blades pick from a different palette for variety
        let alt_palette = &PALETTES[((bs / 7) % 5) as usize];
        let blend = hash_f(bs, 6) * 0.3; // slight cross-palette blending

        let color_root = lerp_color(palette[0], alt_palette[0], blend, cv);
        let color_mid = lerp_color(palette[1], alt_palette[1], blend, cv);
        let color_tip = lerp_color(palette[2], alt_palette[2], blend, cv);

        // Blade direction
        let dir = Vec3::new(angle.cos(), 0.0, angle.sin());
        let perp = Vec3::new(-angle.sin(), 0.0, angle.cos());
        let lean_dir = Vec3::new(lean_angle.cos(), 0.0, lean_angle.sin());

        // ─── Build curved blade ──────────────────────────────
        let vi_start = verts.len() as u32;

        for s in 0..=SEGMENTS {
            let t = s as f32 / SEGMENTS as f32;

            // Height: slight acceleration at base, deceleration at tip
            let y = height * (1.0 - (1.0 - t).powi(2));

            // Width taper: full at base → pointed tip
            let taper = 1.0 - t.powf(0.7); // sharper taper
            let w = base_width * taper * 0.5;

            // Curvature: quadratic lean + slight S-curve
            let primary_lean = lean_dir * lean_amount * t * t;
            let s_curve = dir * (t * t * (1.0 - t)) * 0.06; // subtle S wiggle
            let lean = primary_lean + s_curve;

            let center = base + Vec3::new(0.0, y, 0.0) + lean;

            let left = center - perp * w;
            let right = center + perp * w;

            // Color: root → mid → tip with smooth blend
            let color = if t < 0.5 {
                let lt = t * 2.0;
                [
                    color_root[0] + (color_mid[0] - color_root[0]) * lt,
                    color_root[1] + (color_mid[1] - color_root[1]) * lt,
                    color_root[2] + (color_mid[2] - color_root[2]) * lt,
                ]
            } else {
                let lt = (t - 0.5) * 2.0;
                [
                    color_mid[0] + (color_tip[0] - color_mid[0]) * lt,
                    color_mid[1] + (color_tip[1] - color_mid[1]) * lt,
                    color_mid[2] + (color_tip[2] - color_mid[2]) * lt,
                ]
            };

            // Normal: blend from upward at base to outward at tip
            // This is key for wind detection in vs_main
            let ny = 0.3 + (1.0 - t) * 0.5;
            let nx = dir.x * (1.0 - ny * ny).sqrt();
            let nz = dir.z * (1.0 - ny * ny).sqrt();
            let n_len = (nx * nx + ny * ny + nz * nz).sqrt().max(0.001);
            let normal = [nx / n_len, ny / n_len, nz / n_len];

            verts.push(Vertex { position: left.to_array(), color, normal });
            verts.push(Vertex { position: right.to_array(), color, normal });
        }

        // Front faces
        for s in 0..SEGMENTS {
            let i = vi_start + (s as u32) * 2;
            idxs.extend_from_slice(&[i, i + 2, i + 1]);
            idxs.extend_from_slice(&[i + 1, i + 2, i + 3]);
        }

        // Back faces
        for s in 0..SEGMENTS {
            let i = vi_start + (s as u32) * 2;
            idxs.extend_from_slice(&[i, i + 1, i + 2]);
            idxs.extend_from_slice(&[i + 1, i + 3, i + 2]);
        }
    }
}

fn lerp_color(a: [f32; 3], b: [f32; 3], blend: f32, variation: f32) -> [f32; 3] {
    [
        (a[0] + (b[0] - a[0]) * blend + variation).clamp(0.10, 0.60),
        (a[1] + (b[1] - a[1]) * blend + variation).clamp(0.12, 0.60),
        (a[2] + (b[2] - a[2]) * blend + variation).clamp(0.08, 0.50),
    ]
}

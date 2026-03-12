use glam::Vec3;
use crate::renderer::Vertex;

const TRUNK_SIDES: usize = 8;

/// Pick a tree type based on seed, altitude, and proximity to water.
/// `near_water` should be true if within a few blocks of sea level.
pub fn generate_tree(base: Vec3, seed: u32, altitude_t: f32) -> (Vec<Vertex>, Vec<u32>) {
    // altitude_t: 0 = near sea level, 1 = treeline
    let near_water = altitude_t < 0.1;
    let high = altitude_t > 0.6;
    let mid = altitude_t > 0.3;
    let grove = altitude_t < 0.3;

    let tree_type = match (seed % 20, high, mid, near_water, grove) {
        // Dead trees — rare everywhere (1 in 20)
        (19, _, _, _, _) => TreeType::Dead,
        // Ancient trees — common in groves (~15%), rare elsewhere (~5%)
        (0..=2, _, _, _, true) => TreeType::Ancient,
        (0, false, false, false, false) => TreeType::Ancient,
        // Near water — willows and alders
        (3..=5, _, _, true, _) => TreeType::Willow,
        (6..=7, _, _, true, _) => TreeType::Oak,
        (_, _, _, true, _) => TreeType::Birch,
        // High altitude — pines, hawthorn, rowan
        (1..=2, true, _, _, _) => TreeType::Hawthorn,
        (3..=4, true, _, _, _) => TreeType::Rowan,
        (5, true, _, _, _) => TreeType::Birch,
        (_, true, _, _, _) => TreeType::Pine,
        // Mid altitude — mixed
        (1..=4, _, true, _, _) => TreeType::Pine,
        (5..=6, _, true, _, _) => TreeType::Rowan,
        (7..=8, _, true, _, _) => TreeType::Birch,
        (9, _, true, _, _) => TreeType::Hawthorn,
        (_, _, true, _, _) => TreeType::Oak,
        // Low altitude — broadleaf dominant
        (1..=4, _, _, _, _) => TreeType::Oak,
        (5..=6, _, _, _, _) => TreeType::Birch,
        (9..=10, _, _, _, _) => TreeType::Rowan,
        (_, _, _, _, _) => TreeType::Pine,
    };

    match tree_type {
        TreeType::Pine => generate_pine(base, seed),
        TreeType::Oak => generate_oak(base, seed),
        TreeType::Birch => generate_birch(base, seed),
        TreeType::Rowan => generate_rowan(base, seed),
        TreeType::Hawthorn => generate_hawthorn(base, seed),
        TreeType::Dead => generate_dead(base, seed),
        TreeType::Willow => generate_willow(base, seed),
        TreeType::Ancient => generate_ancient(base, seed),
    }
}

enum TreeType {
    Pine,
    Oak,
    Birch,
    Rowan,
    Hawthorn,
    Dead,
    Willow,
    Ancient,
}

// ─── Pine: tall, conical, layered ──────────────────────────────

fn generate_pine(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    let trunk_height = 14.0 + (seed % 5) as f32 * 2.0;
    let trunk_r_bot = 0.35 + (seed % 3) as f32 * 0.05;
    let trunk_r_top = 0.12;

    let bark = dark_bark(seed);
    let trunk_top = base + Vec3::Y * trunk_height;

    build_trunk(&mut verts, &mut idxs, base, trunk_top, trunk_r_bot, trunk_r_top, bark);

    // Stacked cones — 3 overlapping layers, widest at bottom
    let needle_color = pine_green(seed);
    let num_layers = 3 + (seed % 2) as usize;
    let canopy_start = trunk_height * 0.35;
    let canopy_end = trunk_height + 3.5 + (seed % 3) as f32 * 1.0;

    for i in 0..num_layers {
        let t = i as f32 / num_layers as f32;
        let layer_y = canopy_start + t * (canopy_end - canopy_start);
        let layer_base = base + Vec3::Y * layer_y;
        let layer_height = (canopy_end - layer_y) * 0.55;
        let layer_tip = layer_base + Vec3::Y * layer_height;
        let layer_radius = (5.0 + (seed % 3) as f32 * 0.8) * (1.0 - t * 0.5);

        let shade = 0.75 + t * 0.25;
        let c = [needle_color[0] * shade, needle_color[1] * shade, needle_color[2] * shade];

        build_organic_cone(&mut verts, &mut idxs, layer_base, layer_tip, layer_radius, c, seed.wrapping_add(i as u32));
    }

    (verts, idxs)
}

// ─── Oak: thick trunk, wide rounded canopy ─────────────────────

fn generate_oak(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    let trunk_height = 8.0 + (seed % 4) as f32 * 1.5;
    let trunk_r_bot = 0.50 + (seed % 3) as f32 * 0.08;
    let trunk_r_top = 0.28 + (seed % 2) as f32 * 0.05;

    let bark = warm_bark(seed);
    let trunk_top = base + Vec3::Y * trunk_height;

    build_trunk(&mut verts, &mut idxs, base, trunk_top, trunk_r_bot, trunk_r_top, bark);

    // Wide, bushy canopy — 3-4 overlapping spheroids
    let leaf_color = oak_green(seed);
    let num_blobs = 3 + (seed % 2) as usize;
    let canopy_center = trunk_top + Vec3::Y * 1.5;

    for i in 0..num_blobs {
        let angle = (i as f32 / num_blobs as f32) * std::f32::consts::TAU
            + (seed % 100) as f32 * 0.06;
        let spread = 2.0 + (seed.wrapping_add(i as u32 * 29) % 4) as f32 * 0.5;
        let offset = Vec3::new(angle.cos() * spread, 0.0, angle.sin() * spread);
        let y_offset = ((seed.wrapping_add(i as u32 * 41)) % 5) as f32 * 0.4 - 0.8;
        let blob_center = canopy_center + offset + Vec3::Y * y_offset;
        let blob_radius = 3.5 + (seed.wrapping_add(i as u32 * 17) % 4) as f32 * 0.5;

        let shade = 0.85 + (i as f32 / num_blobs as f32) * 0.15;
        let c = [leaf_color[0] * shade, leaf_color[1] * shade, leaf_color[2] * shade];

        build_sphere(&mut verts, &mut idxs, blob_center, blob_radius, c, seed.wrapping_add(i as u32));
    }

    // One more blob on top for fullness
    let top_blob = canopy_center + Vec3::Y * 2.5;
    let c = [leaf_color[0] * 1.05, leaf_color[1] * 1.05, leaf_color[2] * 0.95];
    build_sphere(&mut verts, &mut idxs, top_blob, 3.0, c, seed.wrapping_add(99));

    (verts, idxs)
}

// ─── Birch: slender white trunk, delicate canopy ───────────────

fn generate_birch(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    let trunk_height = 12.0 + (seed % 4) as f32 * 1.8;
    let trunk_r_bot = 0.22 + (seed % 3) as f32 * 0.03;
    let trunk_r_top = 0.10;

    // Birch bark — grey-silver, not bright white
    let bark = [
        0.42 + (seed % 4) as f32 * 0.02,
        0.40 + (seed % 3) as f32 * 0.02,
        0.38 + (seed % 5) as f32 * 0.01,
    ];

    // Slight curve
    let lean_x = ((seed % 9) as f32 - 4.0) * 0.06;
    let lean_z = ((seed % 7) as f32 - 3.0) * 0.05;
    let lean = Vec3::new(lean_x, 0.0, lean_z);
    let trunk_top = base + Vec3::Y * trunk_height + lean;

    build_trunk(&mut verts, &mut idxs, base, trunk_top, trunk_r_bot, trunk_r_top, bark);

    // Airy, light canopy — 3-4 sphere clusters
    let leaf_color = birch_green(seed);
    let num_blobs = 3 + (seed % 2) as usize;
    let canopy_base = trunk_top - Vec3::Y * (trunk_height * 0.25);

    for i in 0..num_blobs {
        let t = i as f32 / num_blobs as f32;
        let y = canopy_base.y + t * trunk_height * 0.45;
        let spread = 1.2 + (seed.wrapping_add(i as u32 * 31) % 4) as f32 * 0.4;
        let angle = (i as f32 * 2.4) + (seed % 50) as f32 * 0.1;
        let offset = Vec3::new(angle.cos() * spread, 0.0, angle.sin() * spread);
        let center = Vec3::new(trunk_top.x, y, trunk_top.z) + offset;
        let radius = 2.5 + (seed.wrapping_add(i as u32 * 23) % 3) as f32 * 0.5;

        let shade = 0.9 + t * 0.1;
        let c = [leaf_color[0] * shade, leaf_color[1] * shade, leaf_color[2] * shade];

        build_sphere(&mut verts, &mut idxs, center, radius, c, seed.wrapping_add(i as u32));
    }

    // Top cluster
    let c = [leaf_color[0] * 1.08, leaf_color[1] * 1.08, leaf_color[2] * 0.95];
    build_sphere(&mut verts, &mut idxs, trunk_top + Vec3::Y * 1.5, 2.2, c, seed.wrapping_add(77));

    (verts, idxs)
}

// ─── Rowan: twisted trunk, small clustered canopy ───────────────

fn generate_rowan(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    let trunk_height = 7.0 + (seed % 4) as f32 * 1.0;
    let trunk_r_bot = 0.28 + (seed % 3) as f32 * 0.04;
    let trunk_r_top = 0.14;

    // Reddish-brown bark
    let bark = [
        0.32 + (seed % 4) as f32 * 0.02,
        0.20 + (seed % 3) as f32 * 0.015,
        0.14 + (seed % 3) as f32 * 0.01,
    ];

    // Twisted trunk — leans and curves
    let twist_x = ((seed % 11) as f32 - 5.0) * 0.12;
    let twist_z = ((seed % 7) as f32 - 3.0) * 0.10;
    let trunk_top = base + Vec3::new(twist_x, trunk_height, twist_z);

    build_trunk(&mut verts, &mut idxs, base, trunk_top, trunk_r_bot, trunk_r_top, bark);

    // Canopy clusters
    let leaf_color = rowan_green(seed);
    let canopy_center = trunk_top + Vec3::Y * 0.8;
    let num_blobs = 4 + (seed % 2) as usize;

    for i in 0..num_blobs {
        let angle = (i as f32 / num_blobs as f32) * std::f32::consts::TAU
            + (seed % 80) as f32 * 0.08;
        let spread = 1.2 + (seed.wrapping_add(i as u32 * 23) % 4) as f32 * 0.4;
        let offset = Vec3::new(angle.cos() * spread, 0.0, angle.sin() * spread);
        let y_jitter = ((seed.wrapping_add(i as u32 * 37)) % 5) as f32 * 0.3 - 0.5;
        let blob_center = canopy_center + offset + Vec3::Y * y_jitter;
        let blob_radius = 1.8 + (seed.wrapping_add(i as u32 * 13) % 3) as f32 * 0.4;

        let shade = 0.85 + (i as f32 / num_blobs as f32) * 0.15;
        let c = [leaf_color[0] * shade, leaf_color[1] * shade, leaf_color[2] * shade];
        build_sphere(&mut verts, &mut idxs, blob_center, blob_radius, c, seed.wrapping_add(i as u32));
    }

    // Occasional berry clusters
    if seed % 3 == 0 {
        let berry_color = [0.50, 0.22, 0.18];
        let berry_pos = canopy_center + Vec3::new(
            ((seed % 5) as f32 - 2.0) * 0.7,
            -0.5,
            ((seed % 7) as f32 - 3.0) * 0.5,
        );
        build_sphere(&mut verts, &mut idxs, berry_pos, 0.5, berry_color, seed.wrapping_add(200));
    }

    (verts, idxs)
}

// ─── Hawthorn: short, wide, wind-sculpted ───────────────────────

fn generate_hawthorn(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    let trunk_height = 4.0 + (seed % 4) as f32 * 0.8;
    let trunk_r_bot = 0.22 + (seed % 3) as f32 * 0.04;
    let trunk_r_top = 0.14;

    let bark = dark_bark(seed);

    // Wind-bent — consistent lean direction based on seed
    let wind_lean = 0.4 + (seed % 5) as f32 * 0.1;
    let wind_angle = (seed % 8) as f32 * std::f32::consts::TAU / 8.0;
    let lean = Vec3::new(wind_angle.cos() * wind_lean, 0.0, wind_angle.sin() * wind_lean);
    let trunk_top = base + Vec3::Y * trunk_height + lean;

    build_trunk(&mut verts, &mut idxs, base, trunk_top, trunk_r_bot, trunk_r_top, bark);

    // 1-2 secondary trunks splitting low
    let num_forks = 1 + (seed % 2) as usize;
    for f in 0..num_forks {
        let fork_angle = wind_angle + std::f32::consts::PI * 0.4 * (f as f32 - 0.5);
        let fork_lean = Vec3::new(
            fork_angle.cos() * (wind_lean + 0.25),
            0.0,
            fork_angle.sin() * (wind_lean + 0.25),
        );
        let fork_base = base + Vec3::Y * (trunk_height * 0.3);
        let fork_top = fork_base + Vec3::Y * (trunk_height * 0.6) + fork_lean;
        build_trunk(&mut verts, &mut idxs, fork_base, fork_top, trunk_r_bot * 0.7, trunk_r_top * 0.8, bark);
    }

    // Wide, flat canopy — spreading laterally more than vertically
    let leaf_color = hawthorn_green(seed);
    let canopy_center = trunk_top + Vec3::Y * 0.5;
    let num_blobs = 5 + (seed % 3) as usize;

    for i in 0..num_blobs {
        let angle = (i as f32 / num_blobs as f32) * std::f32::consts::TAU
            + (seed % 60) as f32 * 0.1;
        let spread = 2.0 + (seed.wrapping_add(i as u32 * 19) % 5) as f32 * 0.5;
        let wind_bias = lean.normalize_or_zero() * 0.8;
        let offset = Vec3::new(angle.cos() * spread, 0.0, angle.sin() * spread) + wind_bias;
        let y_jitter = ((seed.wrapping_add(i as u32 * 41)) % 4) as f32 * 0.2 - 0.4;
        let blob_center = canopy_center + offset + Vec3::Y * y_jitter;
        let blob_radius = 2.0 + (seed.wrapping_add(i as u32 * 11) % 3) as f32 * 0.4;

        let shade = 0.82 + (i as f32 / num_blobs as f32) * 0.18;
        let c = [leaf_color[0] * shade, leaf_color[1] * shade, leaf_color[2] * shade];
        build_sphere(&mut verts, &mut idxs, blob_center, blob_radius, c, seed.wrapping_add(i as u32));
    }

    (verts, idxs)
}

// ─── Dead tree: bare trunk with branches, no foliage ────────────

fn generate_dead(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    let trunk_height = 10.0 + (seed % 5) as f32 * 1.5;
    let trunk_r_bot = 0.32 + (seed % 3) as f32 * 0.05;
    let trunk_r_top = 0.10 + (seed % 2) as f32 * 0.03;

    // Bleached grey bark
    let bark = dead_bark(seed);

    // Slight lean
    let lean_x = ((seed % 9) as f32 - 4.0) * 0.08;
    let lean_z = ((seed % 7) as f32 - 3.0) * 0.06;
    let trunk_top = base + Vec3::new(lean_x, trunk_height, lean_z);

    build_trunk(&mut verts, &mut idxs, base, trunk_top, trunk_r_bot, trunk_r_top, bark);

    // 2-4 bare branches sticking out at angles
    let num_branches = 2 + (seed % 3) as usize;
    for b in 0..num_branches {
        let branch_seed = seed.wrapping_add(b as u32 * 53);
        let branch_y = trunk_height * (0.4 + (b as f32 / num_branches as f32) * 0.5);
        let branch_angle = (b as f32 / num_branches as f32) * std::f32::consts::TAU
            + (seed % 50) as f32 * 0.12;

        let branch_base_pos = base + Vec3::Y * branch_y
            + Vec3::new(lean_x * branch_y / trunk_height, 0.0, lean_z * branch_y / trunk_height);

        let reach = 3.0 + (branch_seed % 4) as f32 * 0.8;
        let rise = 0.8 + (branch_seed % 3) as f32 * 0.5;
        let branch_end = branch_base_pos + Vec3::new(
            branch_angle.cos() * reach,
            rise,
            branch_angle.sin() * reach,
        );

        let branch_r = trunk_r_top * (1.5 - b as f32 * 0.2);
        build_trunk(&mut verts, &mut idxs, branch_base_pos, branch_end, branch_r, branch_r * 0.3, bark);

        // Occasional sub-branch
        if branch_seed % 3 == 0 {
            let sub_angle = branch_angle + ((branch_seed % 5) as f32 - 2.0) * 0.4;
            let sub_reach = reach * 0.5;
            let sub_end = branch_end + Vec3::new(
                sub_angle.cos() * sub_reach,
                0.4,
                sub_angle.sin() * sub_reach,
            );
            build_trunk(&mut verts, &mut idxs, branch_end, sub_end, branch_r * 0.4, 0.04, bark);
        }
    }

    // Broken top
    let stump_top = trunk_top + Vec3::Y * 0.3;
    let stump_r = trunk_r_top * 1.3;
    build_trunk(&mut verts, &mut idxs, trunk_top, stump_top, trunk_r_top, stump_r * 0.5, bark);

    (verts, idxs)
}

// ─── Willow: drooping branches from a spreading canopy ──────────

fn generate_willow(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    let trunk_height = 9.0 + (seed % 4) as f32 * 1.5;
    let trunk_r_bot = 0.45 + (seed % 3) as f32 * 0.06;
    let trunk_r_top = 0.22;

    let bark = warm_bark(seed);
    let trunk_top = base + Vec3::Y * trunk_height;

    build_trunk(&mut verts, &mut idxs, base, trunk_top, trunk_r_bot, trunk_r_top, bark);

    // Central canopy mass
    let leaf_color = willow_green(seed);
    let canopy_center = trunk_top + Vec3::Y * 1.5;
    for i in 0..4 {
        let angle = (i as f32 / 4.0) * std::f32::consts::TAU + (seed % 40) as f32 * 0.09;
        let spread = 1.2 + (seed.wrapping_add(i as u32 * 19) % 3) as f32 * 0.4;
        let offset = Vec3::new(angle.cos() * spread, 0.0, angle.sin() * spread);
        let c = [leaf_color[0] * 0.9, leaf_color[1] * 0.9, leaf_color[2] * 0.85];
        build_sphere(&mut verts, &mut idxs, canopy_center + offset, 2.5, c, seed.wrapping_add(i as u32));
    }

    // Drooping branch curtains — inverted cones hanging from canopy edge
    let num_drapes = 8 + (seed % 3) as usize;
    let drape_ring_radius = 3.5 + (seed % 3) as f32 * 0.5;

    for d in 0..num_drapes {
        let drape_seed = seed.wrapping_add(d as u32 * 67);
        let angle = (d as f32 / num_drapes as f32) * std::f32::consts::TAU
            + (seed % 30) as f32 * 0.2;

        let drape_top = canopy_center + Vec3::new(
            angle.cos() * drape_ring_radius,
            -0.5,
            angle.sin() * drape_ring_radius,
        );

        // Drape hangs down 4-6 blocks
        let drape_length = 4.0 + (drape_seed % 5) as f32 * 0.5;
        let drape_bottom = drape_top - Vec3::Y * drape_length;
        let drape_radius = 0.7 + (drape_seed % 3) as f32 * 0.15;

        let shade = 0.85 + (d as f32 / num_drapes as f32) * 0.15;
        let c = [leaf_color[0] * shade, leaf_color[1] * shade * 1.05, leaf_color[2] * shade * 0.9];

        build_organic_cone(&mut verts, &mut idxs, drape_bottom, drape_top, drape_radius, c, drape_seed);
    }

    (verts, idxs)
}

// ─── Ancient: Fangorn-style massive old-growth trees ─────────────

fn generate_ancient(base: Vec3, seed: u32) -> (Vec<Vertex>, Vec<u32>) {
    let mut verts = Vec::new();
    let mut idxs = Vec::new();

    let trunk_height = 16.0 + (seed % 9) as f32 * 1.0;
    let trunk_r_bot = 0.80 + (seed % 5) as f32 * 0.08;
    let trunk_r_top = 0.50 + (seed % 3) as f32 * 0.07;

    // Very dark ancient bark
    let bark = ancient_bark(seed);
    let trunk_top = base + Vec3::Y * trunk_height;

    build_trunk(&mut verts, &mut idxs, base, trunk_top, trunk_r_bot, trunk_r_top, bark);

    // ─── Moss patches on trunk ────────────────────────────────
    // Slightly larger geometry over the trunk with mossy color
    let moss_color = [
        0.18 + (seed % 3) as f32 * 0.01,
        0.24 + (seed % 4) as f32 * 0.01,
        0.16 + (seed % 3) as f32 * 0.008,
    ];
    let moss_offset = 0.04;
    build_trunk(
        &mut verts, &mut idxs,
        base + Vec3::Y * 0.5,
        base + Vec3::Y * (trunk_height * 0.6),
        trunk_r_bot + moss_offset,
        trunk_r_top + moss_offset * 0.5,
        moss_color,
    );

    // ─── Exposed roots — thick tendrils extending from base ───
    let num_roots = 3 + (seed % 3) as usize;
    for r in 0..num_roots {
        let root_seed = seed.wrapping_add(r as u32 * 97);
        let root_angle = (r as f32 / num_roots as f32) * std::f32::consts::TAU
            + (seed % 60) as f32 * 0.1;

        let root_start = base + Vec3::Y * 1.5;
        let root_reach = 2.0 + (root_seed % 5) as f32 * 0.5;
        let root_end = base + Vec3::new(
            root_angle.cos() * root_reach,
            -0.3,
            root_angle.sin() * root_reach,
        );

        build_trunk(&mut verts, &mut idxs, root_start, root_end, 0.25, 0.08, bark);
    }

    // ─── Low branches — thick horizontal limbs ────────────────
    let num_branches = 2 + (seed % 2) as usize;
    for b in 0..num_branches {
        let branch_seed = seed.wrapping_add(b as u32 * 131);
        let branch_height = trunk_height * (0.4 + (b as f32 / num_branches as f32) * 0.2);
        let branch_angle = (b as f32 / num_branches as f32) * std::f32::consts::TAU
            + (seed % 40) as f32 * 0.15;

        let branch_base_pos = base + Vec3::Y * branch_height;
        let branch_reach = 3.0 + (branch_seed % 5) as f32 * 0.5;
        let branch_end = branch_base_pos + Vec3::new(
            branch_angle.cos() * branch_reach,
            1.0 + (branch_seed % 3) as f32 * 0.3,
            branch_angle.sin() * branch_reach,
        );

        let branch_r = 0.18 + (branch_seed % 3) as f32 * 0.04;
        build_trunk(&mut verts, &mut idxs, branch_base_pos, branch_end, branch_r, branch_r * 0.4, bark);

        // Small canopy blob at end of each branch
        let leaf_color = ancient_green(seed);
        let blob_radius = 2.5 + (branch_seed % 3) as f32 * 0.5;
        let c = [leaf_color[0] * 0.9, leaf_color[1] * 0.9, leaf_color[2] * 0.85];
        build_sphere(&mut verts, &mut idxs, branch_end + Vec3::Y * 0.5, blob_radius, c, branch_seed);
    }

    // ─── Massive canopy — 5-7 large overlapping sphere blobs ──
    let leaf_color = ancient_green(seed);
    let num_blobs = 5 + (seed % 3) as usize;
    let canopy_center = trunk_top + Vec3::Y * 2.0;

    for i in 0..num_blobs {
        let angle = (i as f32 / num_blobs as f32) * std::f32::consts::TAU
            + (seed % 100) as f32 * 0.06;
        let spread = 3.0 + (seed.wrapping_add(i as u32 * 37) % 5) as f32 * 0.6;
        let offset = Vec3::new(angle.cos() * spread, 0.0, angle.sin() * spread);
        let y_offset = ((seed.wrapping_add(i as u32 * 43)) % 7) as f32 * 0.5 - 1.5;
        let blob_center = canopy_center + offset + Vec3::Y * y_offset;
        let blob_radius = 4.5 + (seed.wrapping_add(i as u32 * 19) % 4) as f32 * 0.4;

        let shade = 0.80 + (i as f32 / num_blobs as f32) * 0.20;
        let c = [leaf_color[0] * shade, leaf_color[1] * shade, leaf_color[2] * shade];

        build_sphere(&mut verts, &mut idxs, blob_center, blob_radius, c, seed.wrapping_add(i as u32));
    }

    // Top crown blob
    let c = [leaf_color[0] * 0.95, leaf_color[1] * 0.95, leaf_color[2] * 0.90];
    build_sphere(&mut verts, &mut idxs, canopy_center + Vec3::Y * 3.0, 4.0, c, seed.wrapping_add(999));

    (verts, idxs)
}

// ─── Color palettes — dark fantasy (LOTR / Fangorn) ─────────────

fn dark_bark(seed: u32) -> [f32; 3] {
    [
        0.14 + (seed % 4) as f32 * 0.015,
        0.10 + (seed % 3) as f32 * 0.01,
        0.06 + (seed % 5) as f32 * 0.008,
    ]
}

fn warm_bark(seed: u32) -> [f32; 3] {
    [
        0.18 + (seed % 4) as f32 * 0.02,
        0.14 + (seed % 3) as f32 * 0.015,
        0.08 + (seed % 3) as f32 * 0.01,
    ]
}

fn ancient_bark(seed: u32) -> [f32; 3] {
    [
        0.10 + (seed % 4) as f32 * 0.012,
        0.08 + (seed % 3) as f32 * 0.008,
        0.05 + (seed % 5) as f32 * 0.006,
    ]
}

fn pine_green(seed: u32) -> [f32; 3] {
    [
        0.10 + (seed % 4) as f32 * 0.015,
        0.18 + (seed % 5) as f32 * 0.02,
        0.08 + (seed % 3) as f32 * 0.01,
    ]
}

fn oak_green(seed: u32) -> [f32; 3] {
    [
        0.14 + (seed % 5) as f32 * 0.02,
        0.22 + (seed % 4) as f32 * 0.025,
        0.10 + (seed % 3) as f32 * 0.012,
    ]
}

fn birch_green(seed: u32) -> [f32; 3] {
    [
        0.18 + (seed % 4) as f32 * 0.02,
        0.28 + (seed % 5) as f32 * 0.02,
        0.12 + (seed % 3) as f32 * 0.015,
    ]
}

fn rowan_green(seed: u32) -> [f32; 3] {
    [
        0.14 + (seed % 4) as f32 * 0.02,
        0.22 + (seed % 5) as f32 * 0.02,
        0.10 + (seed % 3) as f32 * 0.01,
    ]
}

fn hawthorn_green(seed: u32) -> [f32; 3] {
    [
        0.16 + (seed % 4) as f32 * 0.015,
        0.22 + (seed % 5) as f32 * 0.02,
        0.12 + (seed % 3) as f32 * 0.012,
    ]
}

fn dead_bark(seed: u32) -> [f32; 3] {
    // Grey-blue instead of warm
    [
        0.28 + (seed % 4) as f32 * 0.015,
        0.27 + (seed % 3) as f32 * 0.012,
        0.30 + (seed % 5) as f32 * 0.01,
    ]
}

fn willow_green(seed: u32) -> [f32; 3] {
    [
        0.18 + (seed % 4) as f32 * 0.02,
        0.28 + (seed % 5) as f32 * 0.025,
        0.14 + (seed % 3) as f32 * 0.015,
    ]
}

fn ancient_green(seed: u32) -> [f32; 3] {
    // Very dark, deep forest green
    [
        0.12 + (seed % 4) as f32 * 0.015,
        0.20 + (seed % 5) as f32 * 0.02,
        0.10 + (seed % 3) as f32 * 0.01,
    ]
}

// ─── Geometry builders ─────────────────────────────────────────

fn build_trunk(
    verts: &mut Vec<Vertex>,
    idxs: &mut Vec<u32>,
    bottom: Vec3,
    top: Vec3,
    r_bot: f32,
    r_top: f32,
    color: [f32; 3],
) {
    let sides = TRUNK_SIDES;
    let axis = (top - bottom).normalize();
    let up = if axis.y.abs() > 0.99 { Vec3::X } else { Vec3::Y };
    let right = axis.cross(up).normalize();
    let forward = axis.cross(right).normalize();

    for i in 0..sides {
        let a0 = (i as f32 / sides as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / sides as f32) * std::f32::consts::TAU;

        let d0 = right * a0.cos() + forward * a0.sin();
        let d1 = right * a1.cos() + forward * a1.sin();

        let n = (d0 + d1).normalize().to_array();
        let vi = verts.len() as u32;

        verts.push(Vertex { position: (bottom + d0 * r_bot).to_array(), color, normal: n });
        verts.push(Vertex { position: (bottom + d1 * r_bot).to_array(), color, normal: n });
        verts.push(Vertex { position: (top + d1 * r_top).to_array(), color, normal: n });
        verts.push(Vertex { position: (top + d0 * r_top).to_array(), color, normal: n });

        idxs.extend_from_slice(&[vi, vi + 1, vi + 2, vi, vi + 2, vi + 3]);
    }
}

/// Organic cone with irregular base ring — good for pine canopy layers.
fn build_organic_cone(
    verts: &mut Vec<Vertex>,
    idxs: &mut Vec<u32>,
    base: Vec3,
    tip: Vec3,
    radius: f32,
    color: [f32; 3],
    seed: u32,
) {
    let sides = 10usize;
    let axis = (tip - base).normalize();
    let up = if axis.y.abs() > 0.99 { Vec3::X } else { Vec3::Y };
    let right = axis.cross(up).normalize();
    let forward = axis.cross(right).normalize();

    let tip_vi = verts.len() as u32;
    verts.push(Vertex { position: tip.to_array(), color, normal: axis.to_array() });

    // Irregular base ring
    let mut ring_positions = Vec::with_capacity(sides);
    for i in 0..sides {
        let angle = (i as f32 / sides as f32) * std::f32::consts::TAU;
        let var = 0.8 + ((seed.wrapping_mul(i as u32 + 1).wrapping_mul(2654435761)) % 100) as f32 * 0.004;
        let r = radius * var;
        let dir = right * angle.cos() + forward * angle.sin();
        ring_positions.push(base + dir * r);
    }

    // Side faces
    for i in 0..sides {
        let next = (i + 1) % sides;
        let p0 = ring_positions[i];
        let p1 = ring_positions[next];

        let edge0 = tip.to_array();
        let n = ((p0 - base).normalize() + (p1 - base).normalize() + axis * 0.4).normalize();

        let vi = verts.len() as u32;
        verts.push(Vertex { position: p0.to_array(), color, normal: n.to_array() });
        verts.push(Vertex { position: p1.to_array(), color, normal: n.to_array() });
        let _ = edge0;

        idxs.extend_from_slice(&[tip_vi, vi, vi + 1]);
    }

    // Darker underside
    let dark = [color[0] * 0.55, color[1] * 0.55, color[2] * 0.55];
    let bot_vi = verts.len() as u32;
    let bot_n = (-axis).to_array();
    verts.push(Vertex { position: base.to_array(), color: dark, normal: bot_n });
    for pos in &ring_positions {
        verts.push(Vertex { position: pos.to_array(), color: dark, normal: bot_n });
    }
    for i in 0..sides {
        let next = (i + 1) % sides;
        idxs.extend_from_slice(&[bot_vi, bot_vi + 1 + next as u32, bot_vi + 1 + i as u32]);
    }
}

/// Faceted sphere — icosphere-ish, good for broadleaf canopy blobs.
fn build_sphere(
    verts: &mut Vec<Vertex>,
    idxs: &mut Vec<u32>,
    center: Vec3,
    radius: f32,
    color: [f32; 3],
    seed: u32,
) {
    let lat_steps = 5usize;
    let lon_steps = 8usize;
    let base_vi = verts.len() as u32;

    // Generate vertices on a UV sphere with slight irregularity
    for lat in 0..=lat_steps {
        let theta = (lat as f32 / lat_steps as f32) * std::f32::consts::PI;
        for lon in 0..=lon_steps {
            let phi = (lon as f32 / lon_steps as f32) * std::f32::consts::TAU;

            let nx = theta.sin() * phi.cos();
            let ny = theta.cos();
            let nz = theta.sin() * phi.sin();
            let normal = Vec3::new(nx, ny, nz);

            // Irregularity — bump the radius slightly per vertex
            let var_seed = seed.wrapping_add(lat as u32 * 17 + lon as u32 * 31);
            let var = 0.88 + (var_seed.wrapping_mul(2654435761) % 100) as f32 * 0.0024;
            let r = radius * var;

            let pos = center + normal * r;

            // Darken underside slightly
            let shade = 0.7 + ny * 0.15 + 0.15;
            let c = [color[0] * shade, color[1] * shade, color[2] * shade];

            verts.push(Vertex {
                position: pos.to_array(),
                color: c,
                normal: normal.to_array(),
            });
        }
    }

    // Generate indices
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
}

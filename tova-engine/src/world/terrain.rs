use noise::{NoiseFn, Perlin};

use crate::voxel::block::Block;
use crate::voxel::chunk::{Chunk, CHUNK_SIZE, SEA_LEVEL, WORLD_HEIGHT};

const SEED: u32 = 42;

// ─── Grove Hills zone ────────────────────────────────────────
// The player's spawn zone: a forested valley ringed by hills,
// with towering mountain peaks rising in the distance.
const GROVE_CENTER_X: f64 = 64.0;
const GROVE_CENTER_Z: f64 = 0.0;

pub struct TerrainGen {
    continent: Perlin,
    detail: Perlin,
    fine: Perlin,
    biome: Perlin,
    ridge: Perlin,
    cliff: Perlin,
    ravine: Perlin,
    ravine_warp: Perlin,
    peak: Perlin,
    /// Coastal shelf noise — controls where dramatic sea cliffs form
    coastal: Perlin,
    /// River centerline noise — zero-crossings define river paths
    river_path: Perlin,
    /// River warp — organic meandering curves
    river_warp: Perlin,
    /// River width variation
    river_width: Perlin,
    /// Lake basins — high values define lake locations
    lake_noise: Perlin,
}

impl TerrainGen {
    pub fn new() -> Self {
        Self {
            continent: Perlin::new(SEED),
            detail: Perlin::new(SEED + 1),
            fine: Perlin::new(SEED + 2),
            biome: Perlin::new(SEED + 3),
            ridge: Perlin::new(SEED + 4),
            cliff: Perlin::new(SEED + 5),
            ravine: Perlin::new(SEED + 6),
            ravine_warp: Perlin::new(SEED + 7),
            peak: Perlin::new(SEED + 8),
            coastal: Perlin::new(SEED + 9),
            river_path: Perlin::new(SEED + 10),
            river_warp: Perlin::new(SEED + 11),
            river_width: Perlin::new(SEED + 12),
            lake_noise: Perlin::new(SEED + 13),
        }
    }

    fn fbm(&self, noise: &Perlin, x: f64, z: f64, octaves: u32, lacunarity: f64, gain: f64) -> f64 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let mut max_amp = 0.0;

        for _ in 0..octaves {
            sum += noise.get([x * freq, z * freq]) * amp;
            max_amp += amp;
            amp *= gain;
            freq *= lacunarity;
        }

        sum / max_amp
    }

    fn ridged(&self, x: f64, z: f64, octaves: u32) -> f64 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let gain = 0.5;
        let lacunarity = 2.0;

        for _ in 0..octaves {
            let v = self.ridge.get([x * freq, z * freq]);
            let ridged = 1.0 - v.abs();
            sum += ridged * ridged * amp;
            amp *= gain;
            freq *= lacunarity;
        }

        sum
    }

    fn biome_at(&self, wx: f64, wz: f64) -> f64 {
        self.fbm(&self.biome, wx * 0.001, wz * 0.001, 3, 2.0, 0.5)
    }

    /// Terrace with large, variable step sizes for dramatic cliff bands.
    /// Designed for bold moorland plateaus and highland shelves.
    fn terrace(&self, height: f64, wx: f64, wz: f64) -> f64 {
        // Large step sizes: 10-20 blocks — bold highland plateaus
        let step_var = self.cliff.get([wx * 0.0015 + 50.0, wz * 0.0015 + 50.0]);
        let step_size = 14.0 + step_var * 5.0; // 9..19 range

        let k = (height / step_size).floor();
        let f = height / step_size - k;

        // Rough, irregular cliff edges — not ruler-straight
        let edge_noise = self.fine.get([wx * 0.04, wz * 0.04]) * 0.08;
        let cliff_start = 0.08 + edge_noise; // mostly flat
        let cliff_end = cliff_start + 0.12;  // short, sheer transition

        let t = if f < cliff_start {
            0.0
        } else if f < cliff_end {
            // Near-vertical: very steep smoothstep
            let s = (f - cliff_start) / (cliff_end - cliff_start);
            s * s * (3.0 - 2.0 * s)
        } else {
            1.0
        };
        (k + t) * step_size
    }

    /// Coastal cliff function. Returns a height boost that creates dramatic
    /// sea cliffs — terrain stays elevated then drops sheer to the water.
    fn coastal_shelf(&self, raw_height: f64, wx: f64, wz: f64) -> f64 {
        let base = SEA_LEVEL as f64;

        // Only affect terrain near sea level
        let proximity = raw_height - base;
        if proximity > 18.0 || proximity < -5.0 {
            return raw_height;
        }

        // Coastal noise determines where cliffs vs beaches form
        let coast_val = self.fbm(&self.coastal, wx * 0.004, wz * 0.004, 3, 2.0, 0.5);

        // Where coast noise is positive: create sea cliffs
        if coast_val > -0.1 {
            let cliff_strength = ((coast_val + 0.1) * 2.5).min(1.0);

            // Boost terrain to create a shelf, then let it drop sheer
            // Terrain that would gently slope to sea level instead stays up
            if proximity > 0.0 && proximity < 15.0 {
                let boost = (15.0 - proximity) * 0.7 * cliff_strength;
                return raw_height + boost;
            }
            // Terrain just below sea level: push it further down for sheer drop
            if proximity >= -5.0 && proximity <= 0.0 {
                let drop = (5.0 + proximity) * 2.0 * cliff_strength;
                return raw_height - drop;
            }
        }

        raw_height
    }

    fn ravine_at(&self, wx: f64, wz: f64, terrain_height: f64) -> (f64, f64) {
        let warp_x = self.ravine_warp.get([wx * 0.005, wz * 0.005]) * 24.0;
        let warp_z = self.ravine_warp.get([wx * 0.005 + 100.0, wz * 0.005 + 100.0]) * 24.0;

        let warped_x = wx + warp_x;
        let warped_z = wz + warp_z;

        let ravine_val = self.ravine.get([warped_x * 0.008, warped_z * 0.008]);
        let dist = ravine_val.abs();

        let width_mod = self.ravine.get([warped_x * 0.003 + 200.0, warped_z * 0.003 + 200.0]);
        let threshold = 0.03 + (width_mod * 0.5 + 0.5) * 0.04;

        if dist > threshold {
            return (0.0, 0.0);
        }

        let t = 1.0 - (dist / threshold);
        let t_depth = t * t;
        let width_factor = t;

        let max_depth = (terrain_height - SEA_LEVEL as f64 + 8.0).max(0.0).min(35.0);
        (max_depth * t_depth, width_factor)
    }

    /// River detection — noise iso-lines define winding river paths.
    /// Returns (carve_depth, width_factor, water_level) or zeros if no river.
    fn river_at(&self, wx: f64, wz: f64, terrain_height: f64) -> (f64, f64, f64) {
        // Skip near ocean — rivers merge naturally
        let altitude = terrain_height - SEA_LEVEL as f64;
        if altitude < 3.0 {
            return (0.0, 0.0, 0.0);
        }

        // Warp for organic meandering
        let warp_x = self.river_warp.get([wx * 0.003, wz * 0.003]) * 40.0;
        let warp_z = self.river_warp.get([wx * 0.003 + 100.0, wz * 0.003 + 100.0]) * 40.0;
        let warped_x = wx + warp_x;
        let warped_z = wz + warp_z;

        // River centerline: zero-crossing of noise
        let river_val = self.river_path.get([warped_x * 0.004, warped_z * 0.004]);
        let dist = river_val.abs();

        // Variable width: narrower in highlands, wider in lowlands
        let altitude_t = (altitude / 80.0).clamp(0.0, 1.0);
        let base_width = 0.025 + (1.0 - altitude_t) * 0.035;
        let width_noise = self.river_width.get([warped_x * 0.002, warped_z * 0.002]);
        let threshold = base_width + width_noise.abs() * 0.012;

        if dist > threshold {
            return (0.0, 0.0, 0.0);
        }

        let t = 1.0 - (dist / threshold);
        // V-shaped channel: deeper at center, shallower at edges
        let max_carve = altitude.min(6.0).max(0.0);
        let carve = max_carve * t * t;
        let water_level = (terrain_height - 2.0).max(SEA_LEVEL as f64);

        (carve, t, water_level)
    }

    /// Lake detection — noise basins define inland lake locations.
    /// Returns Some(water_level) if this column is in a lake, None otherwise.
    fn lake_at(&self, wx: f64, wz: f64, terrain_height: f64) -> Option<usize> {
        // Lake basin noise — low frequency for large features
        let n = self.lake_noise.get([wx * 0.006, wz * 0.006]);
        if n < 0.55 {
            // Special case: grove valley lake
            let dx = wx - GROVE_CENTER_X;
            let dz = wz - GROVE_CENTER_Z;
            let grove_dist = (dx * dx + dz * dz).sqrt();
            // Small tarn in the grove
            let tarn_noise = self.lake_noise.get([wx * 0.02, wz * 0.02]);
            if grove_dist < 18.0 && tarn_noise > 0.1 {
                let water_level = SEA_LEVEL + 15;
                if (terrain_height as usize) < water_level {
                    return Some(water_level);
                }
            }
            return None;
        }

        // Lake water level: smooth, low-freq noise so it's consistent across the lake
        let level_noise = self.lake_noise.get([wx * 0.001 + 300.0, wz * 0.001 + 300.0]);
        let water_level = (SEA_LEVEL as f64 + 12.0 + level_noise * 25.0) as usize;

        // Only fill if terrain is below water level (natural basin)
        if (terrain_height as usize) < water_level && water_level < WORLD_HEIGHT {
            Some(water_level)
        } else {
            None
        }
    }

    /// Broad highland peaks — wide, rounded summits. Think Ben Nevis, not Matterhorn.
    fn peak_height(&self, wx: f64, wz: f64) -> f64 {
        // Very low frequency = massive, wide dome shapes
        let n = self.peak.get([wx * 0.0008, wz * 0.0008]);
        let t = ((n - 0.15) * 1.5).max(0.0).min(1.0);
        // Linear ramp with soft top — broad plateau-like summits
        let s = t * t * (3.0 - 2.0 * t); // smoothstep for gentle dome
        s
    }

    /// Grove Hills zone influence — sculpts terrain into a designed landscape.
    /// Returns a modified height that blends procedural terrain with the zone layout:
    ///   - Center: forested valley (lowered, flat)
    ///   - Ring ~50-90 blocks: rolling hills (spawn viewpoint)
    ///   - Beyond ~100: mountains amplified to towering peaks
    fn grove_influence(&self, wx: f64, wz: f64, raw_height: f64) -> f64 {
        let dx = wx - GROVE_CENTER_X;
        let dz = wz - GROVE_CENTER_Z;
        let dist = (dx * dx + dz * dz).sqrt();

        // Add noise to distance for organic, non-circular boundaries
        let boundary_warp = self.detail.get([wx * 0.012, wz * 0.012]) * 15.0;
        let dist = dist + boundary_warp;

        let base = SEA_LEVEL as f64;

        if dist < 45.0 {
            // Grove valley — gentle, lowered forest floor
            let t = dist / 45.0;
            let valley_floor = base + 14.0 + self.fine.get([wx * 0.02, wz * 0.02]) * 3.0;
            let blend = t * t;
            valley_floor * (1.0 - blend) + raw_height * blend
        } else if dist < 75.0 {
            // Hill ring — spawn viewpoint, gentle rolling hills
            let t = (dist - 45.0) / 30.0;
            let hill_boost = (1.0 - (t * 2.0 - 1.0).powi(2)) * 12.0;
            raw_height + hill_boost
        } else if dist < 120.0 {
            // Ramp — terrain rises steeply toward the mountains
            let t = (dist - 75.0) / 45.0;
            let rise = t * t * 50.0;
            raw_height + rise
        } else {
            // Mountain amplification — towering peaks visible from the grove
            let height_above = (raw_height - base).max(0.0);
            raw_height + 50.0 + height_above * 1.0
        }
    }

    /// Estimate terrain steepness by sampling neighbors.
    fn steepness(&self, wx: f64, wz: f64) -> f64 {
        let d = 1.5;
        let (h_c, _) = self.sample_raw(wx, wz);
        let (h_e, _) = self.sample_raw(wx + d, wz);
        let (h_w, _) = self.sample_raw(wx - d, wz);
        let (h_n, _) = self.sample_raw(wx, wz - d);
        let (h_s, _) = self.sample_raw(wx, wz + d);

        let dx = ((h_e - h_w) / (2.0 * d)).abs();
        let dz = ((h_s - h_n) / (2.0 * d)).abs();
        let slope = (dx * dx + dz * dz).sqrt();

        // Normalize: slope 2.5 = fully steep
        (slope / 2.5).min(1.0)
        // Allow cliffs even near sea level for coastal drama
        * (((h_c - SEA_LEVEL as f64 + 3.0) / 6.0).clamp(0.0, 1.0))
    }

    /// Raw height sample before integer clamping.
    /// Public alias for LOD terrain generation.
    pub fn sample_raw_pub(&self, wx: f64, wz: f64) -> (f64, f64) {
        self.sample_raw(wx, wz)
    }

    fn sample_raw(&self, wx: f64, wz: f64) -> (f64, f64) {
        let biome_val = self.biome_at(wx, wz);

        let continent = self.fbm(&self.continent, wx * 0.002, wz * 0.002, 4, 2.0, 0.45) * 0.5 + 0.5;
        let detail = self.fbm(&self.detail, wx * 0.008, wz * 0.008, 4, 2.0, 0.5);
        let fine = self.fbm(&self.fine, wx * 0.03, wz * 0.03, 2, 2.0, 0.5);
        let ridge = self.ridged(wx * 0.002, wz * 0.002, 3);
        let cliff_val = self.fbm(&self.cliff, wx * 0.006, wz * 0.006, 3, 2.0, 0.5);

        let base = SEA_LEVEL as f64;

        // --- Height by biome ---
        // Overall: terrain is elevated. Even "plains" sit well above sea level.
        // This creates the highland moorland feel — you're always up on a shelf.
        let raw_height = if biome_val < -0.2 {
            // Moorland plains — elevated flat-ish terrain, not valley floors
            let flatness = ((-0.2 - biome_val) * 2.0).min(1.0);
            let moor_h = continent * 16.0 + detail * 5.0 + fine * 1.5;
            base + moor_h * (1.0 - flatness * 0.5) + 8.0
        } else if biome_val < 0.3 {
            // Rolling highlands — dramatic undulations with cliff edges
            let t = (biome_val + 0.2) / 0.5;
            let hill_h = continent * 22.0 + detail * 14.0 * t + fine * 2.5;
            base + hill_h + 10.0
        } else {
            // Highland mountains — broad shoulders with dramatic cliff edges
            // Increased multipliers for WORLD_HEIGHT=256 — truly towering peaks
            let t = ((biome_val - 0.3) * 2.0).min(1.0);
            let mountain_h = continent * 45.0 + ridge * 40.0 * t + detail * 12.0 + fine * 3.0;
            let peak = self.peak_height(wx, wz);
            let peak_bonus = peak * 45.0 * t;
            base + mountain_h + 18.0 + peak_bonus
        };

        // --- Grove Hills zone shaping ---
        let raw_height = self.grove_influence(wx, wz, raw_height);

        // --- Cliff terracing: active across ALL biomes ---
        // This is what makes it look like Ireland/Scotland — everywhere has
        // the potential for dramatic cliff bands and plateau edges.
        let terraced_height = if cliff_val > -0.15 {
            // Very broad activation — most terrain gets some terracing
            let cliff_strength = ((cliff_val + 0.15) * 2.0).min(1.0);
            // Weaker in plains, stronger in highlands
            let biome_boost = ((biome_val + 0.3) * 1.2).clamp(0.3, 1.0);
            let strength = cliff_strength * biome_boost;

            let terraced = self.terrace(raw_height, wx, wz);
            raw_height * (1.0 - strength) + terraced * strength + fine * 1.0
        } else {
            raw_height
        };

        // --- Coastal sea cliffs ---
        let height = self.coastal_shelf(terraced_height, wx, wz);

        (height, biome_val)
    }

    fn sample(&self, wx: f64, wz: f64) -> (usize, f64) {
        let (height, biome_val) = self.sample_raw(wx, wz);
        ((height as usize).clamp(1, WORLD_HEIGHT - 1), biome_val)
    }

    /// Surface material — Ireland/Scotland is GREEN. Grass grows on surprisingly
    /// steep slopes. Only the most sheer cliff faces and highest peaks are bare rock.
    fn surface_block(&self, h: usize, biome_val: f64, steep: f64, wx: f64, wz: f64) -> Block {
        let altitude = h as f64 - SEA_LEVEL as f64;

        // Narrow beach: only right at water's edge, not up the coast
        if altitude <= 1.5 {
            return Block::Sand;
        }

        // Sheer cliff faces → exposed stone (but threshold is high — grass clings)
        if steep > 0.85 {
            // Mix stone and cobble for texture on cliff faces
            let n = self.fine.get([wx * 0.1, wz * 0.1]);
            return if n > 0.3 { Block::Cobble } else { Block::Stone };
        }

        // Moderately steep → grass still holds, but with rock patches
        if steep > 0.6 {
            let n = self.fine.get([wx * 0.08, wz * 0.08]);
            // Mostly grass! With occasional rock peeking through
            return if n > 0.4 { Block::Stone } else if n > 0.2 { Block::Cobble } else { Block::Grass };
        }

        // High mountain tops — bare rock and gravel scree
        // Thresholds scaled for WORLD_HEIGHT=256 terrain
        if biome_val > 0.3 {
            let band_noise = self.detail.get([wx * 0.02, wz * 0.02]) * 8.0;

            if altitude + band_noise > 100.0 {
                let n = self.fine.get([wx * 0.08, wz * 0.08]);
                return if n > 0.0 { Block::Stone } else { Block::Gravel };
            }
            if altitude + band_noise > 70.0 {
                // High highland — patchy grass giving way to rock
                let n = self.fine.get([wx * 0.06, wz * 0.06]);
                return if n > -0.1 { Block::Grass } else { Block::Cobble };
            }
        }

        // Default: grass. Ireland is green.
        Block::Grass
    }

    fn subsurface_block(&self, h: usize, biome_val: f64, steep: f64) -> Block {
        let altitude = h as f64 - SEA_LEVEL as f64;

        if altitude <= 2.0 {
            return Block::Sand;
        }
        if steep > 0.7 || (biome_val > 0.3 && altitude > 60.0) {
            return Block::Stone;
        }
        Block::Dirt
    }

    pub fn generate(&self, chunk: &mut Chunk) {
        let base_x = chunk.cx * CHUNK_SIZE as i32;
        let base_z = chunk.cz * CHUNK_SIZE as i32;

        // First pass: solid terrain
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let wx = (base_x + lx as i32) as f64;
                let wz = (base_z + lz as i32) as f64;

                let (h, biome_val) = self.sample(wx, wz);
                let steep = self.steepness(wx, wz);

                let surface = self.surface_block(h, biome_val, steep, wx, wz);
                let subsurface = self.subsurface_block(h, biome_val, steep);

                let stone_top = h.saturating_sub(5);
                for y in 0..stone_top {
                    chunk.set(lx, y, lz, Block::Stone);
                }

                for y in stone_top..WORLD_HEIGHT {
                    let block = if y + 1 < h {
                        subsurface
                    } else if y < h {
                        surface
                    } else if y < SEA_LEVEL {
                        Block::Water
                    } else {
                        Block::Air
                    };
                    chunk.set(lx, y, lz, block);
                }
            }
        }

        // Second pass: carve ravines
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let wx = (base_x + lx as i32) as f64;
                let wz = (base_z + lz as i32) as f64;

                let (h, _biome_val) = self.sample(wx, wz);
                let (carve, width_factor) = self.ravine_at(wx, wz, h as f64);

                if carve < 1.5 {
                    continue;
                }

                let carve_floor = ((h as f64 - carve) as usize).max(1);

                // Crumbling lip
                if width_factor > 0.3 && width_factor < 0.8 {
                    let lip_y = h.saturating_sub(1);
                    if lip_y > carve_floor {
                        let n = self.fine.get([wx * 0.12, wz * 0.12]);
                        let lip_block = if n > 0.0 { Block::Gravel } else { Block::Dirt };
                        chunk.set(lx, lip_y, lz, lip_block);
                    }
                }

                for y in carve_floor..h {
                    if y < SEA_LEVEL {
                        chunk.set(lx, y, lz, Block::Water);
                    } else {
                        chunk.set(lx, y, lz, Block::Air);
                    }
                }

                if carve_floor > 0 {
                    if carve_floor < SEA_LEVEL {
                        chunk.set(lx, carve_floor, lz, Block::Sand);
                    } else {
                        let n = self.fine.get([wx * 0.15 + 30.0, wz * 0.15 + 30.0]);
                        let floor = if n > 0.2 { Block::Gravel } else if n > -0.3 { Block::Cobble } else { Block::Stone };
                        chunk.set(lx, carve_floor, lz, floor);
                        if carve_floor + 1 < h && width_factor > 0.6 {
                            chunk.set(lx, carve_floor + 1, lz, Block::Gravel);
                        }
                    }
                }
            }
        }

        // Third pass: carve rivers
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let wx = (base_x + lx as i32) as f64;
                let wz = (base_z + lz as i32) as f64;

                let (h, _) = self.sample(wx, wz);
                let (carve, width_factor, water_level) = self.river_at(wx, wz, h as f64);

                if carve < 0.5 {
                    continue;
                }

                let carve_floor = ((h as f64 - carve) as usize).max(1);
                let water_y = water_level as usize;

                // Riverbank edges: sand and gravel
                if width_factor > 0.2 && width_factor < 0.6 {
                    let lip_y = h.saturating_sub(1);
                    if lip_y >= carve_floor {
                        let n = self.fine.get([wx * 0.1 + 50.0, wz * 0.1 + 50.0]);
                        let bank_block = if n > 0.2 { Block::Sand } else { Block::Gravel };
                        chunk.set(lx, lip_y, lz, bank_block);
                    }
                }

                // Carve channel and fill with water
                for y in carve_floor..h {
                    if y <= water_y {
                        chunk.set(lx, y, lz, Block::Water);
                    } else {
                        chunk.set(lx, y, lz, Block::Air);
                    }
                }

                // River bed: sand
                if carve_floor > 0 {
                    chunk.set(lx, carve_floor.saturating_sub(1), lz, Block::Sand);
                    let n = self.fine.get([wx * 0.12 + 70.0, wz * 0.12 + 70.0]);
                    if n > 0.3 {
                        chunk.set(lx, carve_floor, lz, Block::Gravel);
                    }
                }
            }
        }

        // Fourth pass: fill lakes
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let wx = (base_x + lx as i32) as f64;
                let wz = (base_z + lz as i32) as f64;

                let (h, _) = self.sample(wx, wz);

                if let Some(water_level) = self.lake_at(wx, wz, h as f64) {
                    // Fill from terrain surface up to water level
                    for y in h..water_level.min(WORLD_HEIGHT) {
                        chunk.set(lx, y, lz, Block::Water);
                    }

                    // Shore material: sand near the waterline
                    if h > 0 && (water_level - h) < 3 {
                        let n = self.fine.get([wx * 0.08 + 90.0, wz * 0.08 + 90.0]);
                        let shore = if n > 0.0 { Block::Sand } else { Block::Gravel };
                        chunk.set(lx, h.saturating_sub(1), lz, shore);
                    }

                    // Lake bed: sand
                    if h > 1 && h < water_level {
                        chunk.set(lx, h.saturating_sub(1), lz, Block::Sand);
                    }
                }
            }
        }
    }

    pub fn collect_surface_info(chunk: &Chunk) -> Vec<(usize, usize, f32, bool)> {
        let mut info = Vec::new();
        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let mut surface_y = 0usize;
                for y in (0..WORLD_HEIGHT).rev() {
                    let b = chunk.get(lx, y, lz);
                    if b != Block::Air && b != Block::Water {
                        surface_y = y;
                        break;
                    }
                }
                let is_grass = chunk.get(lx, surface_y, lz) == Block::Grass;
                info.push((lx, lz, (surface_y + 1) as f32, is_grass));
            }
        }
        info
    }
}

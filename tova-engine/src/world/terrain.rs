use noise::{NoiseFn, Perlin};

use crate::voxel::block::Block;
use crate::voxel::chunk::{Chunk, CHUNK_SIZE, SEA_LEVEL, WORLD_HEIGHT};

const SEED: u32 = 42;

pub struct TerrainGen {
    /// Base continent shape — large, smooth features
    continent: Perlin,
    /// Detail noise — medium-scale hills and valleys
    detail: Perlin,
    /// Fine noise — small bumps and roughness
    fine: Perlin,
    /// Biome selector — determines plains vs hills vs mountains
    biome: Perlin,
    /// Ridge noise — sharp mountain ridges
    ridge: Perlin,
}

impl TerrainGen {
    pub fn new() -> Self {
        Self {
            continent: Perlin::new(SEED),
            detail: Perlin::new(SEED + 1),
            fine: Perlin::new(SEED + 2),
            biome: Perlin::new(SEED + 3),
            ridge: Perlin::new(SEED + 4),
        }
    }

    /// Sample FBM (fractal Brownian motion) noise at a point.
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

    /// Ridged noise — absolute value creates sharp ridges.
    fn ridged(&self, x: f64, z: f64, octaves: u32) -> f64 {
        let mut sum = 0.0;
        let mut amp = 1.0;
        let mut freq = 1.0;
        let gain = 0.5;
        let lacunarity = 2.0;

        for _ in 0..octaves {
            let v = self.ridge.get([x * freq, z * freq]);
            // Invert absolute value: peaks become ridges
            let ridged = 1.0 - v.abs();
            // Square it for sharper ridges
            sum += ridged * ridged * amp;
            amp *= gain;
            freq *= lacunarity;
        }

        sum
    }

    /// Sample biome value at world coordinates.
    fn biome_at(&self, wx: f64, wz: f64) -> f64 {
        self.fbm(&self.biome, wx * 0.001, wz * 0.001, 3, 2.0, 0.5)
    }

    /// Get terrain height and biome value at world coordinates.
    fn sample(&self, wx: f64, wz: f64) -> (usize, f64) {
        let biome_val = self.biome_at(wx, wz);

        // Continent shape — broad, gentle
        let continent = self.fbm(&self.continent, wx * 0.002, wz * 0.002, 4, 2.0, 0.45) * 0.5 + 0.5;

        // Detail hills
        let detail = self.fbm(&self.detail, wx * 0.008, wz * 0.008, 4, 2.0, 0.5);

        // Fine roughness
        let fine = self.fbm(&self.fine, wx * 0.03, wz * 0.03, 2, 2.0, 0.5);

        // Ridge mountains
        let ridge = self.ridged(wx * 0.004, wz * 0.004, 4);

        // Blend based on biome:
        //   biome < -0.2  → flat plains (gentle)
        //   biome -0.2..0.3  → rolling hills
        //   biome > 0.3   → mountains with ridges
        let base = SEA_LEVEL as f64;

        let height = if biome_val < -0.2 {
            // Plains — gentle rolling terrain, slightly above sea level
            let flatness = ((-0.2 - biome_val) * 2.0).min(1.0);
            let plains_h = continent * 12.0 + detail * 4.0 + fine * 1.0;
            base + plains_h * (1.0 - flatness * 0.6) + 2.0
        } else if biome_val < 0.3 {
            // Rolling hills — mix of continent and detail
            let t = (biome_val + 0.2) / 0.5; // 0..1
            let hill_h = continent * 18.0 + detail * 10.0 * t + fine * 2.0;
            base + hill_h + 4.0
        } else {
            // Mountains — ridged noise dominates
            let t = ((biome_val - 0.3) * 2.0).min(1.0);
            let mountain_h = continent * 15.0 + ridge * 30.0 * t + detail * 8.0 + fine * 2.0;
            base + mountain_h + 8.0
        };

        ((height as usize).clamp(1, WORLD_HEIGHT - 1), biome_val)
    }

    /// Fill a chunk with terrain data.
    pub fn generate(&self, chunk: &mut Chunk) {
        let base_x = chunk.cx * CHUNK_SIZE as i32;
        let base_z = chunk.cz * CHUNK_SIZE as i32;

        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let wx = (base_x + lx as i32) as f64;
                let wz = (base_z + lz as i32) as f64;

                let (h, biome_val) = self.sample(wx, wz);

                // Bulk fill stone below surface
                let stone_top = h.saturating_sub(5);
                for y in 0..stone_top {
                    chunk.set(lx, y, lz, Block::Stone);
                }

                for y in stone_top..WORLD_HEIGHT {
                    let block = if y < h.saturating_sub(1) {
                        // Near surface: depends on height and biome
                        if h <= SEA_LEVEL + 3 {
                            Block::Sand
                        } else if biome_val > 0.3 && h > SEA_LEVEL + 20 {
                            // High mountains — stone all the way
                            Block::Stone
                        } else {
                            Block::Dirt
                        }
                    } else if y < h {
                        // Surface block
                        if h <= SEA_LEVEL + 2 {
                            Block::Sand
                        } else if h <= SEA_LEVEL + 4 {
                            // Beach transition
                            Block::Sand
                        } else if biome_val > 0.3 && h > SEA_LEVEL + 25 {
                            // Mountain peaks — exposed stone
                            Block::Stone
                        } else if biome_val > 0.3 && h > SEA_LEVEL + 18 {
                            // High altitude — cobblestone/gravel
                            Block::Cobble
                        } else {
                            Block::Grass
                        }
                    } else if y < SEA_LEVEL {
                        Block::Water
                    } else {
                        Block::Air
                    };
                    chunk.set(lx, y, lz, block);
                }
            }
        }
    }
}

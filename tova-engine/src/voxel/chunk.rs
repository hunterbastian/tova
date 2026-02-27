use super::block::Block;

pub const CHUNK_SIZE: usize = 16;
pub const WORLD_HEIGHT: usize = 128;
pub const SEA_LEVEL: usize = 48;
pub const DEFAULT_WORLD_SEED: u64 = 0x544F_5641_2026_0001;

pub struct Chunk {
    pub cx: i32,
    pub cz: i32,
    pub blocks: Vec<u8>,
}

impl Chunk {
    pub fn new(cx: i32, cz: i32) -> Self {
        Self {
            cx,
            cz,
            blocks: vec![0; CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT],
        }
    }

    #[inline]
    pub fn index(x: usize, y: usize, z: usize) -> usize {
        x + CHUNK_SIZE * (z + CHUNK_SIZE * y)
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> Block {
        Block::from_u8(self.blocks[Self::index(x, y, z)])
    }

    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, block: Block) {
        self.blocks[Self::index(x, y, z)] = block as u8;
    }

    /// Seeded procedural terrain generation with biome selection and cave carving.
    pub fn generate_procedural(&mut self, seed: u64) {
        let base_x = self.cx * CHUNK_SIZE as i32;
        let base_z = self.cz * CHUNK_SIZE as i32;

        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let wx = base_x + lx as i32;
                let wz = base_z + lz as i32;
                let surface_y = terrain_height(seed, wx, wz);
                let biome = biome_at(seed, wx, wz, surface_y);
                let (top_block, filler_block) = surface_blocks_for_biome(biome, surface_y);
                let solid_top = surface_y.min(WORLD_HEIGHT - 1);

                for y in 0..=solid_top {
                    let mut block = if y == solid_top {
                        top_block
                    } else if y + 3 >= solid_top {
                        filler_block
                    } else {
                        Block::Stone
                    };

                    if y > 6 && y + 4 < solid_top && should_carve_cave(seed, wx, y as i32, wz) {
                        block = Block::Air;
                    }

                    self.set(lx, y, lz, block);
                }

                for y in solid_top + 1..WORLD_HEIGHT {
                    let block = if y < SEA_LEVEL {
                        Block::Water
                    } else {
                        Block::Air
                    };
                    self.set(lx, y, lz, block);
                }
            }
        }
    }
}

pub fn world_seed_from_env() -> u64 {
    std::env::var("TOVA_WORLD_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(DEFAULT_WORLD_SEED)
}

#[derive(Clone, Copy)]
enum Biome {
    Grassland,
    Desert,
    Rocky,
}

fn terrain_height(seed: u64, wx: i32, wz: i32) -> usize {
    let x = wx as f32;
    let z = wz as f32;

    let continental = fbm2(seed ^ 0x00A1_B2C3_D401, x * 0.0024, z * 0.0024, 4);
    let erosion = fbm2(seed ^ 0x00A1_B2C3_D402, x * 0.0055, z * 0.0055, 3);
    let detail = fbm2(seed ^ 0x00A1_B2C3_D403, x * 0.019, z * 0.019, 3);

    // Ridged mountains where erosion noise is low.
    let ridged = 1.0 - erosion.abs();
    let uplift = (continental * 0.5 + 0.5).clamp(0.0, 1.0);

    let mut height = SEA_LEVEL as f32;
    height += continental * 18.0;
    height += ridged * uplift * 14.0;
    height += detail * 4.5;

    height.round().clamp(6.0, (WORLD_HEIGHT - 2) as f32) as usize
}

fn biome_at(seed: u64, wx: i32, wz: i32, surface_y: usize) -> Biome {
    let x = wx as f32;
    let z = wz as f32;
    let temperature = fbm2(seed ^ 0xBEEF_0001, x * 0.0018, z * 0.0018, 3) * 0.5 + 0.5;
    let moisture = fbm2(seed ^ 0xBEEF_0002, x * 0.0019, z * 0.0019, 3) * 0.5 + 0.5;
    let elevation = ((surface_y as i32 - SEA_LEVEL as i32) as f32 / 48.0).clamp(0.0, 1.0);

    if surface_y <= SEA_LEVEL + 2 || (temperature > 0.62 && moisture < 0.44) {
        Biome::Desert
    } else if elevation > 0.55 && moisture < 0.58 {
        Biome::Rocky
    } else {
        Biome::Grassland
    }
}

fn surface_blocks_for_biome(biome: Biome, surface_y: usize) -> (Block, Block) {
    if surface_y <= SEA_LEVEL + 1 {
        return (Block::Sand, Block::Sand);
    }

    match biome {
        Biome::Grassland => (Block::Grass, Block::Dirt),
        Biome::Desert => (Block::Sand, Block::Sand),
        Biome::Rocky => (Block::Cobble, Block::Stone),
    }
}

fn should_carve_cave(seed: u64, wx: i32, wy: i32, wz: i32) -> bool {
    let x = wx as f32;
    let y = wy as f32;
    let z = wz as f32;

    let cave_a = value_noise_3d(seed ^ 0x00CA_FEBA_BE01, x * 0.06, y * 0.085, z * 0.06);
    let cave_b = value_noise_3d(seed ^ 0x00CA_FEBA_BE02, x * 0.03, y * 0.045, z * 0.03);
    let density = cave_a * 0.72 + cave_b * 0.28;

    // Fewer caves near sea level to avoid flooding large portions of terrain.
    let sea_bias = ((SEA_LEVEL as i32 - wy) as f32 / 48.0).clamp(0.0, 1.0);
    density > (0.63 + sea_bias * 0.14)
}

fn fbm2(seed: u64, x: f32, z: f32, octaves: usize) -> f32 {
    let mut sum = 0.0;
    let mut amplitude = 1.0;
    let mut frequency = 1.0;
    let mut norm = 0.0;

    for octave in 0..octaves {
        let octave_seed = seed.wrapping_add((octave as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15));
        sum += value_noise_2d(octave_seed, x * frequency, z * frequency) * amplitude;
        norm += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }

    if norm <= f32::EPSILON {
        0.0
    } else {
        sum / norm
    }
}

fn value_noise_2d(seed: u64, x: f32, z: f32) -> f32 {
    let x0 = x.floor() as i32;
    let z0 = z.floor() as i32;
    let x1 = x0 + 1;
    let z1 = z0 + 1;

    let tx = smoothstep(x - x0 as f32);
    let tz = smoothstep(z - z0 as f32);

    let n00 = hash01_2d(seed, x0, z0) * 2.0 - 1.0;
    let n10 = hash01_2d(seed, x1, z0) * 2.0 - 1.0;
    let n01 = hash01_2d(seed, x0, z1) * 2.0 - 1.0;
    let n11 = hash01_2d(seed, x1, z1) * 2.0 - 1.0;

    let nx0 = lerp(n00, n10, tx);
    let nx1 = lerp(n01, n11, tx);
    lerp(nx0, nx1, tz)
}

fn value_noise_3d(seed: u64, x: f32, y: f32, z: f32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let z0 = z.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let z1 = z0 + 1;

    let tx = smoothstep(x - x0 as f32);
    let ty = smoothstep(y - y0 as f32);
    let tz = smoothstep(z - z0 as f32);

    let c000 = hash01_3d(seed, x0, y0, z0);
    let c100 = hash01_3d(seed, x1, y0, z0);
    let c010 = hash01_3d(seed, x0, y1, z0);
    let c110 = hash01_3d(seed, x1, y1, z0);
    let c001 = hash01_3d(seed, x0, y0, z1);
    let c101 = hash01_3d(seed, x1, y0, z1);
    let c011 = hash01_3d(seed, x0, y1, z1);
    let c111 = hash01_3d(seed, x1, y1, z1);

    let x00 = lerp(c000, c100, tx);
    let x10 = lerp(c010, c110, tx);
    let x01 = lerp(c001, c101, tx);
    let x11 = lerp(c011, c111, tx);

    let y0v = lerp(x00, x10, ty);
    let y1v = lerp(x01, x11, ty);
    lerp(y0v, y1v, tz)
}

#[inline]
fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn hash01_2d(seed: u64, x: i32, z: i32) -> f32 {
    let mut h = seed;
    h ^= (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= (z as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h as u32) as f32 / u32::MAX as f32
}

fn hash01_3d(seed: u64, x: i32, y: i32, z: i32) -> f32 {
    let mut h = seed;
    h ^= (x as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= (y as i64 as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93);
    h ^= (z as i64 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    (h as u32) as f32 / u32::MAX as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn procedural_generation_is_deterministic_for_seed() {
        let seed = 12345;
        let mut a = Chunk::new(2, -3);
        let mut b = Chunk::new(2, -3);
        a.generate_procedural(seed);
        b.generate_procedural(seed);
        assert_eq!(a.blocks, b.blocks);
    }

    #[test]
    fn different_seeds_generate_different_chunks() {
        let mut a = Chunk::new(0, 0);
        let mut b = Chunk::new(0, 0);
        a.generate_procedural(1);
        b.generate_procedural(2);
        assert_ne!(a.blocks, b.blocks);
    }
}

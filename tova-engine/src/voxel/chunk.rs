use super::block::Block;

pub const CHUNK_SIZE: usize = 16;
pub const WORLD_HEIGHT: usize = 128;
pub const SEA_LEVEL: usize = 48;

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

    /// Generate simple terrain for testing: stone base, dirt, grass top, sand near sea level.
    pub fn generate_test(&mut self) {
        let base_x = self.cx * CHUNK_SIZE as i32;
        let base_z = self.cz * CHUNK_SIZE as i32;

        for lz in 0..CHUNK_SIZE {
            for lx in 0..CHUNK_SIZE {
                let wx = base_x + lx as i32;
                let wz = base_z + lz as i32;

                // Simple height function: rolling hills
                let h = simple_height(wx, wz);

                for y in 0..WORLD_HEIGHT {
                    let block = if y == 0 {
                        Block::Stone
                    } else if y < h.saturating_sub(4) {
                        Block::Stone
                    } else if y < h.saturating_sub(1) {
                        if h <= SEA_LEVEL + 2 {
                            Block::Sand
                        } else {
                            Block::Dirt
                        }
                    } else if y < h {
                        if h <= SEA_LEVEL + 2 {
                            Block::Sand
                        } else {
                            Block::Grass
                        }
                    } else if y < SEA_LEVEL {
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

/// Simple deterministic height using integer hash (no noise crate needed yet).
fn simple_height(wx: i32, wz: i32) -> usize {
    let base = SEA_LEVEL as f32;
    let fx = wx as f32 * 0.02;
    let fz = wz as f32 * 0.02;

    // Layered sine waves for rolling hills
    let h1 = (fx * 0.7 + 0.3).sin() * 8.0;
    let h2 = (fz * 0.9 + 1.1).sin() * 6.0;
    let h3 = (fx * 1.8 + fz * 1.3).sin() * 3.0;
    let h4 = (fx * 0.3 - fz * 0.5 + 2.0).sin() * 12.0;

    let height = base + h1 + h2 + h3 + h4;
    (height as usize).clamp(1, WORLD_HEIGHT - 1)
}

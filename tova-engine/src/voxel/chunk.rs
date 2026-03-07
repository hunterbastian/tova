use super::block::Block;

pub const CHUNK_SIZE: usize = 16;
pub const WORLD_HEIGHT: usize = 128;
pub const SEA_LEVEL: usize = 30;

pub const SPAWN_X: i32 = 0;
pub const SPAWN_SURFACE_Y: usize = 38;
pub const SPAWN_CAMERA_Y: usize = SPAWN_SURFACE_Y + 3;
pub const SPAWN_Z: i32 = 0;

#[derive(Clone)]
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
}

#[cfg(test)]
mod tests {
    use super::{Block, Chunk};

    #[test]
    fn new_chunks_start_empty() {
        let chunk = Chunk::new(0, 0);
        assert_eq!(chunk.get(0, 0, 0), Block::Air);
        assert_eq!(chunk.get(15, 0, 15), Block::Air);
    }

    #[test]
    fn setting_a_block_round_trips() {
        let mut chunk = Chunk::new(0, 0);
        chunk.set(4, 12, 9, Block::Stone);
        assert_eq!(chunk.get(4, 12, 9), Block::Stone);
    }
}

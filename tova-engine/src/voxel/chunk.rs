use super::block::Block;

pub const CHUNK_SIZE: usize = 16;
pub const WORLD_HEIGHT: usize = 256;
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
}

/// Block type IDs matching the original Tova world.
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Air = 0,
    Grass = 1,
    Dirt = 2,
    Stone = 3,
    Sand = 4,
    Water = 5,
    Cobble = 6,
}

impl Block {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Block::Grass,
            2 => Block::Dirt,
            3 => Block::Stone,
            4 => Block::Sand,
            5 => Block::Water,
            6 => Block::Cobble,
            _ => Block::Air,
        }
    }

    pub fn is_solid(self) -> bool {
        !matches!(self, Block::Air | Block::Water)
    }

    pub fn is_transparent(self) -> bool {
        matches!(self, Block::Air)
    }
}

pub const BLOCK_COLORS: [[f32; 3]; 7] = [
    [0.0, 0.0, 0.0],       // Air (unused)
    [0.31, 0.54, 0.25],    // Grass
    [0.43, 0.31, 0.20],    // Dirt
    [0.52, 0.55, 0.58],    // Stone
    [0.71, 0.65, 0.44],    // Sand
    [0.16, 0.34, 0.58],    // Water
    [0.60, 0.60, 0.62],    // Cobble
];

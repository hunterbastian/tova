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
    [0.38, 0.42, 0.32],    // Grass — grey-olive, desaturated
    [0.40, 0.35, 0.28],    // Dirt — dusty tan
    [0.45, 0.44, 0.42],    // Stone — warm slate grey
    [0.55, 0.52, 0.42],    // Sand — muted khaki
    [0.28, 0.32, 0.35],    // Water — dark slate blue-grey
    [0.48, 0.46, 0.43],    // Cobble — weathered grey
];

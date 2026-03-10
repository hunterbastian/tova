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

}

pub const BLOCK_COLORS: [[f32; 3]; 7] = [
    [0.0, 0.0, 0.0],       // Air (unused)
    [0.35, 0.38, 0.28],    // Grass — ashy olive, Bitter Coast scrub
    [0.38, 0.33, 0.25],    // Dirt — Vvardenfell dust
    [0.40, 0.38, 0.35],    // Stone — dark volcanic grey
    [0.50, 0.46, 0.36],    // Sand — Azura's Coast beige
    [0.25, 0.28, 0.30],    // Water — murky, ashfall-tinted
    [0.42, 0.40, 0.37],    // Cobble — weathered Dwemer stone
];

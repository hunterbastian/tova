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
    Wood = 7,
    Leaves = 8,
    Gravel = 9,
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
            7 => Block::Wood,
            8 => Block::Leaves,
            9 => Block::Gravel,
            _ => Block::Air,
        }
    }

    pub fn is_solid(self) -> bool {
        !matches!(self, Block::Air | Block::Water | Block::Wood | Block::Leaves)
    }
}

pub const BLOCK_COLORS: [[f32; 3]; 10] = [
    [0.0, 0.0, 0.0],       // Air (unused)
    [0.38, 0.42, 0.28],    // Grass — highland green, slightly richer for new lighting
    [0.40, 0.34, 0.26],    // Dirt — warm earth
    [0.42, 0.40, 0.37],    // Stone — volcanic grey with warmth
    [0.52, 0.48, 0.38],    // Sand — coastal beige
    [0.24, 0.28, 0.32],    // Water — deep murky blue-grey
    [0.44, 0.42, 0.39],    // Cobble — weathered stone
    [0.34, 0.27, 0.18],    // Wood — rich dark bark
    [0.32, 0.38, 0.22],    // Leaves — olive green, more alive
    [0.46, 0.43, 0.39],    // Gravel — warm scree
];

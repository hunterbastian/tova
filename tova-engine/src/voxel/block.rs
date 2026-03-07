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
    Leaf = 8,
}

#[allow(dead_code)]
pub const BLOCK_COUNT: usize = 9;

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
            8 => Block::Leaf,
            _ => Block::Air,
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Block::Air => "Air",
            Block::Grass => "Grass",
            Block::Dirt => "Dirt",
            Block::Stone => "Stone",
            Block::Sand => "Sand",
            Block::Water => "Water",
            Block::Cobble => "Cobble",
            Block::Wood => "Wood",
            Block::Leaf => "Leaf",
        }
    }

    pub fn is_replaceable(self) -> bool {
        matches!(self, Block::Air | Block::Water)
    }

    pub fn is_collectible(self) -> bool {
        !matches!(self, Block::Air | Block::Water)
    }

    pub fn is_solid(self) -> bool {
        !matches!(self, Block::Air | Block::Water)
    }

    #[allow(dead_code)]
    pub fn is_placeable(self) -> bool {
        self.is_collectible()
    }
}

pub const BLOCK_COLORS: [[f32; 3]; 9] = [
    [0.0, 0.0, 0.0],    // Air (unused)
    [0.27, 0.33, 0.24], // Grass — mildewed field green
    [0.23, 0.19, 0.17], // Dirt — wet umber
    [0.29, 0.30, 0.33], // Stone — cold slate
    [0.34, 0.31, 0.27], // Sand — ash-tinted silt
    [0.08, 0.11, 0.13], // Water — near-black marsh water
    [0.25, 0.26, 0.29], // Cobble — damp ruin stone
    [0.29, 0.22, 0.16], // Wood — dark umber trunk
    [0.21, 0.27, 0.18], // Leaf — ash-green canopy
];

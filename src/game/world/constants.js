export const CHUNK_SIZE = 16;
export const WORLD_HEIGHT = 128;
export const CHUNK_VOLUME = CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT;

export const SEA_LEVEL = 48;

export const CORE_HALF_SIZE = 2000; // 4km x 4km authored core centered at origin.
export const BLEND_RING_SIZE = 800;

export const LOAD_RADIUS = 4;
export const UNLOAD_RADIUS = 5;

export const MAX_CHUNK_BUILDS_PER_FRAME = 2;
export const MAX_MESH_BUILDS_PER_FRAME = 1;

export const DEV_SEED = 133742;

export const BLOCK = {
    AIR: 0,
    GRASS: 1,
    DIRT: 2,
    STONE: 3,
    SAND: 4,
    WATER: 5,
    COBBLE: 6
};

export const BIOME = {
    PLAINS: 0,
    FOREST: 1,
    ALPINE: 2,
    COAST: 3
};

export const isSolidBlock = (blockId) => blockId !== BLOCK.AIR && blockId !== BLOCK.WATER;

export const isTransparentBlock = (blockId) => blockId === BLOCK.AIR;

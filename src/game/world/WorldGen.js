import { AuthoredField } from './TerrainField.js';
import { BIOME, BLOCK, CHUNK_SIZE, SEA_LEVEL, WORLD_HEIGHT } from './constants.js';

const clamp = (value, min, max) => Math.min(max, Math.max(min, value));

const blockIndex = (lx, y, lz) => lx + CHUNK_SIZE * (lz + CHUNK_SIZE * y);

export class WorldGen {
    constructor(options = {}) {
        this.seed = options.seed ?? 133742;
        this.field = options.field ?? new AuthoredField({ seed: this.seed });
    }

    getTopBlockForBiome(biome) {
        if (biome === BIOME.COAST) return BLOCK.SAND;
        if (biome === BIOME.ALPINE) return BLOCK.STONE;
        return BLOCK.GRASS;
    }

    getSurfaceStampBlock(structureTag, fallbackBlock) {
        if (!structureTag) return fallbackBlock;
        if (structureTag === 'road') return BLOCK.COBBLE;
        if (structureTag === 'town') return BLOCK.COBBLE;
        if (structureTag === 'castle') return BLOCK.COBBLE;
        return fallbackBlock;
    }

    generateChunk(cx, cz) {
        const blocks = new Uint8Array(CHUNK_SIZE * CHUNK_SIZE * WORLD_HEIGHT);
        const worldBaseX = cx * CHUNK_SIZE;
        const worldBaseZ = cz * CHUNK_SIZE;

        for (let lz = 0; lz < CHUNK_SIZE; lz += 1) {
            for (let lx = 0; lx < CHUNK_SIZE; lx += 1) {
                const wx = worldBaseX + lx;
                const wz = worldBaseZ + lz;

                const terrainHeight = clamp(this.field.sampleHeight(wx, wz), 1, WORLD_HEIGHT - 1);
                const biome = this.field.sampleBiome(wx, wz);
                const structureTag = this.field.getStructureTag(wx, wz);

                const topBlock = this.getSurfaceStampBlock(structureTag, this.getTopBlockForBiome(biome));
                const subsoilBlock = biome === BIOME.COAST ? BLOCK.SAND : BLOCK.DIRT;

                for (let y = 0; y <= terrainHeight; y += 1) {
                    let blockId = BLOCK.STONE;

                    if (y === terrainHeight) {
                        blockId = topBlock;
                    } else if (y >= terrainHeight - 3) {
                        blockId = subsoilBlock;
                    }

                    // Give the core castle area a deeper stone foundation profile.
                    if (structureTag === 'castle' && y >= terrainHeight - 6) {
                        blockId = BLOCK.COBBLE;
                    }

                    blocks[blockIndex(lx, y, lz)] = blockId;
                }

                for (let y = terrainHeight + 1; y <= SEA_LEVEL && y < WORLD_HEIGHT; y += 1) {
                    blocks[blockIndex(lx, y, lz)] = BLOCK.WATER;
                }
            }
        }

        return blocks;
    }
}

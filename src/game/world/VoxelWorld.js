import { ChunkManager } from './ChunkManager.js';
import { DEV_SEED } from './constants.js';
import { AuthoredField } from './TerrainField.js';
import { WorldGen } from './WorldGen.js';

export class VoxelWorld {
    constructor(scene, options = {}) {
        this.scene = scene;
        this.seed = options.seed ?? DEV_SEED;
        this.field = new AuthoredField({ seed: this.seed });
        this.generator = new WorldGen({ seed: this.seed, field: this.field });
        this.chunkManager = new ChunkManager(scene, this.generator, {
            enableShadows: options.enableShadows ?? true
        });
    }

    update(playerPosition) {
        this.chunkManager.update(playerPosition.x, playerPosition.z);
    }

    getHeightAt(x, z) {
        return this.field.sampleHeight(x, z);
    }

    getDebugStats(playerX, playerZ) {
        const stats = this.chunkManager.getStats();
        const blend = this.field.blendFactor(playerX, playerZ);
        const zone = blend <= 0 ? 'core' : blend >= 1 ? 'frontier' : 'blend';

        return {
            ...stats,
            zone,
            blend
        };
    }
}

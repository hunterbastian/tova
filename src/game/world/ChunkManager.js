import { VoxelMesher } from '../mesh/VoxelMesher.js';
import { Chunk } from './Chunk.js';
import {
    BLOCK,
    CHUNK_SIZE,
    LOAD_RADIUS,
    MAX_CHUNK_BUILDS_PER_FRAME,
    MAX_MESH_BUILDS_PER_FRAME,
    UNLOAD_RADIUS,
    WORLD_HEIGHT
} from './constants.js';

const chunkKey = (cx, cz) => `${cx},${cz}`;

const parseChunkKey = (key) => {
    const [cx, cz] = key.split(',').map(Number);
    return { cx, cz };
};

const toChunkCoord = (value) => Math.floor(value / CHUNK_SIZE);
const toLocalCoord = (value) => ((value % CHUNK_SIZE) + CHUNK_SIZE) % CHUNK_SIZE;
const blockIndex = (lx, y, lz) => lx + CHUNK_SIZE * (lz + CHUNK_SIZE * y);

const chebyshevDistance = (ax, az, bx, bz) => Math.max(Math.abs(ax - bx), Math.abs(az - bz));

export class ChunkManager {
    constructor(scene, worldGen, options = {}) {
        this.scene = scene;
        this.worldGen = worldGen;
        this.mesher = new VoxelMesher({ enableShadows: options.enableShadows ?? true });

        this.chunks = new Map();
        this.generationQueue = [];
        this.generationQueueSet = new Set();
        this.meshQueue = new Set();

        this.playerChunk = { cx: 0, cz: 0 };
    }

    worldToChunk(x, z) {
        return { cx: toChunkCoord(x), cz: toChunkCoord(z) };
    }

    getChunk(cx, cz) {
        return this.chunks.get(chunkKey(cx, cz)) ?? null;
    }

    getBlockAtWorld(x, y, z) {
        if (y < 0 || y >= WORLD_HEIGHT) {
            return BLOCK.AIR;
        }

        const cx = toChunkCoord(x);
        const cz = toChunkCoord(z);
        const key = chunkKey(cx, cz);
        const chunk = this.chunks.get(key);
        if (!chunk) {
            return BLOCK.AIR;
        }

        const lx = toLocalCoord(x);
        const lz = toLocalCoord(z);
        return chunk.blocks[blockIndex(lx, y, lz)] ?? BLOCK.AIR;
    }

    queueChunk(cx, cz) {
        const key = chunkKey(cx, cz);
        if (this.chunks.has(key) || this.generationQueueSet.has(key)) {
            return;
        }
        this.generationQueue.push(key);
        this.generationQueueSet.add(key);
    }

    queueRemesh(cx, cz) {
        const key = chunkKey(cx, cz);
        if (!this.chunks.has(key)) {
            return;
        }
        this.meshQueue.add(key);
    }

    queueChunkAndNeighborsForRemesh(cx, cz) {
        this.queueRemesh(cx, cz);
        this.queueRemesh(cx + 1, cz);
        this.queueRemesh(cx - 1, cz);
        this.queueRemesh(cx, cz + 1);
        this.queueRemesh(cx, cz - 1);
    }

    unloadChunk(cx, cz) {
        const key = chunkKey(cx, cz);
        const chunk = this.chunks.get(key);
        if (!chunk) {
            return;
        }

        if (chunk.mesh) {
            this.scene.remove(chunk.mesh);
            chunk.mesh.geometry.dispose();
            chunk.mesh = null;
        }

        this.chunks.delete(key);
        this.meshQueue.delete(key);

        this.queueRemesh(cx + 1, cz);
        this.queueRemesh(cx - 1, cz);
        this.queueRemesh(cx, cz + 1);
        this.queueRemesh(cx, cz - 1);
    }

    update(playerX, playerZ) {
        const center = this.worldToChunk(playerX, playerZ);
        this.playerChunk = center;

        for (let dz = -LOAD_RADIUS; dz <= LOAD_RADIUS; dz += 1) {
            for (let dx = -LOAD_RADIUS; dx <= LOAD_RADIUS; dx += 1) {
                this.queueChunk(center.cx + dx, center.cz + dz);
            }
        }

        for (const [key, chunk] of this.chunks.entries()) {
            if (chebyshevDistance(chunk.cx, chunk.cz, center.cx, center.cz) > UNLOAD_RADIUS) {
                this.unloadChunk(chunk.cx, chunk.cz);
            }
        }

        this.buildQueuedChunks();
        this.buildQueuedMeshes();
    }

    buildQueuedChunks() {
        let built = 0;

        while (built < MAX_CHUNK_BUILDS_PER_FRAME && this.generationQueue.length > 0) {
            const key = this.generationQueue.shift();
            this.generationQueueSet.delete(key);

            if (this.chunks.has(key)) {
                continue;
            }

            const { cx, cz } = parseChunkKey(key);
            const blocks = this.worldGen.generateChunk(cx, cz);
            const chunk = new Chunk(cx, cz, blocks);
            this.chunks.set(key, chunk);
            this.queueChunkAndNeighborsForRemesh(cx, cz);

            built += 1;
        }
    }

    buildQueuedMeshes() {
        let built = 0;

        while (built < MAX_MESH_BUILDS_PER_FRAME && this.meshQueue.size > 0) {
            const iterator = this.meshQueue.values().next();
            if (iterator.done) {
                break;
            }

            const key = iterator.value;
            this.meshQueue.delete(key);

            const chunk = this.chunks.get(key);
            if (!chunk) {
                continue;
            }

            const geometry = this.mesher.buildGeometry(chunk, (x, y, z) => this.getBlockAtWorld(x, y, z));
            if (!geometry) {
                if (chunk.mesh) {
                    this.scene.remove(chunk.mesh);
                    chunk.mesh.geometry.dispose();
                    chunk.mesh = null;
                }
                chunk.state = 'meshed';
                built += 1;
                continue;
            }

            if (!chunk.mesh) {
                chunk.mesh = this.mesher.createMesh(geometry);
                if (chunk.mesh) {
                    this.scene.add(chunk.mesh);
                }
            } else {
                chunk.mesh.geometry.dispose();
                chunk.mesh.geometry = geometry;
            }

            chunk.state = 'active';
            built += 1;
        }
    }

    getStats() {
        return {
            loadedChunks: this.chunks.size,
            pendingChunks: this.generationQueue.length,
            pendingMeshes: this.meshQueue.size,
            playerChunkX: this.playerChunk.cx,
            playerChunkZ: this.playerChunk.cz
        };
    }
}

import * as THREE from 'three';
import { BLOCK, CHUNK_SIZE, SEA_LEVEL, WORLD_HEIGHT } from '../world/constants.js';

const FACE_DEFS = [
    {
        normal: [1, 0, 0],
        offset: [1, 0, 0],
        corners: [
            [1, 0, 0],
            [1, 1, 0],
            [1, 1, 1],
            [1, 0, 1]
        ],
        shade: 0.84
    },
    {
        normal: [-1, 0, 0],
        offset: [-1, 0, 0],
        corners: [
            [0, 0, 1],
            [0, 1, 1],
            [0, 1, 0],
            [0, 0, 0]
        ],
        shade: 0.72
    },
    {
        normal: [0, 1, 0],
        offset: [0, 1, 0],
        corners: [
            [0, 1, 1],
            [1, 1, 1],
            [1, 1, 0],
            [0, 1, 0]
        ],
        shade: 1.0
    },
    {
        normal: [0, -1, 0],
        offset: [0, -1, 0],
        corners: [
            [0, 0, 0],
            [1, 0, 0],
            [1, 0, 1],
            [0, 0, 1]
        ],
        shade: 0.55
    },
    {
        normal: [0, 0, 1],
        offset: [0, 0, 1],
        corners: [
            [1, 0, 1],
            [1, 1, 1],
            [0, 1, 1],
            [0, 0, 1]
        ],
        shade: 0.8
    },
    {
        normal: [0, 0, -1],
        offset: [0, 0, -1],
        corners: [
            [0, 0, 0],
            [0, 1, 0],
            [1, 1, 0],
            [1, 0, 0]
        ],
        shade: 0.75
    }
];

const BLOCK_COLORS = {
    [BLOCK.GRASS]: [0.31, 0.54, 0.25],
    [BLOCK.DIRT]: [0.43, 0.31, 0.2],
    [BLOCK.STONE]: [0.52, 0.55, 0.58],
    [BLOCK.SAND]: [0.71, 0.65, 0.44],
    [BLOCK.WATER]: [0.16, 0.34, 0.58],
    [BLOCK.COBBLE]: [0.6, 0.6, 0.62]
};

const shouldRenderFace = (current, neighbor) => {
    if (current === BLOCK.AIR) return false;
    if (current === BLOCK.WATER) {
        return neighbor === BLOCK.AIR;
    }
    if (neighbor === BLOCK.AIR || neighbor === BLOCK.WATER) {
        return true;
    }
    return false;
};

export class VoxelMesher {
    constructor(options = {}) {
        this.enableShadows = options.enableShadows ?? true;
        this.material = new THREE.MeshStandardMaterial({
            vertexColors: true,
            roughness: 0.92,
            metalness: 0.02,
            flatShading: true
        });
    }

    createMesh(geometry) {
        if (!geometry) return null;
        const mesh = new THREE.Mesh(geometry, this.material);
        mesh.castShadow = this.enableShadows;
        mesh.receiveShadow = this.enableShadows;
        mesh.frustumCulled = true;
        return mesh;
    }

    buildGeometry(chunk, getBlockAtWorld) {
        const positions = [];
        const normals = [];
        const colors = [];
        const indices = [];

        const baseX = chunk.cx * CHUNK_SIZE;
        const baseZ = chunk.cz * CHUNK_SIZE;

        let vertexIndex = 0;

        for (let y = 0; y < WORLD_HEIGHT; y += 1) {
            for (let lz = 0; lz < CHUNK_SIZE; lz += 1) {
                for (let lx = 0; lx < CHUNK_SIZE; lx += 1) {
                    const localIndex = lx + CHUNK_SIZE * (lz + CHUNK_SIZE * y);
                    const blockId = chunk.blocks[localIndex];
                    if (blockId === BLOCK.AIR) continue;

                    const wx = baseX + lx;
                    const wz = baseZ + lz;
                    const altitude = Math.min(1.15, Math.max(0.7, 0.88 + (y - SEA_LEVEL) * 0.004));
                    const baseColor = BLOCK_COLORS[blockId] ?? BLOCK_COLORS[BLOCK.STONE];

                    for (let faceIndex = 0; faceIndex < FACE_DEFS.length; faceIndex += 1) {
                        const face = FACE_DEFS[faceIndex];
                        const nx = wx + face.offset[0];
                        const ny = y + face.offset[1];
                        const nz = wz + face.offset[2];
                        const neighborId = getBlockAtWorld(nx, ny, nz);

                        if (!shouldRenderFace(blockId, neighborId)) continue;

                        for (let i = 0; i < 4; i += 1) {
                            const corner = face.corners[i];
                            positions.push(wx + corner[0], y + corner[1], wz + corner[2]);
                            normals.push(face.normal[0], face.normal[1], face.normal[2]);

                            const shade = face.shade * altitude;
                            colors.push(
                                Math.min(1, baseColor[0] * shade),
                                Math.min(1, baseColor[1] * shade),
                                Math.min(1, baseColor[2] * shade)
                            );
                        }

                        indices.push(
                            vertexIndex,
                            vertexIndex + 1,
                            vertexIndex + 2,
                            vertexIndex,
                            vertexIndex + 2,
                            vertexIndex + 3
                        );
                        vertexIndex += 4;
                    }
                }
            }
        }

        if (indices.length === 0) {
            return null;
        }

        const geometry = new THREE.BufferGeometry();
        geometry.setAttribute('position', new THREE.Float32BufferAttribute(positions, 3));
        geometry.setAttribute('normal', new THREE.Float32BufferAttribute(normals, 3));
        geometry.setAttribute('color', new THREE.Float32BufferAttribute(colors, 3));
        geometry.setIndex(indices);
        geometry.computeBoundingSphere();
        return geometry;
    }

    dispose() {
        this.material.dispose();
    }
}

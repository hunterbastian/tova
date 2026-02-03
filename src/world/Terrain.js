import * as THREE from 'three';

export class Terrain {
    constructor(scene, options = {}) {
        this.scene = scene;
        this.mesh = null;
        this.enableShadows = options.enableShadows ?? true;
        this.size = 500;
        this.segments = 128;
        this.step = this.size / this.segments;
        this.positionArray = null;
        this.valleyCenter = options.valleyCenter ?? 0;
        this.valleyWidth = options.valleyWidth ?? 140;
        this.valleyDepth = options.valleyDepth ?? 18;
        this.valleyAxis = options.valleyAxis === 'z' ? 'z' : 'x';

        this.init();
    }

    smoothstep(edge0, edge1, x) {
        const t = Math.min(1, Math.max(0, (x - edge0) / (edge1 - edge0)));
        return t * t * (3 - 2 * t);
    }

    init() {
        // Create a large plane for the terrain
        const geometry = new THREE.PlaneGeometry(this.size, this.size, this.segments, this.segments);

        // Manipulate vertices to create a hill and some roughness
        const positionAttribute = geometry.attributes.position;
        const vertex = new THREE.Vector3();

        for (let i = 0; i < positionAttribute.count; i++) {
            vertex.fromBufferAttribute(positionAttribute, i);

            // Simple procedural generation
            // Hill in the center (where the castle will be)
            const dist = Math.sqrt(vertex.x * vertex.x + vertex.y * vertex.y); // y is actually z in 2D plane before rotation

            let height = 0;

            // Central Hill
            if (dist < 80) {
                height += 40 * Math.cos((dist / 80) * (Math.PI / 2));
            }

            // Noise/Roughness (Simulated with Math.sin for now, can use a noise library later)
            height += Math.sin(vertex.x * 0.1) * Math.sin(vertex.y * 0.1) * 2;
            height += Math.sin(vertex.x * 0.5) * Math.sin(vertex.y * 0.5) * 0.5;

            // Flatten area for town near the hill
            if (vertex.x > 20 && vertex.x < 100 && vertex.y > -50 && vertex.y < 50) {
                // Smooth out the town area
                height *= 0.5;
            }

            // Ocean drop-off
            if (vertex.x < -100) {
                height -= 20; // Drop down for ocean
            }

            // Glacial valley carve (U-shaped profile)
            const axisCoord = this.valleyAxis === 'z' ? vertex.y : vertex.x;
            const valleyDistance = Math.abs(axisCoord - this.valleyCenter);
            const valleyBlend = 1 - this.smoothstep(this.valleyWidth * 0.2, this.valleyWidth, valleyDistance);
            if (valleyBlend > 0) {
                const uShape = 1 - Math.pow(valleyDistance / this.valleyWidth, 2);
                height -= this.valleyDepth * valleyBlend * Math.max(0, uShape);
                if (valleyDistance > this.valleyWidth * 0.35) {
                    height += this.valleyDepth * 0.12 * valleyBlend;
                }
                const floorBlend = 1 - this.smoothstep(0, this.valleyWidth * 0.25, valleyDistance);
                height = THREE.MathUtils.lerp(height, height * 0.35, floorBlend * 0.4);
            }

            // Quantize height a bit to give a more "stepped" / voxel-like feeling.
            height = Math.round(height / 2) * 2;

            positionAttribute.setZ(i, height); // Set Z because PlaneGeometry is XY by default
        }

        geometry.computeVertexNormals();

        // Material tuned for a more voxel-like, flat-shaded look.
        const material = new THREE.MeshStandardMaterial({
            color: 0x3a5f0b,
            roughness: 0.9,
            metalness: 0.0,
            flatShading: true
        });

        this.mesh = new THREE.Mesh(geometry, material);
        this.mesh.rotation.x = -Math.PI / 2;
        this.mesh.receiveShadow = this.enableShadows;
        this.mesh.castShadow = this.enableShadows;
        // Enable a dedicated lighting layer for terrain-only fill light.
        this.mesh.layers.enable(1);
        this.mesh.matrixAutoUpdate = false;
        this.mesh.updateMatrix();

        this.scene.add(this.mesh);

        this.positionArray = geometry.attributes.position.array;
    }

    getHeightAt(x, z) {
        const half = this.size / 2;
        const localX = x + half;
        const localZ = z + half;

        if (localX < 0 || localZ < 0 || localX > this.size || localZ > this.size) {
            return 0;
        }

        const gridX = localX / this.step;
        const gridZ = localZ / this.step;
        const ix = Math.floor(gridX);
        const iz = Math.floor(gridZ);
        const fx = gridX - ix;
        const fz = gridZ - iz;

        const vertsPerRow = this.segments + 1;
        const ix1 = Math.min(ix + 1, this.segments);
        const iz1 = Math.min(iz + 1, this.segments);

        const i00 = ix + vertsPerRow * iz;
        const i10 = ix1 + vertsPerRow * iz;
        const i01 = ix + vertsPerRow * iz1;
        const i11 = ix1 + vertsPerRow * iz1;

        const z00 = this.positionArray[i00 * 3 + 2];
        const z10 = this.positionArray[i10 * 3 + 2];
        const z01 = this.positionArray[i01 * 3 + 2];
        const z11 = this.positionArray[i11 * 3 + 2];

        const z0 = z00 * (1 - fx) + z10 * fx;
        const z1 = z01 * (1 - fx) + z11 * fx;

        return z0 * (1 - fz) + z1 * fz;
    }
}

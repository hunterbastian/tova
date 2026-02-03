import * as THREE from 'three';

const mulberry32 = (seed) => {
    let t = seed;
    return () => {
        t += 0x6d2b79f5;
        let r = Math.imul(t ^ (t >>> 15), 1 | t);
        r ^= r + Math.imul(r ^ (r >>> 7), 61 | r);
        return ((r ^ (r >>> 14)) >>> 0) / 4294967296;
    };
};

const clamp = (value, min, max) => Math.min(max, Math.max(min, value));

const lerpColor = (a, b, t) => a.clone().lerp(b, t);

export class Mountains {
    constructor(scene, options = {}) {
        this.scene = scene;
        this.group = new THREE.Group();
        this.layers = options.layers ?? 3;
        this.radius = options.radius ?? 260;
        this.height = options.height ?? 180;
        this.segments = options.segments ?? 200;
        this.seed = options.seed ?? 7381;
        this.haze = options.haze ?? 0.12;
        this.baseLift = options.baseLift ?? 6;
        this.jaggedness = options.jaggedness ?? 0.35;
        this.snowlineStart = options.snowlineStart ?? 0.58;
        this.snowlineFade = options.snowlineFade ?? 0.18;

        this.init();
    }

    init() {
        for (let i = 0; i < this.layers; i += 1) {
            const layerSeed = this.seed + i * 97;
            const radius = this.radius + i * 55;
            const height = this.height + i * 65;
            const opacity = 1 - i * this.haze;

            const layer = this.createRidgeLayer({
                radius,
                height,
                seed: layerSeed,
                opacity,
                snowBias: i / (this.layers - 1 || 1)
            });
            this.group.add(layer);
        }

        this.group.frustumCulled = false;
        this.scene.add(this.group);
    }

    createRidgeLayer({ radius, height, seed, opacity, snowBias }) {
        const rng = mulberry32(seed);
        const segments = this.segments;
        const vertexCount = (segments + 1) * 2;
        const positions = new Float32Array(vertexCount * 3);
        const colors = new Float32Array(vertexCount * 3);
        const indices = [];

        const green = new THREE.Color(0x3a6b45);
        const rock = new THREE.Color(0x4f5e6b);
        const snow = new THREE.Color(0xe7f5ff);
        const tint = lerpColor(new THREE.Color(0xffffff), new THREE.Color(0xc2d7ef), snowBias * 0.75);

        for (let i = 0; i <= segments; i += 1) {
            const t = i / segments;
            const angle = t * Math.PI * 2;
            const noise =
                Math.sin(t * Math.PI * 4 + rng() * 2) * 0.35 +
                Math.sin(t * Math.PI * 10 + rng() * 4) * 0.22 +
                Math.sin(t * Math.PI * 22 + rng() * 6) * (0.12 + this.jaggedness * 0.18) +
                (rng() - 0.5) * (0.22 + this.jaggedness * 0.2);
            const clampedNoise = clamp(noise, -0.35, 0.5);
            const jaggedBoost = 0.08 + this.jaggedness * 0.22;
            const ridgeHeight = height * (0.6 + clampedNoise + Math.max(0, clampedNoise) * jaggedBoost);
            const ridgeRadius = radius + (rng() - 0.5) * 12;

            const x = Math.cos(angle) * ridgeRadius;
            const z = Math.sin(angle) * ridgeRadius;

            const bottomIndex = i * 2;
            const topIndex = i * 2 + 1;

            positions[bottomIndex * 3] = x;
            positions[bottomIndex * 3 + 1] = this.baseLift;
            positions[bottomIndex * 3 + 2] = z;

            positions[topIndex * 3] = x;
            positions[topIndex * 3 + 1] = ridgeHeight + this.baseLift;
            positions[topIndex * 3 + 2] = z;

            const heightT = clamp(ridgeHeight / height, 0, 1);
            let topColor = green.clone();
            const rockStart = Math.max(0.35, this.snowlineStart - 0.28);
            const rockFade = 0.32;
            if (heightT > rockStart) {
                topColor = lerpColor(green, rock, (heightT - rockStart) / rockFade);
            }
            if (heightT > this.snowlineStart) {
                topColor = lerpColor(
                    rock,
                    snow,
                    (heightT - this.snowlineStart) / Math.max(0.08, this.snowlineFade)
                );
            }
            topColor.multiply(tint);

            const bottomColor = lerpColor(green, rock, snowBias * 0.25).multiply(tint);

            colors[bottomIndex * 3] = bottomColor.r;
            colors[bottomIndex * 3 + 1] = bottomColor.g;
            colors[bottomIndex * 3 + 2] = bottomColor.b;

            colors[topIndex * 3] = topColor.r;
            colors[topIndex * 3 + 1] = topColor.g;
            colors[topIndex * 3 + 2] = topColor.b;

            if (i < segments) {
                const a = bottomIndex;
                const b = bottomIndex + 2;
                const c = topIndex;
                const d = topIndex + 2;
                indices.push(a, c, b);
                indices.push(c, d, b);
            }
        }

        const geometry = new THREE.BufferGeometry();
        geometry.setAttribute('position', new THREE.BufferAttribute(positions, 3));
        geometry.setAttribute('color', new THREE.BufferAttribute(colors, 3));
        geometry.setIndex(indices);
        geometry.computeVertexNormals();

        const material = new THREE.MeshStandardMaterial({
            vertexColors: true,
            roughness: 0.95,
            metalness: 0.0,
            flatShading: true,
            transparent: opacity < 1,
            opacity,
            side: THREE.DoubleSide
        });

        const mesh = new THREE.Mesh(geometry, material);
        mesh.castShadow = false;
        mesh.receiveShadow = false;
        mesh.frustumCulled = false;
        return mesh;
    }

    update(playerPosition) {
        if (!playerPosition) return;
        this.group.position.set(playerPosition.x, 0, playerPosition.z);
    }
}

import { BIOME, BLEND_RING_SIZE, CORE_HALF_SIZE, SEA_LEVEL } from './constants.js';

const clamp = (value, min, max) => Math.min(max, Math.max(min, value));
const lerp = (a, b, t) => a + (b - a) * t;
const smoothstep = (edge0, edge1, x) => {
    const t = clamp((x - edge0) / (edge1 - edge0), 0, 1);
    return t * t * (3 - 2 * t);
};

const hash2D = (x, z, seed) => {
    let h = Math.imul(x, 374761393) ^ Math.imul(z, 668265263) ^ seed;
    h = (h ^ (h >>> 13)) >>> 0;
    h = Math.imul(h, 1274126177) >>> 0;
    return ((h ^ (h >>> 16)) >>> 0) / 4294967295;
};

const valueNoise2D = (x, z, seed) => {
    const xi = Math.floor(x);
    const zi = Math.floor(z);
    const xf = x - xi;
    const zf = z - zi;

    const v00 = hash2D(xi, zi, seed);
    const v10 = hash2D(xi + 1, zi, seed);
    const v01 = hash2D(xi, zi + 1, seed);
    const v11 = hash2D(xi + 1, zi + 1, seed);

    const u = smoothstep(0, 1, xf);
    const v = smoothstep(0, 1, zf);

    const x0 = lerp(v00, v10, u);
    const x1 = lerp(v01, v11, u);
    return lerp(x0, x1, v);
};

const fbm = (x, z, seed, octaves = 4, lacunarity = 2, gain = 0.5) => {
    let amplitude = 1;
    let frequency = 1;
    let total = 0;
    let norm = 0;

    for (let i = 0; i < octaves; i += 1) {
        total += valueNoise2D(x * frequency, z * frequency, seed + i * 1013) * amplitude;
        norm += amplitude;
        amplitude *= gain;
        frequency *= lacunarity;
    }

    return norm > 0 ? total / norm : 0;
};

const ridged = (x, z, seed, octaves = 4) => {
    let amplitude = 0.9;
    let frequency = 1;
    let total = 0;
    let norm = 0;

    for (let i = 0; i < octaves; i += 1) {
        const n = valueNoise2D(x * frequency, z * frequency, seed + i * 911) * 2 - 1;
        const r = 1 - Math.abs(n);
        total += r * r * amplitude;
        norm += amplitude;
        amplitude *= 0.58;
        frequency *= 1.95;
    }

    return norm > 0 ? total / norm : 0;
};

const distanceToSegment = (px, pz, ax, az, bx, bz) => {
    const abx = bx - ax;
    const abz = bz - az;
    const apx = px - ax;
    const apz = pz - az;
    const denom = abx * abx + abz * abz;
    if (denom === 0) {
        return Math.hypot(px - ax, pz - az);
    }
    const t = clamp((apx * abx + apz * abz) / denom, 0, 1);
    const qx = ax + abx * t;
    const qz = az + abz * t;
    return Math.hypot(px - qx, pz - qz);
};

/**
 * @interface TerrainField
 */
export class TerrainField {
    sampleHeight(_x, _z) {
        throw new Error('sampleHeight must be implemented');
    }

    sampleBiome(_x, _z) {
        throw new Error('sampleBiome must be implemented');
    }
}

/**
 * @implements {TerrainField}
 */
export class AuthoredField extends TerrainField {
    constructor(options = {}) {
        super();
        this.seed = options.seed ?? 133742;
        this.coreHalfSize = options.coreHalfSize ?? CORE_HALF_SIZE;
        this.blendRingSize = options.blendRingSize ?? BLEND_RING_SIZE;
        this.castleCenter = { x: 0, z: 0 };
        this.townCenter = { x: 240, z: 44 };
    }

    isInsideCore(x, z) {
        return Math.abs(x) <= this.coreHalfSize && Math.abs(z) <= this.coreHalfSize;
    }

    blendFactor(x, z) {
        const edgeDistance = Math.max(Math.abs(x), Math.abs(z));
        if (edgeDistance <= this.coreHalfSize) {
            return 0;
        }
        if (edgeDistance >= this.coreHalfSize + this.blendRingSize) {
            return 1;
        }
        return smoothstep(this.coreHalfSize, this.coreHalfSize + this.blendRingSize, edgeDistance);
    }

    sampleProceduralHeight(x, z) {
        const rolling = 45 + fbm(x * 0.0011, z * 0.0011, this.seed + 17, 5, 2.02, 0.5) * 28;
        const ridgeMask = smoothstep(0.42, 0.82, fbm(x * 0.00032 + 9.1, z * 0.00032 - 3.4, this.seed + 59, 3, 2.1, 0.53));
        const mountainRidges = ridged(x * 0.00057, z * 0.00057, this.seed + 211, 4) * 58 * ridgeMask;
        const detail = (fbm(x * 0.006, z * 0.006, this.seed + 401, 2, 2, 0.55) - 0.5) * 4;
        return clamp(rolling + mountainRidges + detail, 2, 110);
    }

    sampleAuthoredHeight(x, z) {
        const procedural = this.sampleProceduralHeight(x, z);
        const centerDist = Math.hypot(x, z);
        const coreFlatten = smoothstep(0, 1500, centerDist);

        let height = lerp(56, procedural, coreFlatten * 0.75);

        const castleDist = Math.hypot(x - this.castleCenter.x, z - this.castleCenter.z);
        if (castleDist < 190) {
            const t = 1 - castleDist / 190;
            height += Math.pow(t, 1.7) * 34;
        }

        const townDist = Math.hypot(x - this.townCenter.x, z - this.townCenter.z);
        if (townDist < 185) {
            const t = smoothstep(185, 0, townDist);
            height = lerp(height, 54, t * 0.72);
        }

        const roadDistance = distanceToSegment(
            x,
            z,
            this.castleCenter.x,
            this.castleCenter.z,
            this.townCenter.x,
            this.townCenter.z
        );
        if (roadDistance < 24) {
            const roadBlend = smoothstep(24, 0, roadDistance);
            height = lerp(height, 51, roadBlend * 0.45);
        }

        return clamp(height, 2, 108);
    }

    sampleHeight(x, z) {
        const blend = this.blendFactor(x, z);
        const authored = this.sampleAuthoredHeight(x, z);
        const procedural = this.sampleProceduralHeight(x, z);
        const base = blend >= 1 ? procedural : lerp(authored, procedural, blend);

        const micro = (fbm(x * 0.008, z * 0.008, this.seed + 607, 2, 2, 0.55) - 0.5) * (2 + blend * 2);
        return Math.round(clamp(base + micro, 2, 110));
    }

    sampleBiome(x, z) {
        const height = this.sampleHeight(x, z);
        if (height <= SEA_LEVEL + 2) {
            return BIOME.COAST;
        }

        const moisture = fbm(x * 0.00145 + 7.5, z * 0.00145 - 2.4, this.seed + 733, 3, 2, 0.52);
        if (height > 82 || moisture < 0.33) {
            return BIOME.ALPINE;
        }
        if (moisture > 0.65) {
            return BIOME.FOREST;
        }
        return BIOME.PLAINS;
    }

    getStructureTag(x, z) {
        const castleDist = Math.hypot(x - this.castleCenter.x, z - this.castleCenter.z);
        if (castleDist < 58) {
            return 'castle';
        }

        const townDist = Math.hypot(x - this.townCenter.x, z - this.townCenter.z);
        if (townDist < 78) {
            return 'town';
        }

        const roadDistance = distanceToSegment(
            x,
            z,
            this.castleCenter.x,
            this.castleCenter.z,
            this.townCenter.x,
            this.townCenter.z
        );
        if (roadDistance < 10) {
            return 'road';
        }

        return null;
    }
}

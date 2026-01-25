import { ShaderPass } from 'three/examples/jsm/postprocessing/ShaderPass.js';

/**
 * DOOM-style pixelation effect.
 * Renders the scene at a lower effective resolution for that classic chunky look.
 */
const PixelationShader = {
    uniforms: {
        tDiffuse: { value: null },
        resolution: { value: null },
        pixelSize: { value: 4.0 }
    },

    vertexShader: /* glsl */`
        varying vec2 vUv;
        void main() {
            vUv = uv;
            gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
        }
    `,

    fragmentShader: /* glsl */`
        uniform sampler2D tDiffuse;
        uniform vec2 resolution;
        uniform float pixelSize;

        varying vec2 vUv;

        void main() {
            vec2 dxy = pixelSize / resolution;
            vec2 coord = dxy * floor(vUv / dxy);
            gl_FragColor = texture2D(tDiffuse, coord);
        }
    `
};

export class PixelationPass extends ShaderPass {
    constructor(pixelSize = 4.0) {
        super(PixelationShader);
        this.uniforms = this.material.uniforms;
        this.setPixelSize(pixelSize);
    }

    setPixelSize(size) {
        this.uniforms.pixelSize.value = size;
    }

    setSize(width, height) {
        this.uniforms.resolution.value = { x: width, y: height };
    }
}

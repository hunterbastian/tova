import * as THREE from "three";
import { EffectComposer } from "three/examples/jsm/postprocessing/EffectComposer.js";
import { RenderPixelatedPass } from "three/examples/jsm/postprocessing/RenderPixelatedPass.js";
import { ShaderPass } from "three/examples/jsm/postprocessing/ShaderPass.js";
import { OutputPass } from "three/examples/jsm/postprocessing/OutputPass.js";

const ColorBandingShader = {
  name: "ColorBandingShader",
  uniforms: {
    tDiffuse: { value: null },
    levels: { value: 10.0 },
  },
  vertexShader: /* glsl */ `
    varying vec2 vUv;
    void main() {
      vUv = uv;
      gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
    }
  `,
  fragmentShader: /* glsl */ `
    uniform sampler2D tDiffuse;
    uniform float levels;
    varying vec2 vUv;
    void main() {
      vec4 color = texture2D(tDiffuse, vUv);
      color.rgb = floor(color.rgb * levels + 0.5) / levels;
      gl_FragColor = color;
    }
  `,
};

export function createShaderPipeline({ renderer, scene, camera, safeMode }) {
  if (safeMode) {
    return {
      render: () => renderer.render(scene, camera),
      setSize: (w, h) => renderer.setSize(w, h),
    };
  }

  const composer = new EffectComposer(renderer);

  const pixelatedPass = new RenderPixelatedPass(6, scene, camera, {
    normalEdgeStrength: 0.2,
    depthEdgeStrength: 0.35,
  });
  composer.addPass(pixelatedPass);

  const bandingPass = new ShaderPass(ColorBandingShader);
  bandingPass.uniforms.levels.value = 7.0;
  composer.addPass(bandingPass);

  const outputPass = new OutputPass();
  composer.addPass(outputPass);

  return {
    render: () => composer.render(),
    setSize: (w, h) => composer.setSize(w, h),
    composer,
    pixelatedPass,
    bandingPass,
  };
}

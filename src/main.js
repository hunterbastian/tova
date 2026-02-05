import * as THREE from 'three';
import { Environment } from './world/Environment.js';
import { Terrain } from './world/Terrain.js';
import { Ocean } from './world/Ocean.js';
import { Mountains } from './world/Mountains.js';
import { Forest } from './world/Forest.js';
import { Castle } from './structures/Castle.js';
import { Town } from './structures/Town.js';
import { Player } from './controls/Player.js';
import ambientTrackUrl from './assets/fantasy-medieval-ambient.mp3';

const parseNumber = (value, fallback) => {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
};

const readFiniteParam = (paramsRef, key) => {
    const raw = paramsRef.get(key);
    if (raw === null) return null;
    const parsed = Number(raw);
    return Number.isFinite(parsed) ? parsed : null;
};

const params = new URLSearchParams(window.location.search);
const bloomParam = params.get('bloom');
const shadowsParam = params.get('shadows');
const grainParam = params.get('grain');
const vignetteParam = params.get('vignette');
const sharedMomentState = {
    isShared: params.get('shared') === '1',
    x: readFiniteParam(params, 'px'),
    y: readFiniteParam(params, 'py'),
    z: readFiniteParam(params, 'pz'),
    yaw: readFiniteParam(params, 'yaw'),
    pitch: readFiniteParam(params, 'pitch'),
    tod: params.get('tod')
};
const perf = {
    // Default to "pretty" unless explicitly turned off by query params.
    maxPixelRatio: Math.min(2, Math.max(0.75, parseNumber(params.get('pixelRatio'), 1.5))),
    enableBloom: bloomParam === null ? true : bloomParam === '1',
    enableShadows: shadowsParam === null ? true : shadowsParam !== '0',
    bloomResolutionScale: Math.min(1, Math.max(0.25, parseNumber(params.get('bloomScale'), 0.7)))
};

// Basic Scene Setup
const scene = new THREE.Scene();

const camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
camera.position.set(0, 20, 50); // Higher starting position to see the terrain
camera.layers.enable(1);

// Lower default quality for smoother performance on typical Macs.
// Antialiasing is disabled and pixel ratio is capped unless overridden by URL params.
const renderer = new THREE.WebGLRenderer({ antialias: false, powerPreference: 'high-performance' });
renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, perf.maxPixelRatio));
renderer.setSize(window.innerWidth, window.innerHeight);
renderer.shadowMap.enabled = perf.enableShadows;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 0.9; // Brighter, Sildur-like daylight
document.body.appendChild(renderer.domElement);

const crosshair = document.createElement('div');
crosshair.style.position = 'fixed';
crosshair.style.left = '50%';
crosshair.style.top = '50%';
crosshair.style.transform = 'translate(-50%, -50%)';
crosshair.style.width = '14px';
crosshair.style.height = '14px';
crosshair.style.pointerEvents = 'none';
crosshair.style.zIndex = '2';
crosshair.style.opacity = '0.6';

const crosshairVertical = document.createElement('div');
crosshairVertical.style.position = 'absolute';
crosshairVertical.style.left = '50%';
crosshairVertical.style.top = '0';
crosshairVertical.style.width = '1px';
crosshairVertical.style.height = '14px';
crosshairVertical.style.background = 'rgba(240, 243, 255, 0.7)';
crosshairVertical.style.transform = 'translateX(-50%)';

const crosshairHorizontal = document.createElement('div');
crosshairHorizontal.style.position = 'absolute';
crosshairHorizontal.style.left = '0';
crosshairHorizontal.style.top = '50%';
crosshairHorizontal.style.width = '14px';
crosshairHorizontal.style.height = '1px';
crosshairHorizontal.style.background = 'rgba(240, 243, 255, 0.7)';
crosshairHorizontal.style.transform = 'translateY(-50%)';

crosshair.appendChild(crosshairVertical);
crosshair.appendChild(crosshairHorizontal);
document.body.appendChild(crosshair);

const uiLayer = document.createElement('div');
uiLayer.style.position = 'fixed';
uiLayer.style.inset = '0';
uiLayer.style.pointerEvents = 'none';
uiLayer.style.zIndex = '1';
document.body.appendChild(uiLayer);

const trackEvent = (name, properties = {}) => {
    const payload = { name, properties, ts: Date.now() };
    try {
        if (typeof window.gtag === 'function') {
            window.gtag('event', name, properties);
        }
    } catch (error) {
        console.debug('Analytics gtag error', error);
    }
    try {
        if (typeof window.plausible === 'function') {
            window.plausible(name, { props: properties });
        }
    } catch (error) {
        console.debug('Analytics plausible error', error);
    }
    try {
        if (Array.isArray(window.dataLayer)) {
            window.dataLayer.push({ event: name, ...properties });
        }
    } catch (error) {
        console.debug('Analytics dataLayer error', error);
    }
    window.dispatchEvent(new CustomEvent('tova:analytics', { detail: payload }));
};

const toast = document.createElement('div');
toast.style.position = 'fixed';
toast.style.left = '50%';
toast.style.bottom = '18px';
toast.style.transform = 'translateX(-50%) translateY(8px)';
toast.style.padding = '8px 12px';
toast.style.borderRadius = '10px';
toast.style.border = '1px solid rgba(255, 255, 255, 0.2)';
toast.style.background = 'rgba(6, 10, 18, 0.78)';
toast.style.backdropFilter = 'blur(6px)';
toast.style.color = '#f0f3ff';
toast.style.fontFamily = 'serif';
toast.style.fontSize = '11px';
toast.style.letterSpacing = '0.08em';
toast.style.textTransform = 'uppercase';
toast.style.pointerEvents = 'none';
toast.style.opacity = '0';
toast.style.transition = 'opacity 180ms ease, transform 180ms ease';
toast.style.zIndex = '4';
document.body.appendChild(toast);

let toastTimeoutId = null;
const showToast = (message) => {
    toast.textContent = message;
    toast.style.opacity = '1';
    toast.style.transform = 'translateX(-50%) translateY(0)';
    if (toastTimeoutId) {
        window.clearTimeout(toastTimeoutId);
    }
    toastTimeoutId = window.setTimeout(() => {
        toast.style.opacity = '0';
        toast.style.transform = 'translateX(-50%) translateY(8px)';
    }, 1800);
};

const ambientToggle = document.createElement('button');
ambientToggle.type = 'button';
ambientToggle.textContent = 'Music On';
ambientToggle.style.position = 'fixed';
ambientToggle.style.right = '16px';
ambientToggle.style.bottom = '16px';
ambientToggle.style.padding = '6px 10px';
ambientToggle.style.borderRadius = '8px';
ambientToggle.style.border = '1px solid rgba(255, 255, 255, 0.2)';
ambientToggle.style.background = 'rgba(6, 10, 18, 0.45)';
ambientToggle.style.color = '#f0f3ff';
ambientToggle.style.fontFamily = 'serif';
ambientToggle.style.fontSize = '11px';
ambientToggle.style.letterSpacing = '0.12em';
ambientToggle.style.textTransform = 'uppercase';
ambientToggle.style.cursor = 'pointer';
ambientToggle.style.pointerEvents = 'auto';
ambientToggle.style.zIndex = '3';
document.body.appendChild(ambientToggle);

const shareMomentButton = document.createElement('button');
shareMomentButton.type = 'button';
shareMomentButton.textContent = 'Share Moment';
shareMomentButton.style.position = 'fixed';
shareMomentButton.style.right = '16px';
shareMomentButton.style.bottom = '52px';
shareMomentButton.style.padding = '6px 10px';
shareMomentButton.style.borderRadius = '8px';
shareMomentButton.style.border = '1px solid rgba(255, 255, 255, 0.2)';
shareMomentButton.style.background = 'rgba(6, 10, 18, 0.45)';
shareMomentButton.style.color = '#f0f3ff';
shareMomentButton.style.fontFamily = 'serif';
shareMomentButton.style.fontSize = '11px';
shareMomentButton.style.letterSpacing = '0.12em';
shareMomentButton.style.textTransform = 'uppercase';
shareMomentButton.style.cursor = 'pointer';
shareMomentButton.style.pointerEvents = 'auto';
shareMomentButton.style.zIndex = '3';
document.body.appendChild(shareMomentButton);

const createAmbientMusic = () => {
    const state = {
        context: null,
        master: null,
        enabled: true,
        started: false,
        buffer: null,
        source: null,
        loading: false
    };

    const createImpulse = (context, duration = 2.6, decay = 3.2) => {
        const rate = context.sampleRate;
        const length = Math.floor(rate * duration);
        const impulse = context.createBuffer(2, length, rate);
        for (let channel = 0; channel < impulse.numberOfChannels; channel += 1) {
            const data = impulse.getChannelData(channel);
            for (let i = 0; i < length; i += 1) {
                const t = i / length;
                data[i] = (Math.random() * 2 - 1) * Math.pow(1 - t, decay);
            }
        }
        return impulse;
    };

    const loadBuffer = async (context) => {
        if (state.buffer || state.loading) return;
        state.loading = true;
        const response = await fetch(ambientTrackUrl);
        const arrayBuffer = await response.arrayBuffer();
        state.buffer = await context.decodeAudioData(arrayBuffer);
        state.loading = false;
    };

    const start = async () => {
        if (state.started) return;
        const AudioCtx = window.AudioContext || window.webkitAudioContext;
        if (!AudioCtx) return;

        const context = new AudioCtx();
        const master = context.createGain();
        master.gain.value = 0;
        master.connect(context.destination);

        const filter = context.createBiquadFilter();
        filter.type = 'lowpass';
        filter.frequency.value = 1200;
        filter.Q.value = 0.7;

        const convolver = context.createConvolver();
        convolver.buffer = createImpulse(context);

        const wet = context.createGain();
        wet.gain.value = 0.55;
        const dry = context.createGain();
        dry.gain.value = 0.75;

        filter.connect(dry);
        filter.connect(convolver);
        convolver.connect(wet);
        wet.connect(master);
        dry.connect(master);

        await loadBuffer(context);
        if (!state.buffer) return;

        const source = context.createBufferSource();
        source.buffer = state.buffer;
        source.loop = true;
        source.playbackRate.value = 0.92;
        source.connect(filter);
        source.start();

        state.context = context;
        state.master = master;
        state.source = source;
        state.started = true;

        if (state.enabled) {
            master.gain.linearRampToValueAtTime(0.08, context.currentTime + 2.5);
        }
    };

    const setEnabled = (enabled) => {
        state.enabled = enabled;
        if (!state.started || !state.master || !state.context) return;
        const now = state.context.currentTime;
        state.master.gain.cancelScheduledValues(now);
        state.master.gain.setValueAtTime(state.master.gain.value, now);
        state.master.gain.linearRampToValueAtTime(enabled ? 0.08 : 0.0, now + 1.2);
    };

    return { start, setEnabled, state };
};

const ambientMusic = createAmbientMusic();

const updateAmbientLabel = () => {
    ambientToggle.textContent = ambientMusic.state.enabled ? 'Music On' : 'Music Off';
};

ambientToggle.addEventListener('click', () => {
    ambientMusic.state.enabled = !ambientMusic.state.enabled;
    ambientMusic.setEnabled(ambientMusic.state.enabled);
    updateAmbientLabel();
});

document.addEventListener('pointerdown', () => {
    ambientMusic.start();
    if (ambientMusic.state.context && ambientMusic.state.context.state === 'suspended') {
        ambientMusic.state.context.resume();
    }
}, { once: true });

const vignetteEnabled = vignetteParam !== '0';
if (vignetteEnabled) {
    const vignette = document.createElement('div');
    vignette.style.position = 'absolute';
    vignette.style.inset = '0';
    vignette.style.background = 'radial-gradient(circle at 50% 45%, rgba(0,0,0,0) 0%, rgba(0,0,0,0.18) 65%, rgba(0,0,0,0.45) 100%)';
    vignette.style.mixBlendMode = 'multiply';
    vignette.style.opacity = '0.6';
    uiLayer.appendChild(vignette);
}

const grainEnabled = grainParam !== '0';
if (grainEnabled) {
    const grainCanvas = document.createElement('canvas');
    grainCanvas.width = 128;
    grainCanvas.height = 128;
    const grainCtx = grainCanvas.getContext('2d');
    const grainImage = grainCtx.createImageData(grainCanvas.width, grainCanvas.height);
    const grainData = grainImage.data;

    for (let i = 0; i < grainData.length; i += 4) {
        const value = Math.random() * 255;
        grainData[i] = value;
        grainData[i + 1] = value;
        grainData[i + 2] = value;
        grainData[i + 3] = 22;
    }
    grainCtx.putImageData(grainImage, 0, 0);

    const grainLayer = document.createElement('div');
    grainLayer.style.position = 'absolute';
    grainLayer.style.inset = '0';
    grainLayer.style.opacity = '0.06';
    grainLayer.style.imageRendering = 'pixelated';
    grainLayer.style.mixBlendMode = 'soft-light';
    grainLayer.style.backgroundImage = `url(${grainCanvas.toDataURL('image/png')})`;
    grainLayer.style.backgroundSize = '128px 128px';
    grainLayer.style.animation = 'grainShift 1.4s steps(1) infinite';
    uiLayer.appendChild(grainLayer);

    const grainStyle = document.createElement('style');
    grainStyle.textContent = `
        @keyframes grainShift {
            0% { background-position: 0 0; }
            25% { background-position: 20px -30px; }
            50% { background-position: -20px 20px; }
            75% { background-position: 30px 10px; }
            100% { background-position: 0 0; }
        }
    `;
    document.head.appendChild(grainStyle);
}

const timeBar = document.createElement('div');
timeBar.style.position = 'absolute';
timeBar.style.top = '16px';
timeBar.style.left = '50%';
timeBar.style.transform = 'translateX(-50%)';
timeBar.style.width = '170px';
timeBar.style.padding = '4px 6px';
timeBar.style.borderRadius = '6px';
timeBar.style.background = 'rgba(6, 10, 18, 0.16)';
timeBar.style.backdropFilter = 'blur(4px)';
timeBar.style.fontFamily = 'serif';
timeBar.style.color = '#f0f3ff';
timeBar.style.fontSize = '10px';
timeBar.style.letterSpacing = '0.14em';
timeBar.style.textTransform = 'uppercase';
timeBar.style.textAlign = 'center';
timeBar.style.pointerEvents = 'none';
timeBar.style.zIndex = '2';

const timeBarLabel = document.createElement('div');
timeBarLabel.style.display = 'flex';
timeBarLabel.style.alignItems = 'center';
timeBarLabel.style.justifyContent = 'center';
timeBarLabel.style.gap = '6px';

const timeIcon = document.createElement('span');
timeIcon.style.display = 'inline-flex';
timeIcon.style.alignItems = 'center';
timeIcon.style.justifyContent = 'center';
timeIcon.style.width = '12px';
timeIcon.style.height = '12px';

timeBarLabel.appendChild(timeIcon);

const timeBarTrack = document.createElement('div');
timeBarTrack.style.marginTop = '2px';
timeBarTrack.style.height = '2px';
timeBarTrack.style.borderRadius = '999px';
timeBarTrack.style.background = 'rgba(255, 255, 255, 0.12)';
timeBarTrack.style.overflow = 'hidden';

const timeBarFill = document.createElement('div');
timeBarFill.style.height = '100%';
timeBarFill.style.width = '50%';
timeBarFill.style.background = 'linear-gradient(90deg, #f7b46a, #fff1d0)';
timeBarFill.style.transition = 'width 0.2s linear';
timeBarFill.style.borderRadius = '999px';

timeBarTrack.appendChild(timeBarFill);
timeBar.appendChild(timeBarLabel);
timeBar.appendChild(timeBarTrack);
document.body.appendChild(timeBar);

const stats = document.createElement('div');
stats.style.position = 'absolute';
stats.style.top = '16px';
stats.style.left = '16px';
stats.style.display = 'flex';
stats.style.flexDirection = 'column';
stats.style.gap = '4px';
stats.style.padding = '6px 8px';
stats.style.borderRadius = '8px';
stats.style.background = 'rgba(6, 10, 18, 0.2)';
stats.style.backdropFilter = 'blur(6px)';
stats.style.fontFamily = 'monospace';
stats.style.color = '#dfe7ff';
stats.style.fontSize = '11px';
stats.style.letterSpacing = '0.06em';
stats.style.pointerEvents = 'none';
stats.style.zIndex = '2';

const fpsLabel = document.createElement('div');
fpsLabel.textContent = 'FPS 60';

const coordsLabel = document.createElement('div');
coordsLabel.textContent = 'X 0.0 Y 0.0 Z 0.0';

stats.appendChild(fpsLabel);
stats.appendChild(coordsLabel);
document.body.appendChild(stats);

const hud = document.createElement('div');
hud.style.position = 'absolute';
hud.style.top = '16px';
hud.style.right = '16px';
hud.style.display = 'flex';
hud.style.gap = '8px';
hud.style.padding = '6px 8px';
hud.style.borderRadius = '10px';
hud.style.background = 'rgba(6, 10, 18, 0.16)';
hud.style.backdropFilter = 'blur(4px)';
hud.style.fontFamily = 'serif';
hud.style.color = '#f0f3ff';
hud.style.fontSize = '11px';
hud.style.letterSpacing = '0.04em';
hud.style.textTransform = 'uppercase';
hud.style.pointerEvents = 'none';
hud.style.zIndex = '2';

const makeHudItem = (iconSvg) => {
    const item = document.createElement('div');
    item.style.display = 'flex';
    item.style.alignItems = 'center';

    const icon = document.createElement('span');
    icon.style.display = 'inline-flex';
    icon.style.width = '12px';
    icon.style.height = '12px';
    icon.innerHTML = iconSvg;

    item.appendChild(icon);
    return item;
};

const iconKeyboard = `
<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="#f0f3ff" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <rect x="3" y="6" width="18" height="12" rx="2"></rect>
  <path d="M7 10h1M10 10h1M13 10h1M16 10h1M7 13h3M11 13h3M15 13h2"></path>
</svg>`;

const iconMouse = `
<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="#f0f3ff" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <rect x="8" y="3" width="8" height="18" rx="4"></rect>
  <path d="M12 7v3"></path>
</svg>`;

const iconSlash = `
<svg viewBox="0 0 24 24" width="12" height="12" fill="none" stroke="#f0f3ff" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <path d="M7 20L17 4"></path>
</svg>`;

hud.appendChild(makeHudItem(iconKeyboard));
hud.appendChild(makeHudItem(iconMouse));
hud.appendChild(makeHudItem(iconSlash));
document.body.appendChild(hud);

const sunSvg = `
<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#fff1d0" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <circle cx="12" cy="12" r="4"></circle>
  <path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.5 4.5l2.2 2.2M17.3 17.3l2.2 2.2M4.5 19.5l2.2-2.2M17.3 6.7l2.2-2.2"></path>
</svg>`;

const moonSvg = `
<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#a0b6ff" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <path d="M14 3c-1.5 1.6-2.4 3.7-2.4 6.1 0 4.9 4 8.9 8.9 8.9 1.2 0 2.4-.3 3.5-.7-1.9 2.7-5 4.5-8.6 4.5-5.8 0-10.5-4.7-10.5-10.5 0-4.2 2.5-7.8 6.1-9.3z"></path>
</svg>`;

const setTimeIcon = (isDay) => {
    timeIcon.innerHTML = isDay ? sunSvg : moonSvg;
};

setTimeIcon(true);

// Initialize World Components
const environment = new Environment(scene, { enableShadows: perf.enableShadows, dayLength: 120, nightLength: 120 });
const terrain = new Terrain(scene, { enableShadows: perf.enableShadows });
const ocean = new Ocean(scene);
const mountains = new Mountains(scene, { radius: 420, height: 210, layers: 4, haze: 0.08, baseLift: 12 });
const forest = new Forest(scene, terrain, { enableShadows: perf.enableShadows });

const castle = new Castle(scene, terrain, { enableShadows: perf.enableShadows });
const town = new Town(scene, terrain, { enableShadows: perf.enableShadows });

if (perf.enableShadows) {
    renderer.shadowMap.autoUpdate = false;
    renderer.shadowMap.needsUpdate = true;
}

let composer = null;
let bloomPass = null;
let colorGradePass = null;
let renderFrame = () => renderer.render(scene, camera);

const createSunGlowTexture = () => {
    const size = 128;
    const canvas = document.createElement('canvas');
    canvas.width = size;
    canvas.height = size;
    const ctx = canvas.getContext('2d');
    const gradient = ctx.createRadialGradient(size / 2, size / 2, 0, size / 2, size / 2, size / 2);
    gradient.addColorStop(0, 'rgba(255, 255, 255, 1)');
    gradient.addColorStop(0.35, 'rgba(255, 220, 170, 0.7)');
    gradient.addColorStop(0.7, 'rgba(255, 190, 120, 0.25)');
    gradient.addColorStop(1, 'rgba(255, 190, 120, 0)');
    ctx.fillStyle = gradient;
    ctx.fillRect(0, 0, size, size);
    const texture = new THREE.CanvasTexture(canvas);
    texture.minFilter = THREE.LinearFilter;
    texture.magFilter = THREE.LinearFilter;
    return texture;
};

const sunGlow = new THREE.Sprite(
    new THREE.SpriteMaterial({
        map: createSunGlowTexture(),
        color: 0xfff0d4,
        transparent: true,
        blending: THREE.AdditiveBlending,
        depthWrite: false
    })
);
sunGlow.scale.set(38, 38, 1);
sunGlow.renderOrder = 2;
scene.add(sunGlow);

const initPostprocessing = async () => {
    const [{ EffectComposer }, { RenderPass }, { UnrealBloomPass }, { OutputPass }, { ShaderPass }] = await Promise.all([
        import('three/examples/jsm/postprocessing/EffectComposer.js'),
        import('three/examples/jsm/postprocessing/RenderPass.js'),
        import('three/examples/jsm/postprocessing/UnrealBloomPass.js'),
        import('three/examples/jsm/postprocessing/OutputPass.js'),
        import('three/examples/jsm/postprocessing/ShaderPass.js')
    ]);

    composer = new EffectComposer(renderer);
    composer.setPixelRatio(Math.min(window.devicePixelRatio || 1, perf.maxPixelRatio));
    composer.setSize(window.innerWidth, window.innerHeight);

    const renderPass = new RenderPass(scene, camera);
    composer.addPass(renderPass);

    colorGradePass = new ShaderPass({
        uniforms: {
            tDiffuse: { value: null },
            lift: { value: new THREE.Vector3(0.005, 0.008, 0.015) },
            gamma: { value: new THREE.Vector3(0.99, 0.99, 1.0) },
            gain: { value: new THREE.Vector3(1.03, 1.02, 1.01) }
        },
        vertexShader: `
            varying vec2 vUv;
            void main() {
                vUv = uv;
                gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
            }
        `,
        fragmentShader: `
            uniform sampler2D tDiffuse;
            uniform vec3 lift;
            uniform vec3 gamma;
            uniform vec3 gain;
            varying vec2 vUv;
            void main() {
                vec4 color = texture2D(tDiffuse, vUv);
                vec3 graded = color.rgb + lift;
                graded = max(graded, vec3(0.0));
                graded = pow(graded, gamma);
                graded *= gain;
                gl_FragColor = vec4(graded, color.a);
            }
        `
    });
    composer.addPass(colorGradePass);

    if (perf.enableBloom) {
        const bloomSize = new THREE.Vector2(
            window.innerWidth * perf.bloomResolutionScale,
            window.innerHeight * perf.bloomResolutionScale
        );
        bloomPass = new UnrealBloomPass(bloomSize, 1.2, 0.4, 0.85);
        bloomPass.threshold = 0.3;
        bloomPass.strength = 0.35;
        bloomPass.radius = 0.4;
        composer.addPass(bloomPass);
    }

    const outputPass = new OutputPass();
    composer.addPass(outputPass);

    renderFrame = () => composer.render();
};

initPostprocessing();

// Resize Handler
window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, perf.maxPixelRatio));
    if (composer) {
        composer.setSize(window.innerWidth, window.innerHeight);
        composer.setPixelRatio(Math.min(window.devicePixelRatio || 1, perf.maxPixelRatio));
    }
    if (bloomPass) {
        bloomPass.setSize(
            window.innerWidth * perf.bloomResolutionScale,
            window.innerHeight * perf.bloomResolutionScale
        );
    }
});

const player = new Player(scene, camera, terrain, {
    onCommand: (cmd) => {
        if (cmd === 'day') {
            environment.setOverrideMode('day');
        } else if (cmd === 'night') {
            environment.setOverrideMode('night');
        }
    }
});

const getCurrentViewAngles = () => {
    const direction = new THREE.Vector3();
    camera.getWorldDirection(direction);
    return {
        yaw: Math.atan2(direction.x, direction.z),
        pitch: Math.asin(THREE.MathUtils.clamp(direction.y, -1, 1))
    };
};

const getCurrentTod = (cycle) => {
    if (environment.overrideMode === 'day' || environment.overrideMode === 'night') {
        return environment.overrideMode;
    }
    if (cycle) {
        return cycle.isDay ? 'day' : 'night';
    }
    return null;
};

const applySharedMomentFromUrl = () => {
    const hasPosition = Number.isFinite(sharedMomentState.x) && Number.isFinite(sharedMomentState.z);
    if (hasPosition) {
        player.playerObject.position.x = sharedMomentState.x;
        player.playerObject.position.z = sharedMomentState.z;
        if (Number.isFinite(sharedMomentState.y)) {
            player.playerObject.position.y = sharedMomentState.y;
        } else {
            player.alignToTerrain(0);
        }
    }

    if (Number.isFinite(sharedMomentState.yaw) || Number.isFinite(sharedMomentState.pitch)) {
        const yaw = Number.isFinite(sharedMomentState.yaw) ? sharedMomentState.yaw : 0;
        const pitch = Number.isFinite(sharedMomentState.pitch)
            ? THREE.MathUtils.clamp(sharedMomentState.pitch, -Math.PI / 2 + 0.05, Math.PI / 2 - 0.05)
            : 0;
        camera.quaternion.setFromEuler(new THREE.Euler(pitch, yaw, 0, 'YXZ'));
    }

    if (sharedMomentState.tod === 'day' || sharedMomentState.tod === 'night') {
        environment.setOverrideMode(sharedMomentState.tod);
    }

    if (sharedMomentState.isShared) {
        trackEvent('moment_share_opened', {
            tod: sharedMomentState.tod === 'day' || sharedMomentState.tod === 'night' ? sharedMomentState.tod : 'auto'
        });
        showToast('Viewing Shared Moment');
    }
};

const buildSharedMomentUrl = (cycle) => {
    const url = new URL(window.location.href);
    const urlParams = new URLSearchParams(url.search);
    ['px', 'py', 'pz', 'yaw', 'pitch', 'tod', 'shared'].forEach((key) => urlParams.delete(key));

    const position = player.playerObject.position;
    const view = getCurrentViewAngles();
    urlParams.set('px', position.x.toFixed(2));
    urlParams.set('py', position.y.toFixed(2));
    urlParams.set('pz', position.z.toFixed(2));
    urlParams.set('yaw', view.yaw.toFixed(4));
    urlParams.set('pitch', view.pitch.toFixed(4));
    urlParams.set('shared', '1');

    const tod = getCurrentTod(cycle);
    if (tod) {
        urlParams.set('tod', tod);
    }

    url.search = urlParams.toString();
    return { url: url.toString(), tod: tod || 'auto' };
};

let latestCycle = null;
applySharedMomentFromUrl();

shareMomentButton.addEventListener('click', async () => {
    const shared = buildSharedMomentUrl(latestCycle);
    const sharePayload = {
        title: 'Tova',
        text: 'Explore this Tova moment',
        url: shared.url
    };

    if (typeof navigator.share === 'function') {
        try {
            await navigator.share(sharePayload);
            trackEvent('moment_share_clicked', { method: 'web_share', tod: shared.tod });
            showToast('Moment Shared');
            return;
        } catch (error) {
            if (error && error.name === 'AbortError') {
                return;
            }
        }
    }

    if (navigator.clipboard && typeof navigator.clipboard.writeText === 'function') {
        try {
            await navigator.clipboard.writeText(shared.url);
            trackEvent('moment_share_clicked', { method: 'clipboard', tod: shared.tod });
            showToast('Moment Link Copied');
            return;
        } catch (error) {
            console.debug('Clipboard copy failed', error);
        }
    }

    window.prompt('Copy this Tova moment link', shared.url);
    trackEvent('moment_share_clicked', { method: 'prompt', tod: shared.tod });
    showToast('Copy Link From Prompt');
});

// Animation Loop
const clock = new THREE.Clock();

let fpsTimer = 0;
let fpsFrames = 0;
let uiTimer = 0;

function animate() {
    requestAnimationFrame(animate);

    const delta = clock.getDelta();
    const time = clock.getElapsedTime();
    uiTimer += delta;

    fpsFrames += 1;
    fpsTimer += delta;
    if (fpsTimer >= 0.5) {
        const fps = Math.round(fpsFrames / fpsTimer);
        fpsLabel.textContent = `FPS ${fps}`;
        fpsTimer = 0;
        fpsFrames = 0;
    }

    ocean.update(time);
    player.update(delta);
    const cycle = environment.update(time, player.playerObject.position);
    latestCycle = cycle;
    if (cycle) {
        castle.setNightGlow(cycle.night * 0.55);
    }
    mountains.update(player.playerObject.position);

    if (uiTimer >= 0.5) {
        uiTimer = 0;
        const pos = player.playerObject.position;
        coordsLabel.textContent = `X ${pos.x.toFixed(1)} Y ${pos.y.toFixed(1)} Z ${pos.z.toFixed(1)}`;

        if (cycle) {
            const cycleLength = cycle.isDay ? environment.dayLength : environment.nightLength;
            const progress = cycleLength === 0 ? 0 : (cycleLength - cycle.timeToNext) / cycleLength;
            timeBarFill.style.width = `${Math.max(0, Math.min(1, progress)) * 100}%`;
        timeBarFill.style.background = cycle.isDay
            ? 'linear-gradient(90deg, #f7b46a, #fff1d0)'
            : 'linear-gradient(90deg, #2a3f6e, #a0b6ff)';
        setTimeIcon(cycle.isDay);

        if (environment.sunLight) {
            sunGlow.position.copy(environment.sunLight.position);
            const glowStrength = 0.15 + cycle.daylight * 0.85;
            sunGlow.material.opacity = glowStrength;
        }
    }
    }

    // Simple camera rotation for overview
    // camera.position.x = Math.sin(time * 0.1) * 100;
    // camera.position.z = Math.cos(time * 0.1) * 100;
    // camera.lookAt(0, 0, 0);

    renderFrame();
}

// Hide loading screen
const loadingScreen = document.getElementById('loading');
if (loadingScreen) {
    loadingScreen.style.opacity = 0;
    setTimeout(() => loadingScreen.remove(), 500);
}

animate();

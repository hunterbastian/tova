import * as THREE from 'three';
import { Environment } from './world/Environment.js';
import { Terrain } from './world/Terrain.js';
import { Ocean } from './world/Ocean.js';
import { Mountains } from './world/Mountains.js';
import { Forest } from './world/Forest.js';
import { Castle } from './structures/Castle.js';
import { Town } from './structures/Town.js';
import { Player } from './controls/Player.js';

const parseNumber = (value, fallback) => {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : fallback;
};

const params = new URLSearchParams(window.location.search);
const bloomParam = params.get('bloom');
const shadowsParam = params.get('shadows');
const grainParam = params.get('grain');
const vignetteParam = params.get('vignette');
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
    grainCanvas.style.position = 'absolute';
    grainCanvas.style.inset = '0';
    grainCanvas.style.width = '100%';
    grainCanvas.style.height = '100%';
    grainCanvas.style.opacity = '0.06';
    grainCanvas.style.imageRendering = 'pixelated';
    grainCanvas.style.mixBlendMode = 'soft-light';
    uiLayer.appendChild(grainCanvas);

    const grainCtx = grainCanvas.getContext('2d');
    const grainImage = grainCtx.createImageData(grainCanvas.width, grainCanvas.height);
    const grainData = grainImage.data;

    const updateGrain = () => {
        for (let i = 0; i < grainData.length; i += 4) {
            const value = Math.random() * 255;
            grainData[i] = value;
            grainData[i + 1] = value;
            grainData[i + 2] = value;
            grainData[i + 3] = 18;
        }
        grainCtx.putImageData(grainImage, 0, 0);
    };

    updateGrain();
    setInterval(updateGrain, 200);
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
let renderFrame = () => renderer.render(scene, camera);

const initPostprocessing = async () => {
    const [{ EffectComposer }, { RenderPass }, { UnrealBloomPass }, { OutputPass }] = await Promise.all([
        import('three/examples/jsm/postprocessing/EffectComposer.js'),
        import('three/examples/jsm/postprocessing/RenderPass.js'),
        import('three/examples/jsm/postprocessing/UnrealBloomPass.js'),
        import('three/examples/jsm/postprocessing/OutputPass.js')
    ]);

    composer = new EffectComposer(renderer);
    composer.setPixelRatio(Math.min(window.devicePixelRatio || 1, perf.maxPixelRatio));
    composer.setSize(window.innerWidth, window.innerHeight);

    const renderPass = new RenderPass(scene, camera);
    composer.addPass(renderPass);

    const bloomSize = new THREE.Vector2(
        window.innerWidth * perf.bloomResolutionScale,
        window.innerHeight * perf.bloomResolutionScale
    );
    bloomPass = new UnrealBloomPass(bloomSize, 1.2, 0.4, 0.85);
    bloomPass.threshold = 0.3;
    bloomPass.strength = 0.35;
    bloomPass.radius = 0.4;
    composer.addPass(bloomPass);

    const outputPass = new OutputPass();
    composer.addPass(outputPass);

    renderFrame = () => composer.render();
};

if (perf.enableBloom) {
    initPostprocessing();
}

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

import * as THREE from 'three';
import { Environment } from './world/Environment.js';
import { Terrain } from './world/Terrain.js';
import { Ocean } from './world/Ocean.js';
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
renderer.toneMappingExposure = 0.8; // Slightly darker for mood
document.body.appendChild(renderer.domElement);

const timeBar = document.createElement('div');
timeBar.style.position = 'absolute';
timeBar.style.top = '16px';
timeBar.style.left = '50%';
timeBar.style.transform = 'translateX(-50%)';
timeBar.style.width = '240px';
timeBar.style.padding = '4px 6px';
timeBar.style.borderRadius = '8px';
timeBar.style.background = 'rgba(6, 10, 18, 0.2)';
timeBar.style.backdropFilter = 'blur(6px)';
timeBar.style.fontFamily = 'serif';
timeBar.style.color = '#f0f3ff';
timeBar.style.fontSize = '10px';
timeBar.style.letterSpacing = '0.12em';
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

const timeText = document.createElement('span');
timeText.textContent = 'Daylight · 2:00 to sunset';
timeBarLabel.appendChild(timeIcon);
timeBarLabel.appendChild(timeText);

const timeBarTrack = document.createElement('div');
timeBarTrack.style.marginTop = '4px';
timeBarTrack.style.height = '3px';
timeBarTrack.style.borderRadius = '999px';
timeBarTrack.style.background = 'rgba(255, 255, 255, 0.15)';
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

const hud = document.createElement('div');
hud.style.position = 'absolute';
hud.style.top = '16px';
hud.style.right = '16px';
hud.style.display = 'flex';
hud.style.gap = '10px';
hud.style.padding = '8px 10px';
hud.style.borderRadius = '14px';
hud.style.background = 'rgba(6, 10, 18, 0.45)';
hud.style.backdropFilter = 'blur(6px)';
hud.style.fontFamily = 'serif';
hud.style.color = '#f0f3ff';
hud.style.fontSize = '12px';
hud.style.letterSpacing = '0.05em';
hud.style.textTransform = 'uppercase';
hud.style.pointerEvents = 'none';
hud.style.zIndex = '2';

const makeHudItem = (iconSvg, label) => {
    const item = document.createElement('div');
    item.style.display = 'flex';
    item.style.alignItems = 'center';
    item.style.gap = '6px';

    const icon = document.createElement('span');
    icon.style.display = 'inline-flex';
    icon.style.width = '14px';
    icon.style.height = '14px';
    icon.innerHTML = iconSvg;

    const text = document.createElement('span');
    text.textContent = label;

    item.appendChild(icon);
    item.appendChild(text);
    return item;
};

const iconKeyboard = `
<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#f0f3ff" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <rect x="3" y="6" width="18" height="12" rx="2"></rect>
  <path d="M7 10h1M10 10h1M13 10h1M16 10h1M7 13h3M11 13h3M15 13h2"></path>
</svg>`;

const iconMouse = `
<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#f0f3ff" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <rect x="8" y="3" width="8" height="18" rx="4"></rect>
  <path d="M12 7v3"></path>
</svg>`;

const iconSlash = `
<svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="#f0f3ff" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
  <path d="M7 20L17 4"></path>
</svg>`;

hud.appendChild(makeHudItem(iconKeyboard, 'Move'));
hud.appendChild(makeHudItem(iconMouse, 'Look'));
hud.appendChild(makeHudItem(iconSlash, 'Chat'));
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

const formatCountdown = (value) => {
    const total = Math.max(0, Math.ceil(value));
    const minutes = Math.floor(total / 60);
    const seconds = total % 60;
    return `${minutes}:${String(seconds).padStart(2, '0')}`;
};

function animate() {
    requestAnimationFrame(animate);

    const delta = clock.getDelta();
    const time = clock.getElapsedTime();

    ocean.update(time);
    player.update(delta);
    const cycle = environment.update(time, player.playerObject.position);
    if (cycle) {
        castle.setNightGlow(cycle.night * 0.55);

        const cycleLength = cycle.isDay ? environment.dayLength : environment.nightLength;
        const progress = cycleLength === 0 ? 0 : (cycleLength - cycle.timeToNext) / cycleLength;
        timeBarFill.style.width = `${Math.max(0, Math.min(1, progress)) * 100}%`;
        timeBarFill.style.background = cycle.isDay
            ? 'linear-gradient(90deg, #f7b46a, #fff1d0)'
            : 'linear-gradient(90deg, #2a3f6e, #a0b6ff)';
        timeText.textContent = `${cycle.isDay ? 'Daylight' : 'Night'} · ${formatCountdown(cycle.timeToNext)} to ${cycle.isDay ? 'sunset' : 'sunrise'}`;
        setTimeIcon(cycle.isDay);
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

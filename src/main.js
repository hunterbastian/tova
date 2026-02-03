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
const perf = {
    maxPixelRatio: Math.min(2, Math.max(0.75, parseNumber(params.get('pixelRatio'), 1))),
    enableBloom: params.get('bloom') === '1',
    enableShadows: params.get('shadows') !== '0',
    bloomResolutionScale: Math.min(1, Math.max(0.25, parseNumber(params.get('bloomScale'), 0.5)))
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

// Initialize World Components
const environment = new Environment(scene, { enableShadows: perf.enableShadows });
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

const player = new Player(scene, camera, terrain);

// Animation Loop
const clock = new THREE.Clock();

function animate() {
    requestAnimationFrame(animate);

    const delta = clock.getDelta();
    const time = clock.getElapsedTime();

    ocean.update(time);
    player.update(delta);

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

import * as THREE from 'three';
import { Environment } from './world/Environment.js';
import { Terrain } from './world/Terrain.js';
import { Ocean } from './world/Ocean.js';

// Basic Scene Setup
const scene = new THREE.Scene();

const camera = new THREE.PerspectiveCamera(75, window.innerWidth / window.innerHeight, 0.1, 1000);
camera.position.set(0, 20, 50); // Higher starting position to see the terrain

// Slightly lower default quality for smoother performance on typical Macs.
// Antialiasing is disabled and pixel ratio is capped at 1.25 to avoid very high
// resolutions on Retina displays.
const renderer = new THREE.WebGLRenderer({ antialias: false });
renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.25));
renderer.setSize(window.innerWidth, window.innerHeight);
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
renderer.toneMapping = THREE.ACESFilmicToneMapping;
renderer.toneMappingExposure = 0.8; // Slightly darker for mood
document.body.appendChild(renderer.domElement);

// Initialize World Components
const environment = new Environment(scene);
const terrain = new Terrain(scene);
const ocean = new Ocean(scene);

import { Castle } from './structures/Castle.js';
import { Town } from './structures/Town.js';

const castle = new Castle(scene, terrain);
const town = new Town(scene, terrain);


import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';
import { OutputPass } from 'three/examples/jsm/postprocessing/OutputPass.js';
import { PixelationPass } from './postprocessing/PixelationPass.js';

const composer = new EffectComposer(renderer);

const renderPass = new RenderPass(scene, camera);
composer.addPass(renderPass);

// DOOM-style pixelation effect (pixel size 4 = chunky retro look)
const pixelationPass = new PixelationPass(4);
pixelationPass.setSize(window.innerWidth, window.innerHeight);
composer.addPass(pixelationPass);

const bloomPass = new UnrealBloomPass(new THREE.Vector2(window.innerWidth, window.innerHeight), 1.2, 0.4, 0.85);
bloomPass.threshold = 0.3; // Slightly higher threshold so fewer pixels glow
bloomPass.strength = 0.4; // Lower strength for a subtler, cheaper bloom
bloomPass.radius = 0.4;
composer.addPass(bloomPass);

const outputPass = new OutputPass();
composer.addPass(outputPass);

// Resize Handler
window.addEventListener('resize', () => {
    camera.aspect = window.innerWidth / window.innerHeight;
    camera.updateProjectionMatrix();
    renderer.setSize(window.innerWidth, window.innerHeight);
    composer.setSize(window.innerWidth, window.innerHeight);
    pixelationPass.setSize(window.innerWidth, window.innerHeight);
});

import { Player } from './controls/Player.js';

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

    // renderer.render(scene, camera); // Replaced by composer
    composer.render();
}

// Hide loading screen
const loadingScreen = document.getElementById('loading');
if (loadingScreen) {
    loadingScreen.style.opacity = 0;
    setTimeout(() => loadingScreen.remove(), 500);
}

animate();

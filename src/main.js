import * as THREE from 'three';
import { Environment } from './world/Environment.js';
import { Player } from './controls/Player.js';
import { VoxelWorld } from './game/world/VoxelWorld.js';
import ambientTrackUrl from './assets/fantasy-medieval-ambient.mp3';

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
ambientToggle.style.bottom = '224px';
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

const minimapSize = 164;
const minimapBounds = {
    minX: -250,
    maxX: 250,
    minZ: -250,
    maxZ: 250
};
const minimapWorldWidth = minimapBounds.maxX - minimapBounds.minX;
const minimapWorldDepth = minimapBounds.maxZ - minimapBounds.minZ;
const minimapGridSize = 56;
const mapMilestones = [0.25, 0.5, 0.75];
const exploredMapCells = new Set();
let nextMapMilestoneIndex = 0;

const minimapContainer = document.createElement('div');
minimapContainer.style.position = 'fixed';
minimapContainer.style.right = '16px';
minimapContainer.style.bottom = '16px';
minimapContainer.style.padding = '8px';
minimapContainer.style.borderRadius = '12px';
minimapContainer.style.border = '1px solid rgba(255, 255, 255, 0.2)';
minimapContainer.style.background = 'rgba(6, 10, 18, 0.52)';
minimapContainer.style.backdropFilter = 'blur(6px)';
minimapContainer.style.pointerEvents = 'none';
minimapContainer.style.zIndex = '3';
document.body.appendChild(minimapContainer);

const minimapHeader = document.createElement('div');
minimapHeader.style.display = 'flex';
minimapHeader.style.alignItems = 'center';
minimapHeader.style.justifyContent = 'space-between';
minimapHeader.style.marginBottom = '6px';
minimapHeader.style.fontFamily = 'serif';
minimapHeader.style.fontSize = '10px';
minimapHeader.style.letterSpacing = '0.08em';
minimapHeader.style.textTransform = 'uppercase';
minimapHeader.style.color = '#f0f3ff';

const minimapRegionLabel = document.createElement('span');
minimapRegionLabel.textContent = 'Unknown';

const minimapProgressLabel = document.createElement('span');
minimapProgressLabel.textContent = '0%';

minimapHeader.appendChild(minimapRegionLabel);
minimapHeader.appendChild(minimapProgressLabel);
minimapContainer.appendChild(minimapHeader);

const minimapCanvas = document.createElement('canvas');
minimapCanvas.width = minimapSize;
minimapCanvas.height = minimapSize;
minimapCanvas.style.width = `${minimapSize}px`;
minimapCanvas.style.height = `${minimapSize}px`;
minimapCanvas.style.display = 'block';
minimapCanvas.style.borderRadius = '8px';
minimapCanvas.style.border = '1px solid rgba(255, 255, 255, 0.14)';
minimapContainer.appendChild(minimapCanvas);

const minimapBaseCanvas = document.createElement('canvas');
minimapBaseCanvas.width = minimapSize;
minimapBaseCanvas.height = minimapSize;
const minimapFogCanvas = document.createElement('canvas');
minimapFogCanvas.width = minimapSize;
minimapFogCanvas.height = minimapSize;

const minimapCtx = minimapCanvas.getContext('2d');
const minimapBaseCtx = minimapBaseCanvas.getContext('2d');
const minimapFogCtx = minimapFogCanvas.getContext('2d');
const minimapDirection = new THREE.Vector3();

const clamp = (value, min, max) => Math.min(max, Math.max(min, value));

const worldToMinimap = (x, z) => {
    const normalizedX = (x - minimapBounds.minX) / minimapWorldWidth;
    const normalizedZ = (z - minimapBounds.minZ) / minimapWorldDepth;
    return {
        x: clamp(normalizedX, 0, 1) * minimapSize,
        y: (1 - clamp(normalizedZ, 0, 1)) * minimapSize
    };
};

const drawMinimapBase = () => {
    const gradient = minimapBaseCtx.createLinearGradient(0, 0, minimapSize, minimapSize);
    gradient.addColorStop(0, '#1f3726');
    gradient.addColorStop(0.6, '#314f34');
    gradient.addColorStop(1, '#415938');
    minimapBaseCtx.fillStyle = gradient;
    minimapBaseCtx.fillRect(0, 0, minimapSize, minimapSize);

    const oceanEdge = worldToMinimap(-105, 0).x;
    minimapBaseCtx.fillStyle = 'rgba(39, 75, 120, 0.85)';
    minimapBaseCtx.fillRect(0, 0, oceanEdge, minimapSize);

    const drawRegionTint = (x, z, worldRadius, color) => {
        const center = worldToMinimap(x, z);
        const radius = Math.max(8, (worldRadius / minimapWorldWidth) * minimapSize);
        minimapBaseCtx.fillStyle = color;
        minimapBaseCtx.beginPath();
        minimapBaseCtx.arc(center.x, center.y, radius, 0, Math.PI * 2);
        minimapBaseCtx.fill();
    };

    drawRegionTint(-20, 120, 95, 'rgba(122, 143, 92, 0.3)');
    drawRegionTint(-10, -125, 90, 'rgba(32, 88, 54, 0.3)');
    drawRegionTint(170, 165, 110, 'rgba(108, 115, 122, 0.3)');
    drawRegionTint(0, 0, 48, 'rgba(180, 172, 138, 0.32)');
    drawRegionTint(60, 0, 52, 'rgba(166, 140, 104, 0.32)');

    const castle = worldToMinimap(0, 0);
    minimapBaseCtx.fillStyle = '#ffe3ab';
    minimapBaseCtx.beginPath();
    minimapBaseCtx.arc(castle.x, castle.y, 3, 0, Math.PI * 2);
    minimapBaseCtx.fill();

    const townCenter = worldToMinimap(60, 0);
    minimapBaseCtx.fillStyle = '#f5caa8';
    minimapBaseCtx.beginPath();
    minimapBaseCtx.arc(townCenter.x, townCenter.y, 2.5, 0, Math.PI * 2);
    minimapBaseCtx.fill();
};

drawMinimapBase();
minimapFogCtx.fillStyle = 'rgba(3, 7, 13, 0.94)';
minimapFogCtx.fillRect(0, 0, minimapSize, minimapSize);

const getMinimapYaw = () => {
    camera.getWorldDirection(minimapDirection);
    return Math.atan2(minimapDirection.x, minimapDirection.z);
};

const revealMinimapArea = (x, z, worldRadius = 22) => {
    const marker = worldToMinimap(x, z);
    const minimapRadius = Math.max(9, (worldRadius / minimapWorldWidth) * minimapSize);
    const revealGradient = minimapFogCtx.createRadialGradient(
        marker.x, marker.y, minimapRadius * 0.25,
        marker.x, marker.y, minimapRadius
    );
    revealGradient.addColorStop(0, 'rgba(0, 0, 0, 0.9)');
    revealGradient.addColorStop(1, 'rgba(0, 0, 0, 0)');

    minimapFogCtx.save();
    minimapFogCtx.globalCompositeOperation = 'destination-out';
    minimapFogCtx.fillStyle = revealGradient;
    minimapFogCtx.beginPath();
    minimapFogCtx.arc(marker.x, marker.y, minimapRadius, 0, Math.PI * 2);
    minimapFogCtx.fill();
    minimapFogCtx.restore();

    const cellRadiusX = Math.ceil((worldRadius / minimapWorldWidth) * minimapGridSize);
    const cellRadiusZ = Math.ceil((worldRadius / minimapWorldDepth) * minimapGridSize);
    const centerGX = Math.floor(((x - minimapBounds.minX) / minimapWorldWidth) * minimapGridSize);
    const centerGZ = Math.floor(((z - minimapBounds.minZ) / minimapWorldDepth) * minimapGridSize);

    for (let gz = centerGZ - cellRadiusZ; gz <= centerGZ + cellRadiusZ; gz += 1) {
        if (gz < 0 || gz >= minimapGridSize) continue;
        for (let gx = centerGX - cellRadiusX; gx <= centerGX + cellRadiusX; gx += 1) {
            if (gx < 0 || gx >= minimapGridSize) continue;
            const worldX = minimapBounds.minX + ((gx + 0.5) / minimapGridSize) * minimapWorldWidth;
            const worldZ = minimapBounds.minZ + ((gz + 0.5) / minimapGridSize) * minimapWorldDepth;
            const dx = worldX - x;
            const dz = worldZ - z;
            if (dx * dx + dz * dz > worldRadius * worldRadius) continue;
            exploredMapCells.add(`${gx}:${gz}`);
        }
    }

    const completion = exploredMapCells.size / (minimapGridSize * minimapGridSize);
    minimapProgressLabel.textContent = `${Math.round(completion * 100)}%`;
    if (nextMapMilestoneIndex < mapMilestones.length && completion >= mapMilestones[nextMapMilestoneIndex]) {
        const milestonePercent = Math.round(mapMilestones[nextMapMilestoneIndex] * 100);
        showToast(`Map ${milestonePercent}% Revealed`);
        trackEvent('map_exploration_milestone', { milestone_percent: milestonePercent });
        nextMapMilestoneIndex += 1;
    }
};

const renderMinimap = (x, z) => {
    const marker = worldToMinimap(x, z);
    minimapCtx.clearRect(0, 0, minimapSize, minimapSize);
    minimapCtx.drawImage(minimapBaseCanvas, 0, 0);
    minimapCtx.drawImage(minimapFogCanvas, 0, 0);

    minimapCtx.save();
    minimapCtx.translate(marker.x, marker.y);
    minimapCtx.rotate(getMinimapYaw());

    minimapCtx.fillStyle = 'rgba(240, 243, 255, 0.26)';
    minimapCtx.beginPath();
    minimapCtx.moveTo(0, -5);
    minimapCtx.lineTo(-10, -24);
    minimapCtx.lineTo(10, -24);
    minimapCtx.closePath();
    minimapCtx.fill();

    minimapCtx.fillStyle = '#f0f3ff';
    minimapCtx.beginPath();
    minimapCtx.moveTo(0, -7);
    minimapCtx.lineTo(5, 5);
    minimapCtx.lineTo(-5, 5);
    minimapCtx.closePath();
    minimapCtx.fill();
    minimapCtx.restore();
};

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

const chunkLabel = document.createElement('div');
chunkLabel.textContent = 'Chunk 0,0 | Loaded 0 | Queue 0/0 | CORE';

stats.appendChild(fpsLabel);
stats.appendChild(coordsLabel);
stats.appendChild(chunkLabel);
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
const voxelWorld = new VoxelWorld(scene, { enableShadows: perf.enableShadows });
const terrainAdapter = {
    getHeightAt: (x, z) => voxelWorld.getHeightAt(x, z)
};

if (perf.enableShadows) {
    renderer.shadowMap.autoUpdate = true;
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

const player = new Player(scene, camera, terrainAdapter, {
    onCommand: (cmd) => {
        if (cmd === 'day') {
            environment.setOverrideMode('day');
        } else if (cmd === 'night') {
            environment.setOverrideMode('night');
        }
    }
});

const worldRegions = [
    { id: 'crown_keep', name: 'Crown Keep', test: (x, z) => Math.hypot(x, z) <= 48 },
    { id: 'market_village', name: 'Market Village', test: (x, z) => Math.hypot(x - 60, z) <= 54 },
    { id: 'saltshore', name: 'Saltshore', test: (x) => x <= -110 },
    { id: 'highlands', name: 'Highlands', test: (x, z) => z >= 95 && x > -110 },
    { id: 'whisperwood', name: 'Whisperwood', test: (x, z) => z <= -90 && x > -110 },
    { id: 'outer_peaks', name: 'Outer Peaks', test: (x, z) => Math.abs(x) >= 165 || Math.abs(z) >= 165 },
    { id: 'heartlands', name: 'Heartlands', test: () => true }
];

const discoveredRegions = new Set();
let currentRegionId = null;
let regionToastCooldown = 0;
let mapRevealTick = 0;

const findRegion = (x, z) => worldRegions.find((region) => region.test(x, z)) ?? worldRegions[worldRegions.length - 1];

const updateRegionDiscovery = (x, z) => {
    const region = findRegion(x, z);
    if (!region) return;

    minimapRegionLabel.textContent = region.name;
    if (region.id === currentRegionId) return;

    currentRegionId = region.id;
    if (regionToastCooldown > 0) return;

    const isFirstDiscovery = !discoveredRegions.has(region.id);
    discoveredRegions.add(region.id);

    showToast(isFirstDiscovery ? `${region.name} Discovered` : region.name);
    if (isFirstDiscovery) {
        trackEvent('biome_discovered', {
            biome_id: region.id,
            biome_name: region.name,
            x: Number(x.toFixed(1)),
            z: Number(z.toFixed(1))
        });
    }

    regionToastCooldown = 1.1;
};

const playerStart = player.playerObject.position;
revealMinimapArea(playerStart.x, playerStart.z, 26);
renderMinimap(playerStart.x, playerStart.z);
updateRegionDiscovery(playerStart.x, playerStart.z);

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
    mapRevealTick += delta;
    regionToastCooldown = Math.max(0, regionToastCooldown - delta);

    fpsFrames += 1;
    fpsTimer += delta;
    if (fpsTimer >= 0.5) {
        const fps = Math.round(fpsFrames / fpsTimer);
        fpsLabel.textContent = `FPS ${fps}`;
        fpsTimer = 0;
        fpsFrames = 0;
    }

    player.update(delta);
    voxelWorld.update(player.playerObject.position);
    const cycle = environment.update(time, player.playerObject.position);

    if (mapRevealTick >= 0.08) {
        mapRevealTick = 0;
        const pos = player.playerObject.position;
        revealMinimapArea(pos.x, pos.z, 24);
        renderMinimap(pos.x, pos.z);
        updateRegionDiscovery(pos.x, pos.z);
    }

    if (uiTimer >= 0.5) {
        uiTimer = 0;
        const pos = player.playerObject.position;
        coordsLabel.textContent = `X ${pos.x.toFixed(1)} Y ${pos.y.toFixed(1)} Z ${pos.z.toFixed(1)}`;
        const worldStats = voxelWorld.getDebugStats(pos.x, pos.z);
        chunkLabel.textContent = `Chunk ${worldStats.playerChunkX},${worldStats.playerChunkZ} | Loaded ${worldStats.loadedChunks} | Queue ${worldStats.pendingChunks}/${worldStats.pendingMeshes} | ${worldStats.zone.toUpperCase()}`;

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

window.render_game_to_text = () => {
    const pos = player.playerObject.position;
    const stats = voxelWorld.getDebugStats(pos.x, pos.z);
    return JSON.stringify({
        mode: 'play',
        coordinate_system: 'x-right, y-up, z-forward',
        player: {
            x: Number(pos.x.toFixed(2)),
            y: Number(pos.y.toFixed(2)),
            z: Number(pos.z.toFixed(2)),
            flying: player.isFlying
        },
        chunks: {
            current: { x: stats.playerChunkX, z: stats.playerChunkZ },
            loaded: stats.loadedChunks,
            pending_generation: stats.pendingChunks,
            pending_meshes: stats.pendingMeshes
        },
        world_zone: stats.zone
    });
};

window.advanceTime = (ms) => new Promise((resolve) => {
    const end = performance.now() + Math.max(0, ms);
    const step = (now) => {
        if (now >= end) {
            resolve();
            return;
        }
        requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
});

// Hide loading screen
const loadingScreen = document.getElementById('loading');
if (loadingScreen) {
    loadingScreen.style.opacity = 0;
    setTimeout(() => loadingScreen.remove(), 500);
}

animate();

import "./style.css";

import * as THREE from "three";
import { PointerLockControls } from "three/examples/jsm/controls/PointerLockControls.js";
import { Sky } from "three/examples/jsm/objects/Sky.js";

import {
  HOTBAR,
  PLAYER_RESPAWN_DELAY,
  SWORD_DAMAGE,
  createGameState,
  getSafeMode,
  isAutomationSession,
} from "./constants.js";
import { createActorSystem } from "./actors.js";
import { createAudioSystem } from "./audio.js";
import { createCollisionSystem } from "./collision.js";
import { createInteractableSystem } from "./interactables.js";
import { createPlayerSystem } from "./player.js";
import { createUi } from "./ui.js";
import { createWeaponSystem } from "./weapon.js";
import { createWorldSystem } from "./world.js";
import { createShaderPipeline } from "./shaders.js";

const app = document.querySelector("#app");
const safeMode = getSafeMode();
const ui = createUi({ app, hotbar: HOTBAR, safeMode });

const renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: "high-performance" });
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
renderer.setSize(window.innerWidth, window.innerHeight);
renderer.setClearColor(safeMode ? "#c4ceb3" : "#d8c8b8", 1);
renderer.shadowMap.enabled = !safeMode;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;
renderer.outputColorSpace = THREE.SRGBColorSpace;
renderer.toneMapping = safeMode ? THREE.NoToneMapping : THREE.ReinhardToneMapping;
renderer.toneMappingExposure = safeMode ? 1 : 1.35;
app.appendChild(renderer.domElement);

const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(72, window.innerWidth / window.innerHeight, 0.1, 600);
const shaderPipeline = createShaderPipeline({ renderer, scene, camera, safeMode });
scene.background = new THREE.Color(safeMode ? "#c4ceb3" : "#d8c8b8");
scene.fog = safeMode ? null : new THREE.FogExp2("#d8c8b8", 0.012);

const controls = new PointerLockControls(camera, renderer.domElement);
scene.add(controls.object);

const sun = new THREE.Vector3();
sun.setFromSphericalCoords(1, Math.PI * 0.47, Math.PI * 0.12);

if (!safeMode) {
  const sky = new Sky();
  sky.scale.setScalar(450000);
  scene.add(sky);

  const skyUniforms = sky.material.uniforms;
  skyUniforms.turbidity.value = 14;
  skyUniforms.rayleigh.value = 3.2;
  skyUniforms.mieCoefficient.value = 0.03;
  skyUniforms.mieDirectionalG.value = 0.92;
  skyUniforms.sunPosition.value.copy(sun);
}

const ambientLight = new THREE.AmbientLight("#c8a878", 1.1);
scene.add(ambientLight);

const hemiLight = new THREE.HemisphereLight("#d8c0a0", "#8a7060", 1.2);
scene.add(hemiLight);

const sunLight = new THREE.DirectionalLight("#f0c890", 2.0);
sunLight.position.set(88, 132, -24);
sunLight.castShadow = true;
sunLight.shadow.mapSize.set(2048, 2048);
sunLight.shadow.camera.left = -140;
sunLight.shadow.camera.right = 140;
sunLight.shadow.camera.top = 140;
sunLight.shadow.camera.bottom = -140;
sunLight.shadow.camera.near = 1;
sunLight.shadow.camera.far = 280;
scene.add(sunLight);
scene.add(sunLight.target);

const spawnFillLight = new THREE.PointLight("#e0c898", 1.6, 52, 2);
scene.add(spawnFillLight);

if (!safeMode) {
  const moon = new THREE.Mesh(
    new THREE.SphereGeometry(7, 24, 24),
    new THREE.MeshBasicMaterial({ color: "#e8d8c0" }),
  );
  moon.position.set(-110, 92, -210);
  scene.add(moon);
}

const state = createGameState();

let playerSystem;
let weaponSystem;
let actorSystem;
const audio = createAudioSystem();
const collisionSystem = createCollisionSystem();
const interactableSystem = createInteractableSystem();

const setStatus = (message) => {
  ui.setStatus(state, message);
};

const refreshHud = () => {
  if ((playerSystem?.canControl() ?? false) && !state.isDead) {
    audio.startWind();
  }

  ui.updateHud({
    state,
    controlsLocked: controls.isLocked,
    canControl: () => playerSystem?.canControl() ?? false,
    interactionPrompt: interactableSystem.getPrompt(controls.object.position),
    compass: {
      yaw: camera.rotation.y,
      playerX: controls.object.position.x,
      playerZ: controls.object.position.z,
      landmarks: {
        shrine: state.swordPickupPosition,
        castle: state.castleCenter,
        forest: state.forestCenter,
      },
    },
  });
};

function handlePlayerDamage(amount) {
  if (state.isDead) {
    return;
  }

  state.health = Math.max(0, state.health - amount);
  ui.flashDamage();
  audio.playerDamage();

  if (state.health <= 0) {
    state.isDead = true;
    state.respawnTimer = PLAYER_RESPAWN_DELAY;
    ui.showDeathScreen(true);
    setStatus("You have fallen");
  }
}

function handleEnemyHit(actorId) {
  const { hit, killed } = actorSystem.damageActor(actorId, SWORD_DAMAGE);
  if (hit) audio.hit();
  if (killed) {
    state.kills += 1;
    setStatus(`Skeleton slain (${state.kills} killed)`);
  } else if (hit) {
    setStatus("Hit!");
  }
}

weaponSystem = createWeaponSystem({
  camera,
  safeMode,
  state,
  onPickup: () => {
    audio.pickup();
    setStatus("Iron sword taken");
    refreshHud();
  },
  onHit: handleEnemyHit,
  onSwing: () => audio.swordSwing(),
});

const worldSystem = createWorldSystem({
  scene,
  safeMode,
  state,
  createPedestalSword: weaponSystem.createPedestalSword,
  collisionSystem,
});

actorSystem = createActorSystem({
  scene,
  safeMode,
  state,
  onPlayerDamage: (amount) => handlePlayerDamage(amount),
});

const regenerateWorld = () => {
  interactableSystem.clear();
  weaponSystem.reset();
  actorSystem.clear();
  worldSystem.regenerateWorld();
  interactableSystem.register({
    id: "shrine-sword",
    label: "Iron Sword",
    prompt: "Press E to take the iron sword",
    position: state.swordPickupPosition,
    radius: 3.2,
    isAvailable: () => weaponSystem.isSwordAvailable(),
    onInteract: () => weaponSystem.takeSword(),
  });
  actorSystem.rebuild({
    seed: state.seed,
    sampleGroundHeight: worldSystem.sampleGroundHeight,
    castleCenter: state.castleCenter,
    forestCenter: state.forestCenter,
  });
  playerSystem.respawnAtSpawn();
  spawnFillLight.position.set(4, worldSystem.sampleGroundHeight(4, 0) + 7.5, 8);
  state.mode = "intro";
  state.health = 1;
  state.isDead = false;
  state.respawnTimer = 0;
  state.kills = 0;
  ui.showDeathScreen(false);
  setStatus("Green rise ahead. The keep waits in the distance.");
  refreshHud();
};

playerSystem = createPlayerSystem({
  app,
  camera,
  collisionSystem,
  controls,
  domElement: renderer.domElement,
  isAutomationSession,
  onFootstep: (sprinting) => audio.footstep(sprinting),
  onHudChange: refreshHud,
  onInteract: () => { if (!state.isDead) interactableSystem.interact(controls.object.position); },
  onLand: (intensity) => audio.land(intensity),
  onPrimaryAttack: () => { if (!state.isDead) weaponSystem.swing(); },
  onRegenerate: regenerateWorld,
  onStatus: setStatus,
  sampleGroundHeight: worldSystem.sampleGroundHeight,
  state,
  sunLight,
});

function onResize() {
  camera.aspect = window.innerWidth / window.innerHeight;
  camera.updateProjectionMatrix();
  renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
  renderer.setSize(window.innerWidth, window.innerHeight);
  shaderPipeline.setSize(window.innerWidth, window.innerHeight);
}

function updateScene(dt) {
  if (state.isDead) {
    state.respawnTimer -= dt;
    if (state.respawnTimer <= 0) {
      state.isDead = false;
      state.health = 1;
      state.respawnTimer = 0;
      ui.showDeathScreen(false);
      playerSystem.respawnAtSpawn();
      setStatus("You awaken at the shrine");
    }
    shaderPipeline.render();
    refreshHud();
    return;
  }

  playerSystem.update(dt);
  weaponSystem.update(dt);

  const actorTargets = actorSystem.getActorsForCombat();
  weaponSystem.checkHit(actorTargets);
  actorSystem.update(dt, controls.object.position);

  shaderPipeline.render();
  refreshHud();
}

let lastTime = performance.now();
function animate(now = performance.now()) {
  const dt = Math.min((now - lastTime) / 1000, 0.05);
  lastTime = now;
  updateScene(dt);
}

window.advanceTime = (ms) => {
  const steps = Math.max(1, Math.round(ms / (1000 / 60)));
  for (let index = 0; index < steps; index += 1) {
    updateScene(1 / 60);
  }
};

window.render_game_to_text = () =>
  JSON.stringify({
    mode: state.mode,
    safeMode,
    pointerLocked: controls.isLocked,
    walkMode: state.walkMode,
    seed: state.seed.toString(16),
    player: {
      position: {
        x: Number(controls.object.position.x.toFixed(2)),
        y: Number(controls.object.position.y.toFixed(2)),
        z: Number(controls.object.position.z.toFixed(2)),
      },
      velocityY: Number(state.velocity.y.toFixed(2)),
      grounded: state.grounded,
      isDead: state.isDead,
    },
    selected: HOTBAR[state.selectedSlot].label,
    weapon: {
      hasSword: state.hasSword,
      prompt: state.interactionPrompt,
    },
    interactables: interactableSystem.getDebugState(controls.object.position),
    actors: actorSystem.getDebugState(controls.object.position),
    landmarks: {
      spawn: { x: 0, z: 0 },
      swordShrine: {
        x: Number(state.swordPickupPosition.x.toFixed(1)),
        z: Number(state.swordPickupPosition.z.toFixed(1)),
      },
      forest: {
        x: Number(state.forestCenter.x.toFixed(1)),
        z: Number(state.forestCenter.z.toFixed(1)),
      },
      castle: {
        x: Number(state.castleCenter.x.toFixed(1)),
        z: Number(state.castleCenter.z.toFixed(1)),
      },
    },
    vitals: {
      health: Number(state.health.toFixed(2)),
      magicka: Number(state.magicka.toFixed(2)),
      fatigue: Number(state.fatigue.toFixed(2)),
    },
    combat: {
      kills: state.kills,
      enemyPresence: state.enemyPresence,
    },
    axes: "x east-west, y up, z north-south",
  });

window.__tova = { camera, controls, scene, renderer };
window.addEventListener("resize", onResize);

renderer.domElement.addEventListener("webglcontextlost", (event) => {
  event.preventDefault();
  setStatus("Renderer context lost");
});

regenerateWorld();
onResize();
refreshHud();
lastTime = performance.now();
renderer.setAnimationLoop(animate);

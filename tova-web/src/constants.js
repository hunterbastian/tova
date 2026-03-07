import * as THREE from "three";

export const WORLD_SIZE = 220;
export const WORLD_SEGMENTS = 110;
export const SPAWN_RADIUS = 14;
export const SPAWN_BLEND_RADIUS = 30;
export const PLAYER_HEIGHT = 1.8;
export const WALK_SPEED = 6.1;
export const SPRINT_SPEED = 8.7;
export const GRAVITY = 24;
export const JUMP_SPEED = 8.8;
export const SWORD_PICKUP_RADIUS = 3.2;
export const SWORD_SWING_DURATION = 0.28;
export const SWORD_REACH = 4.2;
export const SWORD_DAMAGE = 0.4;
export const ENEMY_MAX_HP = 1.0;
export const ENEMY_ATTACK_DAMAGE = 0.12;
export const ENEMY_ATTACK_RANGE = 2.6;
export const ENEMY_PURSUE_SPEED = 3.2;
export const ENEMY_ATTACK_COOLDOWN = 1.8;
export const ENEMY_STAGGER_DURATION = 0.45;
export const ENEMY_DEATH_DURATION = 0.6;
export const BOB_WALK_FREQ = 1.8;
export const BOB_SPRINT_FREQ = 2.4;
export const BOB_VERTICAL_AMP = 0.044;
export const BOB_ROLL_AMP = 0.006;
export const MOVE_ACCEL = 14;
export const MOVE_DECEL = 10;
export const LAND_DIP_SCALE = 0.012;
export const LAND_DIP_MAX = 0.14;
export const LAND_DIP_RECOVERY = 8;
export const PLAYER_DAMAGE_FLASH_DURATION = 0.3;
export const PLAYER_DEATH_FADE_DURATION = 1.2;
export const PLAYER_RESPAWN_DELAY = 2.0;
export const HOTBAR = [
  { label: "Soil", color: "#7f6543" },
  { label: "Stone", color: "#8c8b84" },
  { label: "Grass", color: "#738e51" },
  { label: "Sand", color: "#b8ab82" },
  { label: "Cobble", color: "#6e706d" },
];

export const isAutomationSession = navigator.webdriver === true;
export const SAFE_MODE_STORAGE_KEY = "tova-safe-mode";

export function getSafeMode() {
  const params = new URLSearchParams(window.location.search);
  if (params.get("safe") === "1") {
    localStorage.setItem(SAFE_MODE_STORAGE_KEY, "1");
    return true;
  }

  if (params.get("safe") === "0") {
    localStorage.removeItem(SAFE_MODE_STORAGE_KEY);
    return false;
  }

  return localStorage.getItem(SAFE_MODE_STORAGE_KEY) === "1";
}

export function createGameState() {
  return {
    mode: "intro",
    walkMode: false,
    selectedSlot: 2,
    hasSword: false,
    health: 1,
    magicka: 0.88,
    fatigue: 0.84,
    status: "Three.js frontier loading",
    interactionPrompt: "",
    seed: 0,
    pressed: new Set(),
    grounded: true,
    velocity: new THREE.Vector3(),
    moveVector: new THREE.Vector3(),
    footPosition: new THREE.Vector3(),
    terrainMesh: null,
    forestCenter: new THREE.Vector3(),
    castleCenter: new THREE.Vector3(),
    swordPickupPosition: new THREE.Vector3(),
    swordPedestalSword: null,
    swordGroup: null,
    swordSwing: 0,
    terrainContext: null,
    enemyPresence: 0,
    kills: 0,
    isDead: false,
    respawnTimer: 0,
    lastStatusAt: performance.now(),
  };
}

import * as THREE from "three";

import { SWORD_REACH, SWORD_SWING_DURATION } from "./constants.js";

function markShared(resource) {
  resource.userData = resource.userData || {};
  resource.userData.tovaShared = true;
  return resource;
}

const sharedSwordAssets = {
  materials: new Map(),
  bladeGeometry: markShared(new THREE.BoxGeometry(0.12, 2.55, 0.05)),
  fullerGeometry: markShared(new THREE.BoxGeometry(0.03, 1.8, 0.01)),
  tipGeometry: markShared(new THREE.ConeGeometry(0.12, 0.36, 4)),
  guardGeometry: markShared(new THREE.BoxGeometry(0.62, 0.08, 0.12)),
  gripGeometry: markShared(new THREE.CylinderGeometry(0.06, 0.075, 0.62, 8)),
  pommelGeometry: markShared(new THREE.SphereGeometry(0.12, 12, 12)),
};

function getSwordMaterial(name, safeMode) {
  const key = `${safeMode ? "safe" : "lit"}:${name}`;
  let material = sharedSwordAssets.materials.get(key);
  if (!material) {
    const definitions = {
      steel: { color: "#cbc6bb", roughness: 0.34, metalness: 0.82, flatShading: true },
      fuller: { color: "#8f8b84", roughness: 0.36, metalness: 0.8, flatShading: true },
      guard: { color: "#8f7444", roughness: 0.58, metalness: 0.46, flatShading: true },
      grip: { color: "#3c2f28", roughness: 0.92, metalness: 0.08, flatShading: true },
    };
    const config = definitions[name];
    material = markShared(
      safeMode
        ? new THREE.MeshBasicMaterial({ color: config.color, fog: true })
        : new THREE.MeshStandardMaterial(config),
    );
    sharedSwordAssets.materials.set(key, material);
  }

  return material;
}

function createSwordModel(safeMode) {
  const sword = new THREE.Group();

  const blade = new THREE.Mesh(sharedSwordAssets.bladeGeometry, getSwordMaterial("steel", safeMode));
  blade.position.y = 1.4;
  sword.add(blade);

  const fuller = new THREE.Mesh(sharedSwordAssets.fullerGeometry, getSwordMaterial("fuller", safeMode));
  fuller.position.set(0, 1.2, 0.028);
  sword.add(fuller);

  const tip = new THREE.Mesh(sharedSwordAssets.tipGeometry, getSwordMaterial("steel", safeMode));
  tip.position.y = 2.82;
  tip.rotation.z = Math.PI;
  sword.add(tip);

  const guard = new THREE.Mesh(sharedSwordAssets.guardGeometry, getSwordMaterial("guard", safeMode));
  guard.position.y = 0.12;
  sword.add(guard);

  const grip = new THREE.Mesh(sharedSwordAssets.gripGeometry, getSwordMaterial("grip", safeMode));
  grip.position.y = -0.25;
  sword.add(grip);

  const pommel = new THREE.Mesh(sharedSwordAssets.pommelGeometry, getSwordMaterial("guard", safeMode));
  pommel.position.y = -0.62;
  sword.add(pommel);

  sword.traverse((node) => {
    if (node.isMesh) {
      node.castShadow = true;
      node.receiveShadow = true;
    }
  });

  return sword;
}

export function createWeaponSystem({ camera, safeMode, state, onPickup, onHit, onSwing }) {
  const weaponAnchor = new THREE.Group();
  weaponAnchor.position.set(0.78, -0.82, -1.02);
  camera.add(weaponAnchor);

  const raycaster = new THREE.Raycaster();
  raycaster.far = SWORD_REACH;
  const rayOrigin = new THREE.Vector3();
  const rayDirection = new THREE.Vector3();
  let hitThisSwing = false;

  state.swordGroup = createSwordModel(safeMode);
  state.swordGroup.visible = false;
  state.swordGroup.rotation.set(-0.18, -0.14, 0.46);
  state.swordGroup.scale.setScalar(0.46);
  weaponAnchor.add(state.swordGroup);

  function isSwordAvailable() {
    return !state.hasSword && Boolean(state.swordPedestalSword);
  }

  function takeSword() {
    if (!isSwordAvailable()) {
      return false;
    }

    state.hasSword = true;
    state.swordSwing = 0;
    state.swordGroup.visible = true;

    if (state.swordPedestalSword?.parent) {
      state.swordPedestalSword.parent.remove(state.swordPedestalSword);
    }

    state.swordPedestalSword = null;
    onPickup?.();
    return true;
  }

  function reset() {
    state.hasSword = false;
    state.swordPedestalSword = null;
    state.swordSwing = 0;
    hitThisSwing = false;
    if (state.swordGroup) {
      state.swordGroup.visible = false;
    }
  }

  function swing() {
    if (state.hasSword && state.swordSwing <= 0) {
      state.swordSwing = SWORD_SWING_DURATION;
      hitThisSwing = false;
      onSwing?.();
    }
  }

  function checkHit(actorTargets) {
    if (!state.hasSword || state.swordSwing <= 0 || hitThisSwing) {
      return null;
    }

    const swingProgress = 1 - state.swordSwing / SWORD_SWING_DURATION;
    if (swingProgress < 0.15 || swingProgress > 0.65) {
      return null;
    }

    camera.getWorldPosition(rayOrigin);
    camera.getWorldDirection(rayDirection);
    raycaster.set(rayOrigin, rayDirection);

    const meshes = [];
    for (const target of actorTargets) {
      target.model.traverse((node) => {
        if (node.isMesh) {
          node.userData.actorId = target.id;
          meshes.push(node);
        }
      });
    }

    const intersections = raycaster.intersectObjects(meshes, false);
    if (intersections.length > 0) {
      hitThisSwing = true;
      const actorId = intersections[0].object.userData.actorId;
      onHit?.(actorId);
      return actorId;
    }

    return null;
  }

  function update(dt) {
    if (!state.swordGroup) {
      return;
    }

    state.swordGroup.visible = state.hasSword;
    if (!state.hasSword) {
      return;
    }

    state.swordSwing = Math.max(0, state.swordSwing - dt);
    const swingProgress =
      state.swordSwing > 0 ? 1 - state.swordSwing / SWORD_SWING_DURATION : 0;
    const moveAmount = state.moveVector.lengthSq() > 0 ? 1 : 0;
    const bobTime = performance.now() * 0.008;
    const bob = Math.sin(bobTime) * 0.015 * moveAmount;
    const lift = Math.abs(Math.cos(bobTime)) * 0.01 * moveAmount;
    const slashArc = Math.sin(swingProgress * Math.PI);

    weaponAnchor.position.set(0.78 + bob, -0.82 + lift, -1.02);
    weaponAnchor.rotation.set(
      -0.44 + slashArc * 0.48,
      -0.12 - slashArc * 0.18,
      0.52 - slashArc * 0.92,
    );
  }

  return {
    checkHit,
    createPedestalSword: () => createSwordModel(safeMode),
    isSwordAvailable,
    reset,
    takeSword,
    swing,
    update,
  };
}

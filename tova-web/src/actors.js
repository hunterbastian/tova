import * as THREE from "three";

import {
  ENEMY_ATTACK_COOLDOWN,
  ENEMY_ATTACK_DAMAGE,
  ENEMY_ATTACK_RANGE,
  ENEMY_DEATH_DURATION,
  ENEMY_MAX_HP,
  ENEMY_PURSUE_SPEED,
  ENEMY_STAGGER_DURATION,
} from "./constants.js";

function markShared(resource) {
  resource.userData = resource.userData || {};
  resource.userData.tovaShared = true;
  return resource;
}

function isSharedResource(resource) {
  return resource?.userData?.tovaShared === true;
}

const sharedActorAssets = {
  boneMaterials: new Map(),
  eyeMaterial: markShared(new THREE.MeshBasicMaterial({ color: "#c9d6a2" })),
  eyeAggroMaterial: markShared(new THREE.MeshBasicMaterial({ color: "#e85040" })),
  ribGeometry: markShared(new THREE.CapsuleGeometry(0.18, 0.72, 4, 8)),
  limbGeometry: markShared(new THREE.CapsuleGeometry(0.09, 0.88, 4, 8)),
  spineGeometry: markShared(new THREE.CapsuleGeometry(0.08, 0.66, 4, 8)),
  skullGeometry: markShared(new THREE.SphereGeometry(0.24, 12, 10)),
  eyeGeometry: markShared(new THREE.SphereGeometry(0.04, 8, 8)),
};

function getBoneMaterial(safeMode) {
  const key = safeMode ? "safe" : "lit";
  let material = sharedActorAssets.boneMaterials.get(key);
  if (!material) {
    material = markShared(
      safeMode
        ? new THREE.MeshBasicMaterial({ color: "#c8c1b2", fog: true })
        : new THREE.MeshStandardMaterial({
            color: "#c8c1b2",
            roughness: 0.92,
            metalness: 0.04,
          }),
    );
    sharedActorAssets.boneMaterials.set(key, material);
  }

  return material;
}

function mulberry32(seed) {
  let t = seed >>> 0;
  return () => {
    t += 0x6d2b79f5;
    let value = Math.imul(t ^ (t >>> 15), t | 1);
    value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
    return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
  };
}

function createSkeletonModel(safeMode) {
  const boneMaterial = getBoneMaterial(safeMode);
  const skeleton = new THREE.Group();

  const pelvis = new THREE.Mesh(sharedActorAssets.ribGeometry, boneMaterial);
  pelvis.scale.set(0.8, 0.45, 0.55);
  pelvis.position.y = 0.8;
  skeleton.add(pelvis);

  const torso = new THREE.Mesh(sharedActorAssets.ribGeometry, boneMaterial);
  torso.scale.set(1, 0.75, 0.62);
  torso.position.y = 1.45;
  skeleton.add(torso);

  const spine = new THREE.Mesh(sharedActorAssets.spineGeometry, boneMaterial);
  spine.position.y = 1.15;
  skeleton.add(spine);

  const skull = new THREE.Mesh(sharedActorAssets.skullGeometry, boneMaterial);
  skull.position.y = 2.18;
  skull.scale.set(0.92, 1.08, 0.9);
  skeleton.add(skull);

  const leftEye = new THREE.Mesh(sharedActorAssets.eyeGeometry, sharedActorAssets.eyeMaterial);
  leftEye.position.set(-0.08, 2.2, 0.17);
  leftEye.name = "eye-left";
  skeleton.add(leftEye);

  const rightEye = leftEye.clone();
  rightEye.position.x = 0.08;
  rightEye.name = "eye-right";
  skeleton.add(rightEye);

  const armOffsets = [-0.42, 0.42];
  for (const x of armOffsets) {
    const arm = new THREE.Mesh(sharedActorAssets.limbGeometry, boneMaterial);
    arm.position.set(x, 1.35, 0);
    arm.rotation.z = x < 0 ? 0.32 : -0.32;
    arm.scale.set(0.82, 1, 0.82);
    skeleton.add(arm);
  }

  const legOffsets = [-0.16, 0.16];
  for (const x of legOffsets) {
    const leg = new THREE.Mesh(sharedActorAssets.limbGeometry, boneMaterial);
    leg.position.set(x, 0.12, 0);
    leg.scale.set(0.86, 1.14, 0.86);
    skeleton.add(leg);
  }

  skeleton.traverse((node) => {
    if (node.isMesh) {
      node.castShadow = true;
      node.receiveShadow = true;
    }
  });

  return skeleton;
}

export function createActorSystem({ scene, safeMode, state, onPlayerDamage }) {
  const actorRoot = new THREE.Group();
  scene.add(actorRoot);
  const actors = [];
  let sampleGroundHeight = null;

  function clear() {
    while (actorRoot.children.length > 0) {
      const child = actorRoot.children[actorRoot.children.length - 1];
      actorRoot.remove(child);
      child.traverse?.((node) => {
        if (node.geometry && !isSharedResource(node.geometry)) {
          node.geometry.dispose();
        }
        if (node.material) {
          if (Array.isArray(node.material)) {
            node.material.forEach((material) => {
              if (material && !isSharedResource(material)) {
                material.dispose();
              }
            });
          } else if (!isSharedResource(node.material)) {
            node.material.dispose();
          }
        }
      });
    }

    actors.length = 0;
  }

  function spawnSkeleton({ id, role, position, facing, wakeRadius }) {
    const model = createSkeletonModel(safeMode);
    model.position.copy(position);
    model.rotation.y = facing;
    actorRoot.add(model);

    actors.push({
      id,
      role,
      model,
      homePosition: position.clone(),
      facing,
      wakeRadius,
      mood: "dormant",
      hp: ENEMY_MAX_HP,
      attackCooldown: 0,
      staggerTimer: 0,
      deathTimer: 0,
      pulse: Math.random() * Math.PI * 2,
      watchTimer: 0,
    });
  }

  function rebuild({ seed, sampleGroundHeight: groundSampler, castleCenter, forestCenter }) {
    clear();
    sampleGroundHeight = groundSampler;

    const rng = mulberry32(seed ^ 0x4b1d2a93);
    const spawnGroups = [
      { center: castleCenter, count: 4, radiusMin: 10, radiusMax: 18, wakeRadius: 16, role: "castle-sentry" },
      { center: forestCenter, count: 2, radiusMin: 8, radiusMax: 14, wakeRadius: 12, role: "woodland-watcher" },
    ];

    let index = 0;
    for (const group of spawnGroups) {
      for (let count = 0; count < group.count; count += 1) {
        const angle = rng() * Math.PI * 2;
        const distance = group.radiusMin + rng() * (group.radiusMax - group.radiusMin);
        const x = group.center.x + Math.cos(angle) * distance;
        const z = group.center.z + Math.sin(angle) * distance;
        const y = groundSampler(x, z);
        spawnSkeleton({
          id: `skeleton-${index}`,
          role: group.role,
          position: new THREE.Vector3(x, y + 0.08, z),
          facing: angle + Math.PI,
          wakeRadius: group.wakeRadius,
        });
        index += 1;
      }
    }
  }

  function setEyeColor(actor, aggressive) {
    const mat = aggressive ? sharedActorAssets.eyeAggroMaterial : sharedActorAssets.eyeMaterial;
    actor.model.traverse((node) => {
      if (node.name === "eye-left" || node.name === "eye-right") {
        node.material = mat;
      }
    });
  }

  function damageActor(actorId, amount) {
    const actor = actors.find((a) => a.id === actorId);
    if (!actor || actor.mood === "dead") {
      return { hit: false, killed: false };
    }

    actor.hp -= amount;
    actor.staggerTimer = ENEMY_STAGGER_DURATION;
    actor.mood = "staggered";

    let killed = false;
    if (actor.hp <= 0) {
      actor.mood = "dead";
      actor.deathTimer = ENEMY_DEATH_DURATION;
      killed = true;
    }

    return { hit: true, killed };
  }

  function getActorsForCombat() {
    const combatActors = [];
    for (const actor of actors) {
      if (actor.mood !== "dead") {
        combatActors.push({
          id: actor.id,
          model: actor.model,
          position: actor.model.position,
        });
      }
    }
    return combatActors;
  }

  function getAliveCount() {
    return actors.filter((a) => a.mood !== "dead").length;
  }

  function update(dt, playerPosition) {
    const toRemove = [];

    for (const actor of actors) {
      const distance = actor.model.position.distanceTo(playerPosition);
      actor.pulse += dt * 2.4;

      if (actor.mood === "dead") {
        actor.deathTimer -= dt;
        const fade = Math.max(0, actor.deathTimer / ENEMY_DEATH_DURATION);
        actor.model.position.y -= dt * 1.5;
        actor.model.traverse((node) => {
          if (node.isMesh && node.material) {
            if (!node.material.transparent) {
              node.material = node.material.clone();
              node.material.transparent = true;
            }
            node.material.opacity = fade;
          }
        });
        if (actor.deathTimer <= 0) {
          toRemove.push(actor);
        }
        continue;
      }

      if (actor.mood === "staggered") {
        actor.staggerTimer -= dt;
        const staggerShake = Math.sin(actor.staggerTimer * 40) * 0.12;
        actor.model.position.x = actor.model.position.x + staggerShake * dt * 8;
        if (actor.staggerTimer <= 0) {
          actor.mood = distance <= actor.wakeRadius ? "pursuing" : "dormant";
        }
        continue;
      }

      actor.attackCooldown = Math.max(0, actor.attackCooldown - dt);

      if (distance > actor.wakeRadius) {
        actor.mood = "dormant";
        actor.watchTimer = 0;
        setEyeColor(actor, false);
      } else if (actor.mood === "dormant") {
        actor.mood = "watching";
        actor.watchTimer = 0;
        setEyeColor(actor, true);
      }

      if (actor.mood === "watching") {
        actor.watchTimer += dt;
        const toPlayer = Math.atan2(
          playerPosition.x - actor.model.position.x,
          playerPosition.z - actor.model.position.z,
        );
        actor.model.rotation.y = toPlayer;
        actor.model.position.y = actor.homePosition.y + Math.sin(actor.pulse) * 0.04;

        if (actor.watchTimer > 1.2) {
          actor.mood = "pursuing";
        }
      }

      if (actor.mood === "pursuing") {
        const toPlayer = Math.atan2(
          playerPosition.x - actor.model.position.x,
          playerPosition.z - actor.model.position.z,
        );
        actor.model.rotation.y = toPlayer;

        if (distance > ENEMY_ATTACK_RANGE) {
          const moveX = Math.sin(toPlayer) * ENEMY_PURSUE_SPEED * dt;
          const moveZ = Math.cos(toPlayer) * ENEMY_PURSUE_SPEED * dt;
          actor.model.position.x += moveX;
          actor.model.position.z += moveZ;
          if (sampleGroundHeight) {
            actor.model.position.y =
              sampleGroundHeight(actor.model.position.x, actor.model.position.z) + 0.08;
          }
          const bobAmount = Math.sin(actor.pulse * 3) * 0.06;
          actor.model.position.y += bobAmount;
        }

        if (distance <= ENEMY_ATTACK_RANGE) {
          actor.mood = "attacking";
        }
      }

      if (actor.mood === "attacking") {
        const toPlayer = Math.atan2(
          playerPosition.x - actor.model.position.x,
          playerPosition.z - actor.model.position.z,
        );
        actor.model.rotation.y = toPlayer;

        if (distance > ENEMY_ATTACK_RANGE * 1.5) {
          actor.mood = "pursuing";
          continue;
        }

        if (actor.attackCooldown <= 0) {
          const lunge = Math.sin(performance.now() * 0.012) * 0.08;
          actor.model.position.y += lunge;

          onPlayerDamage?.(ENEMY_ATTACK_DAMAGE, actor.id);
          actor.attackCooldown = ENEMY_ATTACK_COOLDOWN;
        }
      }

      if (actor.mood === "dormant") {
        actor.model.position.y = actor.homePosition.y + Math.sin(actor.pulse) * 0.05;
        actor.model.rotation.y =
          actor.facing + Math.sin(actor.pulse * 0.45) * 0.08;
      }
    }

    for (const actor of toRemove) {
      actorRoot.remove(actor.model);
      actor.model.traverse?.((node) => {
        if (node.geometry && !isSharedResource(node.geometry)) {
          node.geometry.dispose();
        }
        if (node.material && !isSharedResource(node.material)) {
          node.material.dispose();
        }
      });
    }
    if (toRemove.length > 0) {
      for (const removed of toRemove) {
        const idx = actors.indexOf(removed);
        if (idx !== -1) {
          actors.splice(idx, 1);
        }
      }
    }

    state.enemyPresence = getAliveCount();
  }

  function getDebugState(playerPosition) {
    let nearest = null;
    for (const actor of actors) {
      if (actor.mood === "dead") {
        continue;
      }
      const distance = actor.model.position.distanceTo(playerPosition);
      if (!nearest || distance < nearest.distance) {
        nearest = {
          id: actor.id,
          role: actor.role,
          mood: actor.mood,
          hp: actor.hp,
          distance,
        };
      }
    }

    return {
      count: actors.length,
      alive: getAliveCount(),
      nearest: nearest
        ? {
            id: nearest.id,
            role: nearest.role,
            mood: nearest.mood,
            hp: Number(nearest.hp.toFixed(2)),
            distance: Number(nearest.distance.toFixed(2)),
          }
        : null,
    };
  }

  return {
    clear,
    damageActor,
    getActorsForCombat,
    getAliveCount,
    getDebugState,
    rebuild,
    update,
  };
}

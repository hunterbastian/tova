import * as THREE from "three";
import { ImprovedNoise } from "three/examples/jsm/math/ImprovedNoise.js";

import {
  SPAWN_BLEND_RADIUS,
  SPAWN_RADIUS,
  WORLD_SEGMENTS,
  WORLD_SIZE,
} from "./constants.js";

const terrainPalette = {
  grass: new THREE.Color("#697847"),
  spawn: new THREE.Color("#8fa358"),
  forest: new THREE.Color("#4a583a"),
  highland: new THREE.Color("#776d5d"),
  slope: new THREE.Color("#666459"),
  dry: new THREE.Color("#8d7758"),
};

function markShared(resource) {
  resource.userData = resource.userData || {};
  resource.userData.tovaShared = true;
  return resource;
}

function isSharedResource(resource) {
  return resource?.userData?.tovaShared === true;
}

function createLitMaterial(safeMode, standardConfig, basicConfig = {}) {
  if (safeMode) {
    return new THREE.MeshBasicMaterial({ color: standardConfig.color, ...basicConfig });
  }

  return new THREE.MeshStandardMaterial(standardConfig);
}

function getOrCreateCachedMaterial(cache, key, factory) {
  let material = cache.get(key);
  if (!material) {
    material = markShared(factory());
    cache.set(key, material);
  }
  return material;
}

export function createWorldSystem({ scene, safeMode, state, createPedestalSword, collisionSystem }) {
  const worldRoot = new THREE.Group();
  scene.add(worldRoot);

  const terrainSampler = new ImprovedNoise();
  const sharedAssets = (() => {
    const terrainGeometry = markShared(
      new THREE.PlaneGeometry(WORLD_SIZE, WORLD_SIZE, WORLD_SEGMENTS, WORLD_SEGMENTS),
    );
    terrainGeometry.rotateX(-Math.PI / 2);
    terrainGeometry.setAttribute(
      "color",
      new THREE.Float32BufferAttribute(
        new Float32Array((WORLD_SEGMENTS + 1) * (WORLD_SEGMENTS + 1) * 3),
        3,
      ),
    );

    return {
      terrain: {
        geometry: terrainGeometry,
        material: markShared(
          safeMode
            ? new THREE.MeshBasicMaterial({ vertexColors: true, fog: true })
            : new THREE.MeshStandardMaterial({
                vertexColors: true,
                roughness: 0.96,
                metalness: 0.02,
                flatShading: false,
              }),
        ),
      },
      rock: {
        geometry: markShared(new THREE.DodecahedronGeometry(1, 0)),
        materials: new Map(),
      },
      brazier: {
        bowlGeometry: markShared(new THREE.CylinderGeometry(0.22, 0.3, 0.24, 8)),
        stemGeometry: markShared(new THREE.CylinderGeometry(0.06, 0.08, 1.1, 6)),
        flameGeometry: markShared(new THREE.SphereGeometry(0.14, 12, 10)),
        bowlMaterial: markShared(createLitMaterial(safeMode, { color: "#50443a", roughness: 0.92 })),
        stemMaterial: markShared(createLitMaterial(safeMode, { color: "#70645a", roughness: 0.94 })),
        flameMaterial: markShared(new THREE.MeshBasicMaterial({ color: "#f6c56d" })),
      },
      shrine: {
        stoneMaterial: markShared(
          createLitMaterial(safeMode, {
            color: "#84796f",
            roughness: 0.96,
            metalness: 0.04,
          }),
        ),
        daisGeometry: markShared(new THREE.CylinderGeometry(1.7, 2.1, 0.72, 10)),
        altarGeometry: markShared(new THREE.BoxGeometry(0.86, 1.28, 0.86)),
        steleGeometry: markShared(new THREE.BoxGeometry(1.2, 2.8, 0.34)),
        archPostGeometry: markShared(new THREE.BoxGeometry(0.28, 2.2, 0.28)),
        archCapGeometry: markShared(new THREE.BoxGeometry(2.18, 0.26, 0.28)),
        pathStoneGeometry: markShared(new THREE.BoxGeometry(1, 1, 1)),
      },
    };
  })();

  function mulberry32(seed) {
    let t = seed >>> 0;
    return () => {
      t += 0x6d2b79f5;
      let value = Math.imul(t ^ (t >>> 15), t | 1);
      value ^= value + Math.imul(value ^ (value >>> 7), value | 61);
      return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
    };
  }

  function randomSeed() {
    return (crypto.getRandomValues(new Uint32Array(1))[0] ^ Math.floor(performance.now())) >>> 0;
  }

  function smootherstep(t) {
    return t * t * t * (t * (t * 6 - 15) + 10);
  }

  function lerp(a, b, t) {
    return a + (b - a) * t;
  }

  function distance2d(a, b) {
    const dx = a.x - b.x;
    const dz = a.z - b.z;
    return Math.hypot(dx, dz);
  }

  function buildTerrainContext(seed) {
    const rng = mulberry32(seed);
    const offsetX = rng() * 1000;
    const offsetZ = rng() * 1000;
    const ridgeOffset = rng() * 800 + 200;
    const forestCenter = new THREE.Vector3(40 + rng() * 16, 0, 12 + rng() * 16);
    const castleCenter = new THREE.Vector3(-14 - rng() * 10, 0, -44 - rng() * 10);
    const mountainPeak = new THREE.Vector3(28 + rng() * 18, 0, -94 - rng() * 18);
    const castleLengthSq = castleCenter.x * castleCenter.x + castleCenter.z * castleCenter.z;

    function sampleHeight(x, z) {
      const broad = terrainSampler.noise((x + offsetX) / 70, 0.15, (z + offsetZ) / 70) * 7.5;
      const hills = terrainSampler.noise((x + offsetX) / 24, 0.32, (z + offsetZ) / 24) * 3.2;
      const ridge = terrainSampler.noise((x - ridgeOffset) / 11, 0.52, (z + ridgeOffset * 0.35) / 11) * 2.1;
      const peakDistance = Math.hypot(x - mountainPeak.x, z - mountainPeak.z);
      const peakLift = Math.max(0, 1 - peakDistance / 70);
      const forestLift = Math.max(0, 1 - distance2d({ x, z }, forestCenter) / 22) * 1.15;
      const castleLift = Math.max(0, 1 - distance2d({ x, z }, castleCenter) / 18) * 1.8;

      let height = 8 + broad + hills + ridge;
      height += smootherstep(peakLift) * 16;
      height += forestLift;
      height += castleLift;

      const spawnDistance = Math.hypot(x, z);
      if (spawnDistance < SPAWN_BLEND_RADIUS) {
        const target = 8.6 + terrainSampler.noise((x + offsetX) / 18, 0.12, (z + offsetZ) / 18) * 0.16;
        const blend = smootherstep(Math.max(0, Math.min(1, spawnDistance / SPAWN_BLEND_RADIUS)));
        height = lerp(target, height, blend);
        if (spawnDistance < SPAWN_RADIUS) {
          height = target;
        }
      }

      const castleDistance = Math.hypot(x - castleCenter.x, z - castleCenter.z);
      if (castleDistance < 16) {
        const plateau = 12 + terrainSampler.noise((x + offsetX) / 30, 0.1, (z + offsetZ) / 30) * 0.32;
        const blend = smootherstep(castleDistance / 16);
        height = lerp(plateau, height, blend);
      }

      if (castleLengthSq > 0) {
        const projection =
          ((x * castleCenter.x + z * castleCenter.z) / castleLengthSq);
        const clampedProjection = Math.max(0, Math.min(1, projection));
        const nearestX = castleCenter.x * clampedProjection;
        const nearestZ = castleCenter.z * clampedProjection;
        const viewDistance = Math.hypot(x - nearestX, z - nearestZ);
        const inViewLane = clampedProjection > 0.12 && clampedProjection < 0.88 && viewDistance < 12;

        if (inViewLane) {
          const laneTarget =
            8.4 +
            terrainSampler.noise((x + offsetX) / 20, 0.12, (z + offsetZ) / 20) * 0.12 +
            clampedProjection * 0.6;
          const laneBlend = 1 - smootherstep(viewDistance / 12);
          height = lerp(height, laneTarget, laneBlend * 0.82);
        }
      }

      return height;
    }

    forestCenter.y = sampleHeight(forestCenter.x, forestCenter.z);
    castleCenter.y = sampleHeight(castleCenter.x, castleCenter.z);
    mountainPeak.y = sampleHeight(mountainPeak.x, mountainPeak.z);

    return { sampleHeight, forestCenter, castleCenter, mountainPeak, rng };
  }

  function clearWorld() {
    while (worldRoot.children.length > 0) {
      const child = worldRoot.children[worldRoot.children.length - 1];
      if (!child) {
        continue;
      }

      worldRoot.remove(child);
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
  }

  function buildTerrain(seed) {
    const context = buildTerrainContext(seed);
    const geometry = sharedAssets.terrain.geometry;
    const positions = geometry.attributes.position;
    const colors = geometry.attributes.color.array;

    for (let index = 0; index < positions.count; index += 1) {
      const x = positions.getX(index);
      const z = positions.getZ(index);
      const y = context.sampleHeight(x, z);
      positions.setY(index, y);

      const forestDistance = Math.hypot(x - context.forestCenter.x, z - context.forestCenter.z);
      const spawnDistance = Math.hypot(x, z);
      const moisture = terrainSampler.noise((x + seed) / 16, 1.4, (z - seed) / 16) * 0.5 + 0.5;
      let color = terrainPalette.grass;

      if (spawnDistance < SPAWN_BLEND_RADIUS + 8) {
        color = terrainPalette.spawn;
      } else if (forestDistance < 26) {
        color = terrainPalette.forest;
      } else if (y > 20) {
        color = terrainPalette.highland;
      } else if (y > 15) {
        color = terrainPalette.slope;
      } else if (moisture < 0.32) {
        color = terrainPalette.dry;
      }

      const colorOffset = index * 3;
      colors[colorOffset] = color.r;
      colors[colorOffset + 1] = color.g;
      colors[colorOffset + 2] = color.b;
    }

    positions.needsUpdate = true;
    geometry.attributes.color.needsUpdate = true;
    geometry.computeVertexNormals();

    if (!state.terrainMesh) {
      state.terrainMesh = new THREE.Mesh(geometry, sharedAssets.terrain.material);
      state.terrainMesh.receiveShadow = true;
      state.terrainMesh.castShadow = false;
    }

    worldRoot.add(state.terrainMesh);
    state.terrainContext = context;
    state.forestCenter.copy(context.forestCenter);
    state.castleCenter.copy(context.castleCenter);
  }

  function createRock(x, y, z, scale, color = "#6e6a63") {
    const material = getOrCreateCachedMaterial(
      sharedAssets.rock.materials,
      color,
      () =>
        createLitMaterial(
          safeMode,
          {
            color,
            roughness: 1,
            metalness: 0.02,
            flatShading: true,
          },
          { fog: true },
        ),
    );
    const rock = new THREE.Mesh(sharedAssets.rock.geometry, material);
    rock.castShadow = true;
    rock.receiveShadow = true;
    rock.scale.setScalar(scale);
    rock.position.set(x, y + scale * 0.5, z);
    collisionSystem.addCylinder(x, z, scale * 0.55);
    worldRoot.add(rock);
  }

  function createBrazier(position) {
    const brazier = new THREE.Group();

    const bowl = new THREE.Mesh(sharedAssets.brazier.bowlGeometry, sharedAssets.brazier.bowlMaterial);
    bowl.castShadow = true;
    bowl.receiveShadow = true;
    bowl.position.y = 1.25;
    brazier.add(bowl);

    const stem = new THREE.Mesh(sharedAssets.brazier.stemGeometry, sharedAssets.brazier.stemMaterial);
    stem.position.y = 0.55;
    stem.castShadow = true;
    stem.receiveShadow = true;
    brazier.add(stem);

    const flame = new THREE.Mesh(sharedAssets.brazier.flameGeometry, sharedAssets.brazier.flameMaterial);
    flame.position.y = 1.32;
    brazier.add(flame);

    const light = new THREE.PointLight("#f0bf63", 1.2, 13, 2);
    light.position.y = 1.5;
    brazier.add(light);

    brazier.position.copy(position);
    worldRoot.add(brazier);
  }

  function buildSpawnSanctum(seed) {
    const rng = mulberry32(seed ^ 0xa7810d3f);
    const shrine = new THREE.Group();
    const shrineX = 7.6 + rng() * 1.4;
    const shrineZ = 2.4 + rng() * 1.2;
    const shrineY = state.terrainContext.sampleHeight(shrineX, shrineZ);
    shrine.position.set(shrineX, shrineY, shrineZ);

    const dais = new THREE.Mesh(sharedAssets.shrine.daisGeometry, sharedAssets.shrine.stoneMaterial);
    dais.position.y = 0.36;
    dais.castShadow = true;
    dais.receiveShadow = true;
    shrine.add(dais);

    const altar = new THREE.Mesh(sharedAssets.shrine.altarGeometry, sharedAssets.shrine.stoneMaterial);
    altar.position.y = 1.05;
    altar.castShadow = true;
    altar.receiveShadow = true;
    shrine.add(altar);

    const stele = new THREE.Mesh(sharedAssets.shrine.steleGeometry, sharedAssets.shrine.stoneMaterial);
    stele.position.set(0, 1.8, 1.1);
    stele.castShadow = true;
    stele.receiveShadow = true;
    shrine.add(stele);

    const archLeft = new THREE.Mesh(sharedAssets.shrine.archPostGeometry, sharedAssets.shrine.stoneMaterial);
    archLeft.position.set(-0.95, 1.4, 0.82);
    archLeft.castShadow = true;
    archLeft.receiveShadow = true;
    shrine.add(archLeft);

    const archRight = archLeft.clone();
    archRight.position.x = 0.95;
    shrine.add(archRight);

    const archCap = new THREE.Mesh(sharedAssets.shrine.archCapGeometry, sharedAssets.shrine.stoneMaterial);
    archCap.position.set(0, 2.45, 0.82);
    archCap.castShadow = true;
    archCap.receiveShadow = true;
    shrine.add(archCap);

    const pedestalSword = createPedestalSword();
    pedestalSword.position.set(0, 1.82, 0);
    pedestalSword.rotation.z = 0.05;
    shrine.add(pedestalSword);
    state.swordPedestalSword = pedestalSword;
    state.swordPickupPosition.set(shrineX, shrineY + 1.6, shrineZ);

    collisionSystem.addBox(shrineX, shrineZ, 0.5, 0.5);
    collisionSystem.addBox(shrineX, shrineZ + 1.1, 0.65, 0.2);
    collisionSystem.addCylinder(shrineX - 0.95, shrineZ + 0.82, 0.2);
    collisionSystem.addCylinder(shrineX + 0.95, shrineZ + 0.82, 0.2);

    createBrazier(new THREE.Vector3(shrineX - 1.7, shrineY + 0.02, shrineZ + 0.8));
    createBrazier(new THREE.Vector3(shrineX + 1.7, shrineY + 0.02, shrineZ + 0.8));

    for (let index = 0; index < 5; index += 1) {
      const stone = new THREE.Mesh(sharedAssets.shrine.pathStoneGeometry, sharedAssets.shrine.stoneMaterial);
      stone.scale.set(0.44 + rng() * 0.18, 0.12, 0.62 + rng() * 0.22);
      stone.position.set(
        THREE.MathUtils.lerp(0, shrineX, index / 6),
        state.terrainContext.sampleHeight(
          THREE.MathUtils.lerp(0, shrineX, index / 6),
          THREE.MathUtils.lerp(0, shrineZ, index / 6),
        ) + 0.05,
        THREE.MathUtils.lerp(0, shrineZ, index / 6),
      );
      stone.rotation.y = rng() * Math.PI;
      stone.castShadow = true;
      stone.receiveShadow = true;
      worldRoot.add(stone);
    }

    worldRoot.add(shrine);

    /* ── scatter rocks + grass tufts around spawn ─────── */
    for (let index = 0; index < 14; index += 1) {
      const angle = rng() * Math.PI * 2;
      const dist = SPAWN_RADIUS + 2 + rng() * (SPAWN_BLEND_RADIUS - SPAWN_RADIUS + 6);
      const rx = Math.cos(angle) * dist;
      const rz = Math.sin(angle) * dist;
      const ry = state.terrainContext.sampleHeight(rx, rz);
      createRock(rx, ry, rz, 0.25 + rng() * 0.45);
    }

    const grassGeometry = new THREE.ConeGeometry(0.15, 0.55, 4);
    const grassMaterial = createLitMaterial(safeMode,
      { color: "#7a9e52", roughness: 0.96 },
      { fog: true },
    );
    const grassCount = 180;
    const grassMesh = new THREE.InstancedMesh(grassGeometry, grassMaterial, grassCount);
    grassMesh.receiveShadow = true;
    const grassMatrix = new THREE.Matrix4();
    for (let index = 0; index < grassCount; index += 1) {
      const angle = rng() * Math.PI * 2;
      const dist = 1.5 + rng() * (SPAWN_BLEND_RADIUS + 10);
      const gx = Math.cos(angle) * dist;
      const gz = Math.sin(angle) * dist;
      const gy = state.terrainContext.sampleHeight(gx, gz);
      grassMatrix.compose(
        new THREE.Vector3(gx, gy + 0.22, gz),
        new THREE.Quaternion().setFromAxisAngle(
          new THREE.Vector3(0, 1, 0),
          rng() * Math.PI * 2,
        ),
        new THREE.Vector3(0.6 + rng() * 0.6, 0.7 + rng() * 0.8, 0.6 + rng() * 0.6),
      );
      grassMesh.setMatrixAt(index, grassMatrix);
    }
    grassMesh.instanceMatrix.needsUpdate = true;
    worldRoot.add(grassMesh);
  }

  function buildForest(seed) {
    const rng = mulberry32(seed ^ 0x1f123bb5);
    const treeCount = 220;
    const trunkGeometry = new THREE.CylinderGeometry(0.18, 0.28, 2.8, 7);
    const canopyGeometry = new THREE.ConeGeometry(1.35, 3.8, 8);
    const trunkMaterial = createLitMaterial(safeMode, { color: "#54402d", roughness: 1 });
    const canopyMaterial = createLitMaterial(safeMode, { color: "#36472d", roughness: 0.96 });
    const trunkMesh = new THREE.InstancedMesh(trunkGeometry, trunkMaterial, treeCount);
    const canopyMesh = new THREE.InstancedMesh(canopyGeometry, canopyMaterial, treeCount);
    trunkMesh.castShadow = true;
    trunkMesh.receiveShadow = true;
    canopyMesh.castShadow = true;
    canopyMesh.receiveShadow = true;

    const matrix = new THREE.Matrix4();
    let placed = 0;
    while (placed < treeCount) {
      const angle = rng() * Math.PI * 2;
      const distance = 4 + Math.sqrt(rng()) * 18;
      const x = state.forestCenter.x + Math.cos(angle) * distance;
      const z = state.forestCenter.z + Math.sin(angle) * distance;
      const spawnDistance = Math.hypot(x, z);
      const castleDistance = Math.hypot(x - state.castleCenter.x, z - state.castleCenter.z);
      if (spawnDistance < SPAWN_BLEND_RADIUS + 4 || castleDistance < 16) {
        continue;
      }

      const y = state.terrainContext.sampleHeight(x, z);
      const trunkHeight = 2 + rng() * 1.6;
      const canopyHeight = 3.1 + rng() * 1.4;
      const canopyScale = 0.8 + rng() * 0.55;

      matrix.compose(
        new THREE.Vector3(x, y + trunkHeight * 0.5, z),
        new THREE.Quaternion(),
        new THREE.Vector3(1, trunkHeight / 2.8, 1),
      );
      trunkMesh.setMatrixAt(placed, matrix);

      matrix.compose(
        new THREE.Vector3(x, y + trunkHeight + canopyHeight * 0.42, z),
        new THREE.Quaternion(),
        new THREE.Vector3(canopyScale, canopyHeight / 3.8, canopyScale),
      );
      canopyMesh.setMatrixAt(placed, matrix);
      collisionSystem.addCylinder(x, z, 0.38);
      placed += 1;
    }

    trunkMesh.instanceMatrix.needsUpdate = true;
    canopyMesh.instanceMatrix.needsUpdate = true;
    worldRoot.add(trunkMesh, canopyMesh);

    for (let index = 0; index < 28; index += 1) {
      const angle = rng() * Math.PI * 2;
      const distance = 8 + rng() * 24;
      const x = state.forestCenter.x + Math.cos(angle) * distance;
      const z = state.forestCenter.z + Math.sin(angle) * distance;
      const y = state.terrainContext.sampleHeight(x, z);
      createRock(x, y, z, 0.45 + rng() * 0.55);
    }
  }

  function buildCastle(seed) {
    const rng = mulberry32(seed ^ 0x9e3779b9);
    const castle = new THREE.Group();
    const wallMaterial = createLitMaterial(safeMode, {
      color: "#68675f",
      roughness: 0.95,
      metalness: 0.03,
    });
    const roofMaterial = createLitMaterial(safeMode, {
      color: "#3f3933",
      roughness: 0.92,
      metalness: 0.01,
    });

    const baseY = state.terrainContext.sampleHeight(state.castleCenter.x, state.castleCenter.z);
    castle.position.set(state.castleCenter.x, baseY, state.castleCenter.z);

    const courtyard = new THREE.Mesh(new THREE.BoxGeometry(22, 1.9, 18), wallMaterial);
    courtyard.position.set(0, 0.8, 0);
    courtyard.receiveShadow = true;
    courtyard.castShadow = true;
    castle.add(courtyard);

    const wallSegments = [
      { size: [22, 5.8, 1.3], position: [0, 3.8, -8.4] },
      { size: [22, 5.8, 1.3], position: [0, 3.8, 8.4] },
      { size: [1.3, 5.8, 15.5], position: [-10.3, 3.8, 0] },
      { size: [1.3, 5.8, 15.5], position: [10.3, 3.8, 0] },
      { size: [8.4, 8.6, 6.6], position: [0, 5.4, 0.5] },
    ];

    for (const segment of wallSegments) {
      const wall = new THREE.Mesh(new THREE.BoxGeometry(...segment.size), wallMaterial);
      wall.position.set(...segment.position);
      wall.castShadow = true;
      wall.receiveShadow = true;
      castle.add(wall);
    }

    const towerOffsets = [
      [-9.4, 0, -7.7],
      [9.4, 0, -7.7],
      [-9.4, 0, 7.7],
      [9.4, 0, 7.7],
    ];

    for (const [x, y, z] of towerOffsets) {
      const tower = new THREE.Mesh(new THREE.CylinderGeometry(1.95, 2.15, 11.4, 10), wallMaterial);
      tower.position.set(x, 5.7 + y, z);
      tower.castShadow = true;
      tower.receiveShadow = true;
      castle.add(tower);

      const roof = new THREE.Mesh(new THREE.ConeGeometry(2.85, 3.8, 10), roofMaterial);
      roof.position.set(x, 12.8 + y, z);
      roof.castShadow = true;
      roof.receiveShadow = true;
      castle.add(roof);
    }

    const gate = new THREE.Mesh(new THREE.BoxGeometry(5.1, 5.4, 1.5), roofMaterial);
    gate.position.set(0, 3.1, 8.5);
    gate.castShadow = true;
    gate.receiveShadow = true;
    castle.add(gate);

    /* ── castle colliders (world-space) ──────────────── */
    const cx = state.castleCenter.x;
    const cz = state.castleCenter.z;
    // back wall
    collisionSystem.addBox(cx, cz - 8.4, 11, 0.65);
    // front wall — split for gate opening (gate width 5.1)
    collisionSystem.addBox(cx - 6.775, cz + 8.4, 4.225, 0.65);
    collisionSystem.addBox(cx + 6.775, cz + 8.4, 4.225, 0.65);
    // side walls
    collisionSystem.addBox(cx - 10.3, cz, 0.65, 7.75);
    collisionSystem.addBox(cx + 10.3, cz, 0.65, 7.75);
    // keep
    collisionSystem.addBox(cx, cz + 0.5, 4.2, 3.3);
    // corner towers
    for (const [tx, , tz] of towerOffsets) {
      collisionSystem.addCylinder(cx + tx, cz + tz, 2.2);
    }

    for (let index = 0; index < 10; index += 1) {
      const angle = rng() * Math.PI * 2;
      const distance = 14 + rng() * 12;
      const x = state.castleCenter.x + Math.cos(angle) * distance;
      const z = state.castleCenter.z + Math.sin(angle) * distance;
      const y = state.terrainContext.sampleHeight(x, z);
      createRock(x, y, z, 0.5 + rng() * 0.7, "#7a756c");
    }

    worldRoot.add(castle);
  }

  function buildHazeAndLandmarks(seed) {
    const rng = mulberry32(seed ^ 0x53142fcd);
    const mist = new THREE.Group();
    const mistMaterial = new THREE.MeshBasicMaterial({
      color: "#8f987f",
      transparent: true,
      opacity: 0.11,
      depthWrite: false,
    });

    for (let index = 0; index < 12; index += 1) {
      const sphere = new THREE.Mesh(new THREE.SphereGeometry(10 + rng() * 12, 18, 14), mistMaterial);
      sphere.position.set(-40 + rng() * 120, 12 + rng() * 10, -20 + rng() * 120);
      sphere.scale.set(1.7, 0.44, 1.1);
      mist.add(sphere);
    }

    const obeliskMaterial = createLitMaterial(safeMode, { color: "#7c6f5d", roughness: 0.96 });
    const obelisk = new THREE.Mesh(new THREE.BoxGeometry(2.4, 9, 2.4), obeliskMaterial);
    obelisk.position.set(22, state.terrainContext.sampleHeight(22, 12) + 4.5, 12);
    collisionSystem.addBox(22, 12, 1.3, 1.3);
    obelisk.castShadow = true;
    obelisk.receiveShadow = true;
    mist.add(obelisk);

    const ruinMaterial = createLitMaterial(safeMode, { color: "#756857", roughness: 0.96 });
    for (let index = 0; index < 3; index += 1) {
      const ruin = new THREE.Group();
      const originX = -18 + rng() * 52;
      const originZ = 28 + rng() * 46;
      const originY = state.terrainContext.sampleHeight(originX, originZ);
      ruin.position.set(originX, originY, originZ);

      const left = new THREE.Mesh(new THREE.BoxGeometry(0.42, 3.4, 0.48), ruinMaterial);
      left.position.set(-1.2, 1.7, 0);
      left.castShadow = true;
      left.receiveShadow = true;
      ruin.add(left);

      const right = left.clone();
      right.position.x = 1.2;
      ruin.add(right);

      const top = new THREE.Mesh(new THREE.BoxGeometry(2.8, 0.42, 0.52), ruinMaterial);
      top.position.set(0, 3.3, 0);
      top.castShadow = true;
      top.receiveShadow = true;
      ruin.add(top);

      mist.add(ruin);
    }

    worldRoot.add(mist);
  }

  function regenerateWorld() {
    state.seed = randomSeed();
    state.swordPedestalSword = null;
    collisionSystem.clear();
    clearWorld();
    buildTerrain(state.seed);
    buildSpawnSanctum(state.seed);
    buildForest(state.seed);
    buildCastle(state.seed);
    buildHazeAndLandmarks(state.seed);
  }

  function sampleGroundHeight(x, z) {
    return state.terrainContext.sampleHeight(x, z);
  }

  return { regenerateWorld, sampleGroundHeight };
}

import * as THREE from 'three';
import { mergeGeometries } from 'three/examples/jsm/utils/BufferGeometryUtils.js';

export class Castle {
    constructor(scene, terrain, options = {}) {
        this.scene = scene;
        this.terrain = terrain;
        this.group = new THREE.Group();
        this.enableShadows = options.enableShadows ?? true;
        this.glowLight = null;
        this.init();
    }

    init() {
        // Castle Material - Stone
        const stoneMaterial = new THREE.MeshStandardMaterial({
            color: 0x888888,
            roughness: 0.9,
            metalness: 0.1
        });

        const geometries = [];
        const tempPos = new THREE.Vector3();
        const tempQuat = new THREE.Quaternion();
        const tempEuler = new THREE.Euler();
        const tempMatrix = new THREE.Matrix4();

        const addGeometry = (geometry, x, y, z, rotationY = 0) => {
            const cloned = geometry.clone();
            tempPos.set(x, y, z);
            tempEuler.set(0, rotationY, 0);
            tempQuat.setFromEuler(tempEuler);
            tempMatrix.compose(tempPos, tempQuat, new THREE.Vector3(1, 1, 1));
            cloned.applyMatrix4(tempMatrix);
            geometries.push(cloned);
        };

        // Main Keep
        const keepGeo = new THREE.BoxGeometry(15, 30, 15);
        addGeometry(keepGeo, 0, 15, 0);

        // Towers at corners
        const towerGeo = new THREE.CylinderGeometry(4, 5, 25, 8);
        const towerPositions = [
            { x: 15, z: 15 },
            { x: -15, z: 15 },
            { x: 15, z: -15 },
            { x: -15, z: -15 }
        ];

        towerPositions.forEach((pos) => {
            addGeometry(towerGeo, pos.x, 12.5, pos.z);
        });

        // Walls connecting towers
        const wallGeo = new THREE.BoxGeometry(30, 15, 2);
        const wallPositions = [
            { x: 0, z: 15, rot: 0 },
            { x: 0, z: -15, rot: 0 },
            { x: 15, z: 0, rot: Math.PI / 2 },
            { x: -15, z: 0, rot: Math.PI / 2 }
        ];

        wallPositions.forEach((pos) => {
            addGeometry(wallGeo, pos.x, 7.5, pos.z, pos.rot);
        });

        const merged = mergeGeometries(geometries, false);
        if (merged) {
            const castleMesh = new THREE.Mesh(merged, stoneMaterial);
            castleMesh.castShadow = this.enableShadows;
            castleMesh.receiveShadow = this.enableShadows;
            castleMesh.matrixAutoUpdate = false;
            castleMesh.updateMatrix();
            this.group.add(castleMesh);
        }

        // Position the entire castle on top of the hill (0,0)
        // We know the hill is at 0,0 and roughly height 40 from Terrain.js
        // But let's use getHeightAt to be safe if we move it
        const hillHeight = this.terrain.getHeightAt(0, 0);
        this.group.position.set(0, hillHeight, 0);

        this.group.traverse((child) => {
            if (child.isMesh) {
                child.matrixAutoUpdate = false;
                child.updateMatrix();
            }
        });
        this.group.matrixAutoUpdate = false;
        this.group.updateMatrix();

        this.scene.add(this.group);

        this.glowLight = new THREE.PointLight(0xffc98b, 0, 120, 2);
        this.glowLight.position.set(0, 22, 0);
        this.group.add(this.glowLight);
    }

    setNightGlow(intensity) {
        if (!this.glowLight) return;
        this.glowLight.intensity = Math.max(0, intensity);
    }
}

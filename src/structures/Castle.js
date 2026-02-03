import * as THREE from 'three';

export class Castle {
    constructor(scene, terrain, options = {}) {
        this.scene = scene;
        this.terrain = terrain;
        this.group = new THREE.Group();
        this.enableShadows = options.enableShadows ?? true;
        this.init();
    }

    init() {
        // Castle Material - Stone
        const stoneMaterial = new THREE.MeshStandardMaterial({
            color: 0x888888,
            roughness: 0.9,
            metalness: 0.1
        });

        // Main Keep
        const keepGeo = new THREE.BoxGeometry(15, 30, 15);
        const keep = new THREE.Mesh(keepGeo, stoneMaterial);
        keep.position.y = 15; // Half height
        keep.castShadow = this.enableShadows;
        keep.receiveShadow = this.enableShadows;
        this.group.add(keep);

        // Towers at corners
        const towerGeo = new THREE.CylinderGeometry(4, 5, 25, 8);
        const towerPositions = [
            { x: 15, z: 15 },
            { x: -15, z: 15 },
            { x: 15, z: -15 },
            { x: -15, z: -15 }
        ];

        towerPositions.forEach(pos => {
            const tower = new THREE.Mesh(towerGeo, stoneMaterial);
            tower.position.set(pos.x, 12.5, pos.z);
            tower.castShadow = this.enableShadows;
            tower.receiveShadow = this.enableShadows;
            this.group.add(tower);
        });

        // Walls connecting towers
        const wallGeo = new THREE.BoxGeometry(30, 15, 2);
        const wallPositions = [
            { x: 0, z: 15, rot: 0 },
            { x: 0, z: -15, rot: 0 },
            { x: 15, z: 0, rot: Math.PI / 2 },
            { x: -15, z: 0, rot: Math.PI / 2 }
        ];

        wallPositions.forEach(pos => {
            const wall = new THREE.Mesh(wallGeo, stoneMaterial);
            wall.position.set(pos.x, 7.5, pos.z);
            wall.rotation.y = pos.rot;
            wall.castShadow = false;
            wall.receiveShadow = this.enableShadows;
            this.group.add(wall);
        });

        // Position the entire castle on top of the hill (0,0)
        // We know the hill is at 0,0 and roughly height 40 from Terrain.js
        // But let's use getHeightAt to be safe if we move it
        const hillHeight = this.terrain.getHeightAt(0, 0);
        this.group.position.set(0, hillHeight, 0);

        this.scene.add(this.group);
    }
}

import * as THREE from 'three';

export class Town {
    constructor(scene, terrain) {
        this.scene = scene;
        this.terrain = terrain;
        this.group = new THREE.Group();
        this.init();
    }

    init() {
        const houseMaterial = new THREE.MeshStandardMaterial({
            color: 0x8B4513, // Wood
            roughness: 1.0,
            metalness: 0.0,
            flatShading: true
        });
        const roofMaterial = new THREE.MeshStandardMaterial({
            color: 0x800000, // Red roof
            roughness: 1.0,
            metalness: 0.0,
            flatShading: true
        });

        const houseGeo = new THREE.BoxGeometry(4, 4, 4);
        const roofGeo = new THREE.ConeGeometry(3.5, 2, 4);

        // Generate houses in the "flat" area defined in Terrain.js
        // x: 20 to 100, z: -50 to 50

        for (let i = 0; i < 20; i++) {
            const x = 30 + Math.random() * 60;
            const z = -40 + Math.random() * 80;

            const y = this.terrain.getHeightAt(x, z);

            // Simple House Group
            const houseGroup = new THREE.Group();

            const base = new THREE.Mesh(houseGeo, houseMaterial);
            base.position.y = 2;
            base.castShadow = true;
            base.receiveShadow = true;
            houseGroup.add(base);

            const roof = new THREE.Mesh(roofGeo, roofMaterial);
            roof.position.y = 4 + 1;
            roof.rotation.y = Math.PI / 4;
            roof.castShadow = true;
            houseGroup.add(roof);

            houseGroup.position.set(x, y, z);
            houseGroup.rotation.y = Math.random() * Math.PI * 2;

            this.group.add(houseGroup);
        }

        this.scene.add(this.group);
    }
}

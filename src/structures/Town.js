import * as THREE from 'three';

export class Town {
    constructor(scene, terrain, options = {}) {
        this.scene = scene;
        this.terrain = terrain;
        this.group = new THREE.Group();
        this.enableShadows = options.enableShadows ?? true;
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

        const count = 20;
        const baseMesh = new THREE.InstancedMesh(houseGeo, houseMaterial, count);
        const roofMesh = new THREE.InstancedMesh(roofGeo, roofMaterial, count);
        baseMesh.castShadow = false;
        roofMesh.castShadow = false;
        baseMesh.receiveShadow = this.enableShadows;
        roofMesh.receiveShadow = this.enableShadows;

        const dummy = new THREE.Object3D();

        for (let i = 0; i < count; i++) {
            const x = 30 + Math.random() * 60;
            const z = -40 + Math.random() * 80;

            const y = this.terrain.getHeightAt(x, z);

            const rotation = Math.random() * Math.PI * 2;

            dummy.position.set(x, y + 2, z);
            dummy.rotation.set(0, rotation, 0);
            dummy.updateMatrix();
            baseMesh.setMatrixAt(i, dummy.matrix);

            dummy.position.set(x, y + 5, z);
            dummy.rotation.set(0, rotation + Math.PI / 4, 0);
            dummy.updateMatrix();
            roofMesh.setMatrixAt(i, dummy.matrix);
        }

        baseMesh.instanceMatrix.needsUpdate = true;
        roofMesh.instanceMatrix.needsUpdate = true;

        this.group.add(baseMesh);
        this.group.add(roofMesh);

        this.scene.add(this.group);
    }
}

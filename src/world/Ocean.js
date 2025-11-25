import * as THREE from 'three';

export class Ocean {
    constructor(scene) {
        this.scene = scene;
        this.mesh = null;
        this.init();
    }

    init() {
        const geometry = new THREE.PlaneGeometry(1000, 1000);

        // Water material
        const material = new THREE.MeshPhysicalMaterial({
            color: 0x001e0f,
            metalness: 0,
            roughness: 0.1,
            transmission: 0.9, // Glass-like
            thickness: 1.5,
            envMapIntensity: 1.0,
            clearcoat: 1.0,
            clearcoatRoughness: 0.1,
            side: THREE.DoubleSide
        });

        this.mesh = new THREE.Mesh(geometry, material);
        this.mesh.rotation.x = -Math.PI / 2;
        this.mesh.position.y = -15; // Water level below the main terrain

        this.scene.add(this.mesh);
    }

    update(time) {
        // Simple wave animation could go here if we used a custom shader
        // For now, just a static beautiful surface
        if (this.mesh) {
            this.mesh.position.y = -15 + Math.sin(time * 0.5) * 0.5; // Gentle tide
        }
    }
}

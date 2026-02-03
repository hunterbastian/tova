import * as THREE from 'three';

export class Ocean {
    constructor(scene) {
        this.scene = scene;
        this.mesh = null;
        this.init();
    }

    init() {
        const geometry = new THREE.PlaneGeometry(1000, 1000);

        // Cheaper water material for better performance.
        const material = new THREE.MeshStandardMaterial({
            color: 0x0b2a24,
            metalness: 0.0,
            roughness: 0.5
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

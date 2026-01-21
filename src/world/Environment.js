import * as THREE from 'three';

export class Environment {
    constructor(scene) {
        this.scene = scene;
        this.init();
    }

    init() {
        // Sky color (gradient or solid for now)
        this.scene.background = new THREE.Color(0x87ceeb);

        // Fog for depth and atmosphere
        this.scene.fog = new THREE.FogExp2(0xcce0ff, 0.0015);

        // Lighting
        this.setupLights();
    }

    setupLights() {
        const ambientLight = new THREE.AmbientLight(0x404040, 0.6); // Softer ambient
        this.scene.add(ambientLight);

        const hemiLight = new THREE.HemisphereLight(0xffffff, 0x444444, 0.6);
        hemiLight.position.set(0, 200, 0);
        this.scene.add(hemiLight);

        const dirLight = new THREE.DirectionalLight(0xffdfba, 1.5); // Warm sunlight
        dirLight.position.set(100, 100, 50);
        dirLight.castShadow = true;

        // Shadow properties tuned for a balance of quality and performance.
        // On many Macs, a 1024x1024 shadow map is a good compromise between
        // visual quality and framerate.
        dirLight.shadow.mapSize.width = 1024;
        dirLight.shadow.mapSize.height = 1024;
        dirLight.shadow.camera.near = 0.5;
        dirLight.shadow.camera.far = 350;

        const d = 160;
        dirLight.shadow.camera.left = -d;
        dirLight.shadow.camera.right = d;
        dirLight.shadow.camera.top = d;
        dirLight.shadow.camera.bottom = -d;

        // Soft shadows
        dirLight.shadow.radius = 4;
        dirLight.shadow.bias = -0.0005;

        this.scene.add(dirLight);
    }
}

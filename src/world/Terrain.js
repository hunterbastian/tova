import * as THREE from 'three';

export class Terrain {
    constructor(scene) {
        this.scene = scene;
        this.mesh = null;
        this.init();
    }

    init() {
        // Create a large plane for the terrain
        const geometry = new THREE.PlaneGeometry(500, 500, 128, 128);

        // Manipulate vertices to create a hill and some roughness
        const positionAttribute = geometry.attributes.position;
        const vertex = new THREE.Vector3();

        for (let i = 0; i < positionAttribute.count; i++) {
            vertex.fromBufferAttribute(positionAttribute, i);

            // Simple procedural generation
            // Hill in the center (where the castle will be)
            const dist = Math.sqrt(vertex.x * vertex.x + vertex.y * vertex.y); // y is actually z in 2D plane before rotation

            let height = 0;

            // Central Hill
            if (dist < 80) {
                height += 40 * Math.cos((dist / 80) * (Math.PI / 2));
            }

            // Noise/Roughness (Simulated with Math.sin for now, can use a noise library later)
            height += Math.sin(vertex.x * 0.1) * Math.sin(vertex.y * 0.1) * 2;
            height += Math.sin(vertex.x * 0.5) * Math.sin(vertex.y * 0.5) * 0.5;

            // Flatten area for town near the hill
            if (vertex.x > 20 && vertex.x < 100 && vertex.y > -50 && vertex.y < 50) {
                // Smooth out the town area
                height *= 0.5;
            }

            // Ocean drop-off
            if (vertex.x < -100) {
                height -= 20; // Drop down for ocean
            }

            positionAttribute.setZ(i, height); // Set Z because PlaneGeometry is XY by default
        }

        geometry.computeVertexNormals();

        // Material with some texture mixing (placeholder color for now)
        const material = new THREE.MeshStandardMaterial({
            color: 0x3a5f0b,
            roughness: 0.8,
            metalness: 0.1,
            flatShading: false,
            side: THREE.DoubleSide
        });

        this.mesh = new THREE.Mesh(geometry, material);
        this.mesh.rotation.x = -Math.PI / 2;
        this.mesh.receiveShadow = true;
        this.mesh.castShadow = true;

        this.scene.add(this.mesh);
    }

    getHeightAt(x, z) {
        // Raycaster approach is accurate but expensive. 
        // For now, we can approximate or use a raycaster downwards from high up.
        const raycaster = new THREE.Raycaster();
        raycaster.set(new THREE.Vector3(x, 100, z), new THREE.Vector3(0, -1, 0));
        const intersects = raycaster.intersectObject(this.mesh);
        if (intersects.length > 0) {
            return intersects[0].point.y;
        }
        return 0;
    }
}

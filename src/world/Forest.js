import * as THREE from 'three';

export class Forest {
    constructor(scene, terrain, options = {}) {
        this.scene = scene;
        this.terrain = terrain;
        this.enableShadows = options.enableShadows ?? true;
        this.count = options.count ?? 480;
        this.spruceRatio = options.spruceRatio ?? 0.75;
        this.birchRatio = 1 - this.spruceRatio;
        this.region = options.region ?? {
            xMin: -60,
            xMax: 230,
            zMin: -230,
            zMax: 230
        };

        this.init();
    }

    init() {
        // Low-poly Nordic palette: deep spruce greens + cool birch bark.
        const spruceTrunkGeometry = new THREE.CylinderGeometry(0.45, 0.75, 10, 5);
        const spruceLayerGeometry = new THREE.ConeGeometry(2.6, 4.2, 6, 1);

        const birchTrunkGeometry = new THREE.CylinderGeometry(0.2, 0.35, 5.5, 5);
        const birchCanopyGeometry = new THREE.IcosahedronGeometry(1.6, 0);

        const spruceTrunkMaterial = new THREE.MeshStandardMaterial({
            color: 0x3a2c1f,
            roughness: 0.95,
            metalness: 0.0,
            flatShading: true
        });
        const spruceNeedleMaterial = new THREE.MeshStandardMaterial({
            color: 0x24412b,
            roughness: 0.85,
            metalness: 0.0,
            flatShading: true
        });
        const birchTrunkMaterial = new THREE.MeshStandardMaterial({
            color: 0xb6b0a5,
            roughness: 0.9,
            metalness: 0.0,
            flatShading: true
        });
        const birchCanopyMaterial = new THREE.MeshStandardMaterial({
            color: 0x4c6f3d,
            roughness: 0.85,
            metalness: 0.0,
            flatShading: true
        });

        const spruceCount = Math.round(this.count * this.spruceRatio);
        const birchCount = this.count - spruceCount;

        const spruceTrunks = new THREE.InstancedMesh(spruceTrunkGeometry, spruceTrunkMaterial, spruceCount);
        const spruceLayerA = new THREE.InstancedMesh(spruceLayerGeometry, spruceNeedleMaterial, spruceCount);
        const spruceLayerB = new THREE.InstancedMesh(spruceLayerGeometry, spruceNeedleMaterial, spruceCount);
        const spruceLayerC = new THREE.InstancedMesh(spruceLayerGeometry, spruceNeedleMaterial, spruceCount);

        const birchTrunks = new THREE.InstancedMesh(birchTrunkGeometry, birchTrunkMaterial, birchCount);
        const birchCanopies = new THREE.InstancedMesh(birchCanopyGeometry, birchCanopyMaterial, birchCount);

        [
            spruceTrunks,
            spruceLayerA,
            spruceLayerB,
            spruceLayerC,
            birchTrunks,
            birchCanopies
        ].forEach((mesh) => {
            mesh.instanceMatrix.setUsage(THREE.StaticDrawUsage);
            mesh.castShadow = this.enableShadows;
            mesh.receiveShadow = this.enableShadows;
            mesh.frustumCulled = true;
        });

        const matrix = new THREE.Matrix4();
        const scale = new THREE.Vector3();
        const position = new THREE.Vector3();
        const rotation = new THREE.Euler();
        const quaternion = new THREE.Quaternion();

        let sprucePlaced = 0;
        let birchPlaced = 0;
        let attempts = 0;
        const maxAttempts = this.count * 8;

        while ((sprucePlaced < spruceCount || birchPlaced < birchCount) && attempts < maxAttempts) {
            attempts += 1;
            const x = THREE.MathUtils.lerp(this.region.xMin, this.region.xMax, Math.random());
            const z = THREE.MathUtils.lerp(this.region.zMin, this.region.zMax, Math.random());
            const y = this.terrain.getHeightAt(x, z);

            if (y < -5) {
                continue;
            }

            const useSpruce = (Math.random() < this.spruceRatio && sprucePlaced < spruceCount)
                || birchPlaced >= birchCount;

            rotation.set(0, Math.random() * Math.PI * 2, 0);
            quaternion.setFromEuler(rotation);

            if (useSpruce) {
                const trunkHeight = THREE.MathUtils.lerp(10, 16.5, Math.random());
                const trunkRadius = THREE.MathUtils.lerp(0.45, 0.8, Math.random());
                const layerScale = THREE.MathUtils.lerp(0.9, 1.25, Math.random());

                scale.set(trunkRadius, trunkHeight / 10, trunkRadius);
                position.set(x, y + (trunkHeight / 2), z);
                matrix.compose(position, quaternion, scale);
                spruceTrunks.setMatrixAt(sprucePlaced, matrix);

                scale.set(layerScale, layerScale, layerScale);
                position.set(x, y + trunkHeight * 0.6, z);
                matrix.compose(position, quaternion, scale);
                spruceLayerA.setMatrixAt(sprucePlaced, matrix);

                scale.set(layerScale * 0.9, layerScale * 0.9, layerScale * 0.9);
                position.set(x, y + trunkHeight * 0.78, z);
                matrix.compose(position, quaternion, scale);
                spruceLayerB.setMatrixAt(sprucePlaced, matrix);

                scale.set(layerScale * 0.65, layerScale * 0.65, layerScale * 0.65);
                position.set(x, y + trunkHeight * 0.94, z);
                matrix.compose(position, quaternion, scale);
                spruceLayerC.setMatrixAt(sprucePlaced, matrix);

                sprucePlaced += 1;
            } else if (birchPlaced < birchCount) {
                const trunkHeight = THREE.MathUtils.lerp(4.5, 7, Math.random());
                const trunkRadius = THREE.MathUtils.lerp(0.18, 0.35, Math.random());
                const canopyScale = THREE.MathUtils.lerp(0.7, 1.1, Math.random());

                scale.set(trunkRadius, trunkHeight / 5.5, trunkRadius);
                position.set(x, y + (trunkHeight / 2), z);
                matrix.compose(position, quaternion, scale);
                birchTrunks.setMatrixAt(birchPlaced, matrix);

                scale.set(canopyScale, canopyScale * 0.85, canopyScale);
                position.set(x, y + trunkHeight + 0.6, z);
                matrix.compose(position, quaternion, scale);
                birchCanopies.setMatrixAt(birchPlaced, matrix);

                birchPlaced += 1;
            }
        }

        spruceTrunks.count = sprucePlaced;
        spruceLayerA.count = sprucePlaced;
        spruceLayerB.count = sprucePlaced;
        spruceLayerC.count = sprucePlaced;
        birchTrunks.count = birchPlaced;
        birchCanopies.count = birchPlaced;

        [
            spruceTrunks,
            spruceLayerA,
            spruceLayerB,
            spruceLayerC,
            birchTrunks,
            birchCanopies
        ].forEach((mesh) => {
            mesh.instanceMatrix.needsUpdate = true;
            this.scene.add(mesh);
        });
    }
}

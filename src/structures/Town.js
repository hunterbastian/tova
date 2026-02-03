import * as THREE from 'three';
import { mergeGeometries } from 'three/examples/jsm/utils/BufferGeometryUtils.js';

export class Town {
    constructor(scene, terrain, options = {}) {
        this.scene = scene;
        this.terrain = terrain;
        this.group = new THREE.Group();
        this.enableShadows = options.enableShadows ?? true;
        this.init();
    }

    init() {
        const materials = {
            timber: new THREE.MeshStandardMaterial({
                color: 0x5b3a22,
                roughness: 1.0,
                metalness: 0.0,
                flatShading: true
            }),
            plaster: new THREE.MeshStandardMaterial({
                color: 0xe2d4ba,
                roughness: 0.95,
                metalness: 0.0,
                flatShading: true
            }),
            stone: new THREE.MeshStandardMaterial({
                color: 0x8f8f8f,
                roughness: 1.0,
                metalness: 0.0,
                flatShading: true
            }),
            wood: new THREE.MeshStandardMaterial({
                color: 0x8b5a2b,
                roughness: 1.0,
                metalness: 0.0,
                flatShading: true
            }),
            roofRed: new THREE.MeshStandardMaterial({
                color: 0x7a1f1f,
                roughness: 1.0,
                metalness: 0.0,
                flatShading: true
            }),
            roofSlate: new THREE.MeshStandardMaterial({
                color: 0x39424e,
                roughness: 0.9,
                metalness: 0.0,
                flatShading: true
            }),
            road: new THREE.MeshStandardMaterial({
                color: 0x4b3f34,
                roughness: 1.0,
                metalness: 0.0,
                flatShading: true
            }),
            cobble: new THREE.MeshStandardMaterial({
                color: 0x6f6f6f,
                roughness: 1.0,
                metalness: 0.0,
                flatShading: true
            })
        };

        const geometries = {
            baseSmall: new THREE.BoxGeometry(4, 3.5, 4),
            baseWide: new THREE.BoxGeometry(6, 4.2, 5),
            roofPyramid: new THREE.ConeGeometry(3.6, 2.6, 4),
            roofLow: new THREE.ConeGeometry(4.1, 1.8, 4),
            chimney: new THREE.BoxGeometry(0.8, 2.2, 0.8),
            door: new THREE.BoxGeometry(0.6, 1.4, 0.2),
            window: new THREE.BoxGeometry(0.5, 0.5, 0.15),
            beam: new THREE.BoxGeometry(0.3, 3.8, 0.2),
            roadEW: new THREE.BoxGeometry(80, 0.25, 6),
            roadNS: new THREE.BoxGeometry(6, 0.25, 80),
            plaza: new THREE.CylinderGeometry(11, 11, 0.4, 20),
            wellBase: new THREE.CylinderGeometry(2.2, 2.4, 2, 12),
            wellRoof: new THREE.ConeGeometry(2.6, 1.6, 6),
            wellPost: new THREE.BoxGeometry(0.25, 2.4, 0.25),
            stallBase: new THREE.BoxGeometry(3.6, 1.2, 2.2),
            stallRoof: new THREE.ConeGeometry(2.4, 1.2, 4),
            tavernSign: new THREE.BoxGeometry(1.2, 1.2, 0.15)
        };

        const geoBuckets = new Map();
        const tempPos = new THREE.Vector3();
        const tempScale = new THREE.Vector3(1, 1, 1);
        const tempQuat = new THREE.Quaternion();
        const tempEuler = new THREE.Euler();
        const tempMatrix = new THREE.Matrix4();

        const addGeometry = (geometry, materialKey, position, rotationY = 0, scale = tempScale) => {
            const cloned = geometry.clone();
            tempEuler.set(0, rotationY, 0);
            tempQuat.setFromEuler(tempEuler);
            tempMatrix.compose(position, tempQuat, scale);
            cloned.applyMatrix4(tempMatrix);
            const bucket = geoBuckets.get(materialKey) ?? [];
            bucket.push(cloned);
            geoBuckets.set(materialKey, bucket);
        };

        const addShadowFlags = (mesh) => {
            mesh.castShadow = false;
            mesh.receiveShadow = this.enableShadows;
            mesh.matrixAutoUpdate = false;
            mesh.updateMatrix();
        };

        const addAtTerrain = (geometry, materialKey, x, z, yOffset = 0, rotation = 0) => {
            const y = this.terrain.getHeightAt(x, z);
            tempPos.set(x, y + yOffset, z);
            addGeometry(geometry, materialKey, tempPos, rotation);
        };

        const createHouse = (x, z, style = 'timber') => {
            const rotation = Math.random() * Math.PI * 2;
            const y = this.terrain.getHeightAt(x, z);

            if (style === 'stone') {
                tempPos.set(x, y + 2.2, z);
                addGeometry(geometries.baseWide, 'stone', tempPos, rotation);
                tempPos.set(x, y + 5.5, z);
                addGeometry(geometries.roofLow, 'roofSlate', tempPos, rotation + Math.PI / 4);
            } else {
                tempPos.set(x, y + 2.2, z);
                addGeometry(geometries.baseWide, 'plaster', tempPos, rotation);
                tempPos.set(x, y + 5.5, z);
                addGeometry(geometries.roofPyramid, 'roofRed', tempPos, rotation + Math.PI / 4);
            }

            tempPos.set(x + 1.2, y + 6.3, z + 0.8);
            addGeometry(geometries.chimney, 'stone', tempPos, rotation * 0.5);

            if (style === 'timber') {
                tempPos.set(x + 1.6, y + 2.6, z + 1.9);
                addGeometry(geometries.beam, 'timber', tempPos, rotation);
                tempPos.set(x - 1.6, y + 2.6, z + 1.9);
                addGeometry(geometries.beam, 'timber', tempPos, rotation);
            }

            tempPos.set(x, y + 1.2, z + 2.05);
            addGeometry(geometries.door, 'timber', tempPos, rotation);

            tempPos.set(x - 1.1, y + 2.5, z + 2.05);
            addGeometry(geometries.window, 'roofSlate', tempPos, rotation);
            tempPos.set(x + 1.1, y + 2.5, z + 2.05);
            addGeometry(geometries.window, 'roofSlate', tempPos, rotation);
        };

        const townCenter = { x: 60, z: 0 };

        addAtTerrain(geometries.roadEW, 'road', townCenter.x, townCenter.z, 0.05, 0);
        addAtTerrain(geometries.roadNS, 'road', townCenter.x - 10, townCenter.z, 0.05, 0);
        addAtTerrain(geometries.plaza, 'cobble', townCenter.x, townCenter.z, 0.1, 0);

        addAtTerrain(geometries.wellBase, 'stone', townCenter.x, townCenter.z, 1, 0);
        addAtTerrain(geometries.wellRoof, 'roofRed', townCenter.x, townCenter.z, 2.8, Math.PI / 6);
        addAtTerrain(geometries.wellPost, 'timber', townCenter.x - 1.2, townCenter.z + 0.9, 2.2, 0);
        addAtTerrain(geometries.wellPost, 'timber', townCenter.x + 1.2, townCenter.z - 0.9, 2.2, 0);

        for (let i = 0; i < 3; i++) {
            const angle = (i / 3) * Math.PI * 2 + Math.PI / 6;
            const stallX = townCenter.x + Math.cos(angle) * 13;
            const stallZ = townCenter.z + Math.sin(angle) * 13;
            addAtTerrain(geometries.stallBase, 'wood', stallX, stallZ, 1, angle);
            addAtTerrain(geometries.stallRoof, 'roofRed', stallX, stallZ, 2.2, angle + Math.PI / 4);
        }

        const tavernPos = { x: townCenter.x + 16, z: townCenter.z - 10 };
        createHouse(tavernPos.x, tavernPos.z, 'stone');
        addAtTerrain(geometries.tavernSign, 'roofRed', tavernPos.x + 2.6, tavernPos.z + 1.2, 3.2, Math.PI / 8);

        const houseRows = [];
        for (let i = -3; i <= 3; i++) {
            houseRows.push({ x: townCenter.x + i * 9 + (Math.random() * 1.5 - 0.75), z: townCenter.z - 16 + (Math.random() * 1.5 - 0.75) });
            houseRows.push({ x: townCenter.x + i * 9 + (Math.random() * 1.5 - 0.75), z: townCenter.z + 16 + (Math.random() * 1.5 - 0.75) });
        }

        for (let i = -2; i <= 2; i++) {
            houseRows.push({ x: townCenter.x - 20 + (Math.random() * 1.5 - 0.75), z: townCenter.z + i * 11 + (Math.random() * 1.5 - 0.75) });
            houseRows.push({ x: townCenter.x + 26 + (Math.random() * 1.5 - 0.75), z: townCenter.z + i * 11 + (Math.random() * 1.5 - 0.75) });
        }

        const styles = ['timber', 'stone'];
        houseRows.forEach((spot, idx) => {
            const style = styles[idx % styles.length];
            createHouse(spot.x, spot.z, style);
        });

        for (const [materialKey, bucket] of geoBuckets.entries()) {
            const merged = mergeGeometries(bucket, false);
            if (!merged) continue;
            const mesh = new THREE.Mesh(merged, materials[materialKey]);
            addShadowFlags(mesh);
            this.group.add(mesh);
        }

        this.group.matrixAutoUpdate = false;
        this.group.updateMatrix();

        this.scene.add(this.group);
    }
}

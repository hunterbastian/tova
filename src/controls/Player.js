import * as THREE from 'three';
import { PointerLockControls } from 'three/examples/jsm/controls/PointerLockControls.js';

export class Player {
    constructor(scene, camera, terrain) {
        this.scene = scene;
        this.camera = camera;
        this.terrain = terrain;

        this.controls = new PointerLockControls(camera, document.body);
        this.playerObject = this.controls.getObject();

        this.moveForward = false;
        this.moveBackward = false;
        this.moveLeft = false;
        this.moveRight = false;

        this.walkSpeed = 30;
        this.eyeHeight = 3;
        this.groundSnapSpeed = 10; // higher = quicker snapping to terrain

        this.init();
        this.alignToTerrain(0);
    }

    init() {
        const instructions = document.createElement('div');
        instructions.style.position = 'absolute';
        instructions.style.top = '50%';
        instructions.style.left = '50%';
        instructions.style.transform = 'translate(-50%, -50%)';
        instructions.style.textAlign = 'center';
        instructions.style.color = '#ffffff';
        instructions.style.fontFamily = 'serif';
        instructions.style.fontSize = '24px';
        instructions.style.cursor = 'pointer';
        instructions.innerHTML = '<div style="font-size:32px; letter-spacing:4px;">Tova</div>Click to Walk<br/>WASD to move';
        document.body.appendChild(instructions);

        instructions.addEventListener('click', () => {
            this.controls.lock();
        });

        this.controls.addEventListener('lock', () => {
            instructions.style.display = 'none';
        });

        this.controls.addEventListener('unlock', () => {
            instructions.style.display = 'block';
        });

        const onKeyDown = (event) => {
            switch (event.code) {
                case 'ArrowUp':
                case 'KeyW':
                    this.moveForward = true;
                    break;
                case 'ArrowLeft':
                case 'KeyA':
                    this.moveLeft = true;
                    break;
                case 'ArrowDown':
                case 'KeyS':
                    this.moveBackward = true;
                    break;
                case 'ArrowRight':
                case 'KeyD':
                    this.moveRight = true;
                    break;
            }
        };

        const onKeyUp = (event) => {
            switch (event.code) {
                case 'ArrowUp':
                case 'KeyW':
                    this.moveForward = false;
                    break;
                case 'ArrowLeft':
                case 'KeyA':
                    this.moveLeft = false;
                    break;
                case 'ArrowDown':
                case 'KeyS':
                    this.moveBackward = false;
                    break;
                case 'ArrowRight':
                case 'KeyD':
                    this.moveRight = false;
                    break;
            }
        };

        document.addEventListener('keydown', onKeyDown);
        document.addEventListener('keyup', onKeyUp);
    }

    update(delta) {
        if (this.controls.isLocked === true) {
            const forward = Number(this.moveForward) - Number(this.moveBackward);
            const strafe = Number(this.moveRight) - Number(this.moveLeft);

            if (forward !== 0) {
                this.controls.moveForward(forward * this.walkSpeed * delta);
            }

            if (strafe !== 0) {
                this.controls.moveRight(strafe * this.walkSpeed * delta);
            }
        }

        this.alignToTerrain(delta);
    }

    alignToTerrain(delta) {
        const position = this.playerObject.position;
        const groundHeight = this.terrain.getHeightAt(position.x, position.z) + this.eyeHeight;

        if (!delta) {
            position.y = groundHeight;
            return;
        }

        const snapFactor = Math.min(1, this.groundSnapSpeed * delta);
        position.y += (groundHeight - position.y) * snapFactor;
    }
}

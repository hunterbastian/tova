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

        // Flying controls (enabled via chat command)
        this.moveUp = false;
        this.moveDown = false;
        this.isFlying = false;

        this.walkSpeed = 30;
        this.flySpeed = 40;
        this.eyeHeight = 3;
        this.groundSnapSpeed = 10; // higher = quicker snapping to terrain

        // Simple chat UI for commands like `fly` / `walk`
        this.chatOpen = false;

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
        instructions.innerHTML = '<div style="font-size:32px; letter-spacing:4px;">Tova</div>Click to Walk<br/>WASD to move<br/>Press / to open chat';
        document.body.appendChild(instructions);

        // Simple chat box for text commands (e.g., "fly", "walk")
        this.chatContainer = document.createElement('div');
        this.chatContainer.style.position = 'absolute';
        this.chatContainer.style.left = '50%';
        this.chatContainer.style.bottom = '20px';
        this.chatContainer.style.transform = 'translateX(-50%)';
        this.chatContainer.style.background = 'rgba(0, 0, 0, 0.6)';
        this.chatContainer.style.padding = '6px 10px';
        this.chatContainer.style.borderRadius = '4px';
        this.chatContainer.style.fontFamily = 'monospace';
        this.chatContainer.style.color = '#ffffff';
        this.chatContainer.style.display = 'none';

        const chatInput = document.createElement('input');
        chatInput.type = 'text';
        chatInput.style.background = 'transparent';
        chatInput.style.border = 'none';
        chatInput.style.outline = 'none';
        chatInput.style.color = '#ffffff';
        chatInput.style.minWidth = '220px';
        chatInput.placeholder = 'type command (fly / walk)';

        this.chatContainer.appendChild(chatInput);
        document.body.appendChild(this.chatContainer);
        this.chatInput = chatInput;

        instructions.addEventListener('click', () => {
            this.controls.lock();
        });

        this.controls.addEventListener('lock', () => {
            instructions.style.display = 'none';
        });

        this.controls.addEventListener('unlock', () => {
            instructions.style.display = 'block';
        });

        const openChat = () => {
            this.chatOpen = true;
            this.chatContainer.style.display = 'block';
            this.chatInput.value = '';
            this.moveForward = false;
            this.moveBackward = false;
            this.moveLeft = false;
            this.moveRight = false;
            this.moveUp = false;
            this.moveDown = false;
            this.chatInput.focus();
        };

        const closeChat = () => {
            this.chatOpen = false;
            this.chatContainer.style.display = 'none';
        };

        const handleChatCommand = (raw) => {
            const cmd = raw.trim().toLowerCase();
            if (!cmd) return;

            if (cmd === 'fly') {
                this.isFlying = true;
            } else if (cmd === 'walk') {
                this.isFlying = false;
                // Snap back to terrain when returning to walk mode.
                this.alignToTerrain(0);
            }
        };

        this.chatInput.addEventListener('keydown', (event) => {
            if (event.key === 'Enter') {
                handleChatCommand(this.chatInput.value);
                closeChat();
            } else if (event.key === 'Escape') {
                closeChat();
            }
        });

        const onKeyDown = (event) => {
            // If chat is open, only handle Escape here; everything else goes to the input.
            if (this.chatOpen) {
                if (event.key === 'Escape') {
                    closeChat();
                }
                return;
            }

            // Open chat with '/'
            if (event.key === '/') {
                event.preventDefault();
                openChat();
                return;
            }

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
                case 'Space':
                    this.moveUp = true;
                    break;
                case 'ShiftLeft':
                case 'ShiftRight':
                    this.moveDown = true;
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
                case 'Space':
                    this.moveUp = false;
                    break;
                case 'ShiftLeft':
                case 'ShiftRight':
                    this.moveDown = false;
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

            const speed = this.isFlying ? this.flySpeed : this.walkSpeed;

            if (forward !== 0) {
                this.controls.moveForward(forward * speed * delta);
            }

            if (strafe !== 0) {
                this.controls.moveRight(strafe * speed * delta);
            }

            if (this.isFlying) {
                const vertical = Number(this.moveUp) - Number(this.moveDown);
                if (vertical !== 0) {
                    this.playerObject.position.y += vertical * this.flySpeed * delta;
                }
            }
        }

        if (!this.isFlying) {
            this.alignToTerrain(delta);
        }
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

import * as THREE from "three";

import {
  BOB_ROLL_AMP,
  BOB_SPRINT_FREQ,
  BOB_VERTICAL_AMP,
  BOB_WALK_FREQ,
  GRAVITY,
  HOTBAR,
  JUMP_SPEED,
  LAND_DIP_MAX,
  LAND_DIP_RECOVERY,
  LAND_DIP_SCALE,
  MOVE_ACCEL,
  MOVE_DECEL,
  PLAYER_HEIGHT,
  SPRINT_SPEED,
  WALK_SPEED,
} from "./constants.js";

export function createPlayerSystem({
  app,
  camera,
  collisionSystem,
  controls,
  domElement,
  isAutomationSession,
  onFootstep,
  onHudChange,
  onInteract,
  onLand,
  onPrimaryAttack,
  onRegenerate,
  onStatus,
  sampleGroundHeight,
  state,
  sunLight,
}) {
  const LOOK_SENSITIVITY = 0.0022;
  const MAX_PITCH = Math.PI / 2 - 0.04;
  const worldUp = new THREE.Vector3(0, 1, 0);
  const forward = new THREE.Vector3();
  const right = new THREE.Vector3();
  const displacement = new THREE.Vector3();
  let lockAttemptTimer = null;

  let currentSpeed = 0;
  let bobPhase = 0;
  let bobBlend = 0;
  let wasGrounded = true;
  let landDipOffset = 0;
  let lastStepIndex = 0;

  camera.rotation.order = "YXZ";

  function canControl() {
    return controls.isLocked || state.walkMode;
  }

  function markReady() {
    if (state.mode === "ready") {
      return;
    }

    state.mode = "ready";
  }

  function clearLockAttemptTimer() {
    if (lockAttemptTimer !== null) {
      window.clearTimeout(lockAttemptTimer);
      lockAttemptTimer = null;
    }
  }

  function enableWalkMode(message = "Walk mode engaged") {
    clearLockAttemptTimer();
    if (controls.isLocked || state.walkMode) {
      return;
    }

    markReady();
    state.walkMode = true;
    onStatus(message);
    onHudChange();
  }

  function applyFallbackLook(deltaX, deltaY) {
    if (controls.isLocked || !state.walkMode) {
      return;
    }

    camera.rotation.y -= deltaX * LOOK_SENSITIVITY;
    camera.rotation.x = THREE.MathUtils.clamp(
      camera.rotation.x - deltaY * LOOK_SENSITIVITY,
      -MAX_PITCH,
      MAX_PITCH,
    );
  }

  function engageFrontier() {
    if (canControl()) {
      return;
    }

    if (isAutomationSession) {
      enableWalkMode("Walk mode engaged");
      return;
    }

    clearLockAttemptTimer();
    markReady();
    state.walkMode = true;
    onStatus("Entering frontier");
    onHudChange();
    controls.lock();
    lockAttemptTimer = window.setTimeout(() => {
      if (!controls.isLocked) {
        enableWalkMode("Pointer lock unavailable. Walk mode engaged");
      }
    }, 180);
  }

  function respawnAtSpawn() {
    const y = sampleGroundHeight(0, 0) + PLAYER_HEIGHT;
    controls.object.position.set(0, y, 0);
    state.velocity.set(0, 0, 0);
    state.walkMode = false;
    currentSpeed = 0;
    bobPhase = 0;
    bobBlend = 0;
    landDipOffset = 0;
    lastStepIndex = 0;
    wasGrounded = true;
    camera.rotation.z = 0;
    const target = state.castleCenter.clone();
    target.y = y - 0.5;
    camera.lookAt(target);
  }

  function update(dt) {
    if (!canControl()) {
      camera.rotation.z = 0;
      return;
    }

    const moveForward = Number(state.pressed.has("KeyW")) - Number(state.pressed.has("KeyS"));
    const moveRight = Number(state.pressed.has("KeyD")) - Number(state.pressed.has("KeyA"));
    const isSprinting = state.pressed.has("ShiftLeft") || state.pressed.has("ShiftRight");
    const speed = isSprinting ? SPRINT_SPEED : WALK_SPEED;
    const hasInput = moveForward !== 0 || moveRight !== 0;

    /* ── acceleration / deceleration ─────────────────────── */
    const targetSpeed = hasInput ? speed : 0;
    const accelRate = currentSpeed < targetSpeed ? MOVE_ACCEL : MOVE_DECEL;
    currentSpeed += (targetSpeed - currentSpeed) * (1 - Math.exp(-accelRate * dt));
    if (currentSpeed < 0.08) currentSpeed = 0;

    /* ── horizontal movement ─────────────────────────────── */
    state.moveVector.set(0, 0, 0);
    if (hasInput) {
      state.moveVector.z = moveForward;
      state.moveVector.x = moveRight;
      state.moveVector.normalize();
    }

    controls.getDirection(forward);
    forward.y = 0;
    forward.normalize();
    right.crossVectors(forward, worldUp).normalize();

    displacement.set(0, 0, 0);
    displacement.addScaledVector(forward, state.moveVector.z * currentSpeed * dt);
    displacement.addScaledVector(right, state.moveVector.x * currentSpeed * dt);
    controls.object.position.add(displacement);
    collisionSystem.resolve(controls.object.position);

    /* ── gravity + ground check ──────────────────────────── */
    state.velocity.y -= GRAVITY * dt;
    const preGroundVy = state.velocity.y;
    controls.object.position.y += state.velocity.y * dt;

    const groundHeight = sampleGroundHeight(controls.object.position.x, controls.object.position.z) + PLAYER_HEIGHT;
    if (controls.object.position.y <= groundHeight) {
      controls.object.position.y = groundHeight;
      state.velocity.y = 0;
      state.grounded = true;
    } else {
      state.grounded = false;
    }

    /* ── landing camera dip ──────────────────────────────── */
    if (state.grounded && !wasGrounded) {
      const fallSpeed = Math.abs(preGroundVy);
      landDipOffset = Math.min(fallSpeed * LAND_DIP_SCALE, LAND_DIP_MAX);
      if (fallSpeed > 3) onLand?.(fallSpeed / 12);
    }
    wasGrounded = state.grounded;
    landDipOffset *= Math.exp(-LAND_DIP_RECOVERY * dt);

    /* ── head bob ────────────────────────────────────────── */
    const movingOnGround = currentSpeed > 0.5 && state.grounded;
    bobBlend += ((movingOnGround ? 1 : 0) - bobBlend) * (1 - Math.exp(-12 * dt));

    if (movingOnGround) {
      const freq = isSprinting ? BOB_SPRINT_FREQ : BOB_WALK_FREQ;
      bobPhase += freq * Math.PI * 2 * dt;

      /* ── footstep audio sync ─────────────────────────────── */
      const stepIndex = Math.floor(bobPhase / Math.PI);
      if (stepIndex !== lastStepIndex) {
        lastStepIndex = stepIndex;
        onFootstep?.(isSprinting);
      }
    }

    const verticalBob = Math.sin(bobPhase) * BOB_VERTICAL_AMP * bobBlend;
    const rollBob = Math.cos(bobPhase * 0.5) * BOB_ROLL_AMP * bobBlend;

    /* ── apply camera offsets ────────────────────────────── */
    controls.object.position.y += verticalBob - landDipOffset;
    camera.rotation.z = rollBob;

    /* ── fatigue / magicka ───────────────────────────────── */
    const effort = currentSpeed > 0.5 ? (isSprinting ? 0.5 : 0.28) : -0.22;
    state.fatigue = THREE.MathUtils.clamp(state.fatigue - effort * dt, 0.22, 1);
    state.magicka = THREE.MathUtils.clamp(state.magicka + 0.05 * dt, 0.18, 0.88);

    sunLight.target.position.set(
      controls.object.position.x,
      0,
      controls.object.position.z,
    );
    sunLight.target.updateMatrixWorld();
  }

  function normalizeKeyCode(code) {
    switch (code) {
      case "ArrowUp":
        return "KeyW";
      case "ArrowDown":
        return "KeyS";
      case "ArrowLeft":
        return "KeyA";
      case "ArrowRight":
        return "KeyD";
      default:
        return code;
    }
  }

  app.addEventListener("pointerdown", (event) => {
    if (event.button !== 0) {
      return;
    }

    event.preventDefault();

    if (canControl()) {
      onPrimaryAttack();
      return;
    }

    engageFrontier();
  });

  controls.addEventListener("lock", () => {
    clearLockAttemptTimer();
    markReady();
    state.walkMode = false;
    onStatus("Frontier entered");
    onHudChange();
  });

  controls.addEventListener("unlock", () => {
    clearLockAttemptTimer();
    state.walkMode = false;
    onStatus("Cursor released");
    onHudChange();
  });

  document.addEventListener("pointerlockerror", () => {
    enableWalkMode("Pointer lock unavailable. Walk mode engaged");
  });

  window.addEventListener("keydown", (event) => {
    const code = normalizeKeyCode(event.code);
    const isManualWalkToggle = code === "Enter";
    const isInteractKey = code === "KeyE" || (isAutomationSession && code === "KeyB");

    if (code === "Escape") {
      state.walkMode = false;
      if (controls.isLocked) {
        controls.unlock();
      } else {
        onStatus("Cursor released");
        onHudChange();
      }
      return;
    }

    if (isManualWalkToggle && !controls.isLocked) {
      enableWalkMode("Walk mode engaged");
      return;
    }

    if (code === "KeyR") {
      onRegenerate();
      return;
    }

    if (isInteractKey) {
      if (canControl()) {
        onInteract();
      }
      return;
    }

    if (code === "Space" && state.grounded && canControl()) {
      state.velocity.y = JUMP_SPEED;
      state.grounded = false;
      event.preventDefault();
    }

    if (/^Digit[1-5]$/.test(code)) {
      state.selectedSlot = Number(code.slice(-1)) - 1;
      onStatus(`Selected ${HOTBAR[state.selectedSlot].label}`);
      onHudChange();
      return;
    }

    if (code === "KeyF") {
      if (document.fullscreenElement) {
        document.exitFullscreen().catch(() => {});
      } else {
        app.requestFullscreen?.().catch(() => {});
      }
      return;
    }

    state.pressed.add(code);
  });

  window.addEventListener("keyup", (event) => {
    state.pressed.delete(normalizeKeyCode(event.code));
  });

  window.addEventListener("mousemove", (event) => {
    applyFallbackLook(event.movementX, event.movementY);
  });

  window.addEventListener("blur", () => {
    clearLockAttemptTimer();
    state.pressed.clear();
  });

  return { canControl, engageFrontier, respawnAtSpawn, update };
}

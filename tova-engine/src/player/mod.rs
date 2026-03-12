use std::collections::HashSet;
use glam::Vec3;
use winit::keyboard::KeyCode;

use crate::voxel::chunk::CHUNK_SIZE;
use crate::world::ChunkManager;

// ─── Physics constants ──────────────────────────────────────
const GRAVITY: f32 = -25.0;
const JUMP_VELOCITY: f32 = 8.5;
const WALK_SPEED: f32 = 4.3;
const SPRINT_SPEED: f32 = 7.0;
const CROUCH_SPEED: f32 = 2.0;
const PLAYER_HEIGHT: f32 = 1.8;
const CROUCH_HEIGHT: f32 = 1.2;
const EYE_HEIGHT: f32 = 1.62;
const CROUCH_EYE_HEIGHT: f32 = 1.0;
const HALF_WIDTH: f32 = 0.28;
const TERMINAL_VELOCITY: f32 = -50.0;
const GOD_SPEED: f32 = 20.0;
const GOD_SPRINT_SPEED: f32 = 50.0;
const SENSITIVITY: f32 = 0.003;
const STEP_RATE: f32 = 2.8;
const SPRINT_STEP_RATE: f32 = 4.0;
const HEAD_BOB_Y: f32 = 0.04;
const HEAD_BOB_X: f32 = 0.02;
const SPRINT_HEAD_BOB_Y: f32 = 0.04;
const SPRINT_HEAD_BOB_X: f32 = 0.02;
const EYE_HEIGHT_LERP_SPEED: f32 = 10.0;
const SHAKE_DECAY: f32 = 8.0;
const STAMINA_DRAIN: f32 = 0.15;  // per second while sprinting
const STAMINA_REGEN: f32 = 0.25;  // per second while not sprinting
const STAMINA_SPRINT_MIN: f32 = 0.05; // can't sprint below this

/// Tracks which keys are currently held down.
pub struct Input {
    pressed: HashSet<KeyCode>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            pressed: HashSet::new(),
        }
    }

    pub fn key_down(&mut self, key: KeyCode) {
        self.pressed.insert(key);
    }

    pub fn key_up(&mut self, key: KeyCode) {
        self.pressed.remove(&key);
    }

    pub fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    pub fn forward(&self) -> bool {
        self.is_pressed(KeyCode::KeyW) || self.is_pressed(KeyCode::ArrowUp)
    }

    pub fn back(&self) -> bool {
        self.is_pressed(KeyCode::KeyS) || self.is_pressed(KeyCode::ArrowDown)
    }

    pub fn left(&self) -> bool {
        self.is_pressed(KeyCode::KeyA) || self.is_pressed(KeyCode::ArrowLeft)
    }

    pub fn right(&self) -> bool {
        self.is_pressed(KeyCode::KeyD) || self.is_pressed(KeyCode::ArrowRight)
    }

    pub fn jump(&self) -> bool {
        self.is_pressed(KeyCode::Space)
    }

    pub fn sprint(&self) -> bool {
        self.is_pressed(KeyCode::ShiftLeft) || self.is_pressed(KeyCode::ShiftRight)
    }

    pub fn crouch(&self) -> bool {
        self.is_pressed(KeyCode::ControlLeft) || self.is_pressed(KeyCode::ControlRight)
            || self.is_pressed(KeyCode::KeyC)
    }
}

/// First-person player with gravity and voxel collision.
pub struct Player {
    /// Feet position in world space.
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub sprinting: bool,
    pub crouching: bool,
    pub god_mode: bool,
    pub stamina: f32,
    pub sensitivity: f32,
    pub walk_speed_override: Option<f32>,
    walk_cycle: f32,
    step_count: u32,
    step_pending: bool,
    current_eye_height: f32,
    landing_shake: f32,
}

impl Player {
    pub fn new(spawn: Vec3) -> Self {
        Self {
            position: spawn,
            velocity: Vec3::ZERO,
            on_ground: false,
            yaw: 0.0,
            pitch: 0.0,
            sprinting: false,
            crouching: false,
            god_mode: false,
            stamina: 1.0,
            sensitivity: SENSITIVITY,
            walk_speed_override: None,
            walk_cycle: 0.0,
            step_count: 0,
            step_pending: false,
            current_eye_height: EYE_HEIGHT,
            landing_shake: 0.0,
        }
    }

    /// Eye position (camera attaches here), with smooth crouch transition.
    pub fn eye_position(&self) -> Vec3 {
        self.position + Vec3::new(0.0, self.current_eye_height, 0.0)
    }

    /// Head bob offset in world space (Y = vertical bob, local-right = sway).
    pub fn head_bob_offset(&self) -> Vec3 {
        if self.walk_cycle == 0.0 || !self.on_ground {
            return Vec3::ZERO;
        }
        let phase = self.walk_cycle * std::f32::consts::TAU;
        let (bob_y, bob_x) = if self.sprinting {
            (SPRINT_HEAD_BOB_Y, SPRINT_HEAD_BOB_X)
        } else {
            (HEAD_BOB_Y, HEAD_BOB_X)
        };
        let y = phase.sin() * bob_y;
        // Sway uses cos so it's offset 90° from the vertical bob
        let right = Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin());
        let x_sway = phase.cos() * bob_x;
        Vec3::new(0.0, y, 0.0) + right * x_sway
    }

    /// Landing shake intensity (decays over time).
    pub fn landing_shake(&self) -> f32 {
        self.landing_shake
    }

    /// Which chunk the player is standing in.
    pub fn chunk_pos(&self) -> (i32, i32) {
        let cx = (self.position.x as i32).div_euclid(CHUNK_SIZE as i32);
        let cz = (self.position.z as i32).div_euclid(CHUNK_SIZE as i32);
        (cx, cz)
    }

    /// Horizontal forward direction (no pitch).
    fn forward_flat(&self) -> Vec3 {
        Vec3::new(self.yaw.sin(), 0.0, -self.yaw.cos()).normalize()
    }

    /// Horizontal right direction.
    fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.cos(), 0.0, self.yaw.sin())
    }

    /// Walk cycle phase (0..1), used for foot animation.
    pub fn walk_cycle(&self) -> f32 {
        self.walk_cycle
    }

    /// Returns true once per footstep, then clears the flag.
    pub fn take_step(&mut self) -> Option<u32> {
        if self.step_pending {
            self.step_pending = false;
            Some(self.step_count)
        } else {
            None
        }
    }

    /// Apply mouse delta.
    pub fn rotate(&mut self, dx: f64, dy: f64) {
        self.yaw += dx as f32 * self.sensitivity;
        self.pitch -= dy as f32 * self.sensitivity;
        let limit = 89.0_f32.to_radians();
        self.pitch = self.pitch.clamp(-limit, limit);
    }

    /// Run one physics tick.
    pub fn update(&mut self, dt: f32, input: &Input, cm: &ChunkManager) {
        let dt = dt.min(0.05);
        if self.god_mode {
            self.update_fly(dt, input);
        } else {
            self.update_walk(dt, input, cm);
        }
    }

    /// God mode — free flight, no gravity, no collision. Minecraft creative-style.
    fn update_fly(&mut self, dt: f32, input: &Input) {
        self.sprinting = input.sprint();
        self.crouching = false;
        self.on_ground = false;

        let speed = if self.sprinting { GOD_SPRINT_SPEED } else { GOD_SPEED };

        // Full 3D forward direction (follows pitch for flying up/down)
        let forward = Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        ).normalize();
        let right = self.right();

        let mut move_dir = Vec3::ZERO;
        if input.forward() { move_dir += forward; }
        if input.back() { move_dir -= forward; }
        if input.right() { move_dir += right; }
        if input.left() { move_dir -= right; }
        if input.jump() { move_dir += Vec3::Y; }
        if input.crouch() { move_dir -= Vec3::Y; }

        if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalize();
        }

        self.position += move_dir * speed * dt;
        self.velocity = Vec3::ZERO;
        self.walk_cycle = 0.0;

        // Smooth eye height back to normal
        self.current_eye_height += (EYE_HEIGHT - self.current_eye_height)
            * (EYE_HEIGHT_LERP_SPEED * dt).min(1.0);
    }

    /// Walk mode — gravity, collision, footsteps, sprint, crouch.
    fn update_walk(&mut self, dt: f32, input: &Input, cm: &ChunkManager) {
        // Sprint / crouch state (can't sprint while crouching or out of stamina)
        self.crouching = input.crouch();
        self.sprinting = input.sprint() && !self.crouching && self.stamina > STAMINA_SPRINT_MIN;

        // Stamina system
        if self.sprinting {
            self.stamina = (self.stamina - STAMINA_DRAIN * dt).max(0.0);
        } else {
            self.stamina = (self.stamina + STAMINA_REGEN * dt).min(1.0);
        }

        let base_walk = self.walk_speed_override.unwrap_or(WALK_SPEED);
        let move_speed = if self.crouching {
            CROUCH_SPEED
        } else if self.sprinting {
            SPRINT_SPEED
        } else {
            base_walk
        };

        let mut move_dir = Vec3::ZERO;
        if input.forward() { move_dir += self.forward_flat(); }
        if input.back() { move_dir -= self.forward_flat(); }
        if input.right() { move_dir += self.right(); }
        if input.left() { move_dir -= self.right(); }

        if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalize();
        }

        self.velocity.x = move_dir.x * move_speed;
        self.velocity.z = move_dir.z * move_speed;

        if input.jump() && self.on_ground && !self.crouching {
            self.velocity.y = JUMP_VELOCITY;
            self.on_ground = false;
        }

        self.velocity.y += GRAVITY * dt;
        self.velocity.y = self.velocity.y.max(TERMINAL_VELOCITY);

        let was_on_ground = self.on_ground;
        let pre_land_vel = self.velocity.y;

        // Move with collision (axis by axis, snap to wall face on hit)
        const EPSILON: f32 = 0.001;

        let new_x = self.position.x + self.velocity.x * dt;
        if !self.collides_at_height(Vec3::new(new_x, self.position.y, self.position.z), cm) {
            self.position.x = new_x;
        } else {
            // Snap to block face with epsilon gap to prevent corner catching
            if self.velocity.x > 0.0 {
                self.position.x = (new_x + HALF_WIDTH).floor() - HALF_WIDTH - EPSILON;
            } else if self.velocity.x < 0.0 {
                self.position.x = (new_x - HALF_WIDTH).floor() + 1.0 + HALF_WIDTH + EPSILON;
            }
            self.velocity.x = 0.0;
        }

        let new_z = self.position.z + self.velocity.z * dt;
        if !self.collides_at_height(Vec3::new(self.position.x, self.position.y, new_z), cm) {
            self.position.z = new_z;
        } else {
            if self.velocity.z > 0.0 {
                self.position.z = (new_z + HALF_WIDTH).floor() - HALF_WIDTH - EPSILON;
            } else if self.velocity.z < 0.0 {
                self.position.z = (new_z - HALF_WIDTH).floor() + 1.0 + HALF_WIDTH + EPSILON;
            }
            self.velocity.z = 0.0;
        }

        let new_y = self.position.y + self.velocity.y * dt;
        if !self.collides_at_height(Vec3::new(self.position.x, new_y, self.position.z), cm) {
            self.position.y = new_y;
            self.on_ground = false;
        } else {
            if self.velocity.y < 0.0 {
                self.position.y = (self.position.y as i32) as f32;
                self.snap_to_ground(cm);
                self.on_ground = true;
            }
            self.velocity.y = 0.0;
        }

        // Landing shake — trigger on ground impact
        if self.on_ground && !was_on_ground && pre_land_vel < -5.0 {
            self.landing_shake = (pre_land_vel.abs() / TERMINAL_VELOCITY.abs()).min(1.0) * 0.15;
        }
        if self.landing_shake > 0.001 {
            self.landing_shake *= (-SHAKE_DECAY * dt).exp();
        } else {
            self.landing_shake = 0.0;
        }

        // Smooth eye height transition
        let target_eye = if self.crouching { CROUCH_EYE_HEIGHT } else { EYE_HEIGHT };
        self.current_eye_height += (target_eye - self.current_eye_height)
            * (EYE_HEIGHT_LERP_SPEED * dt).min(1.0);

        // Walk cycle
        let step_rate = if self.sprinting { SPRINT_STEP_RATE } else { STEP_RATE };
        let moving = move_dir.length_squared() > 0.0 && self.on_ground;
        if moving {
            let old_half = (self.walk_cycle * 2.0) as u32;
            self.walk_cycle += dt * step_rate;
            let new_half = (self.walk_cycle * 2.0) as u32;
            if new_half > old_half {
                self.step_count = self.step_count.wrapping_add(1);
                self.step_pending = true;
            }
            self.walk_cycle %= 1.0;
        } else {
            self.walk_cycle = 0.0;
        }
    }

    /// Check if the player AABB at `pos` (feet) overlaps any solid block.
    fn collides_at_height(&self, pos: Vec3, cm: &ChunkManager) -> bool {
        let height = if self.crouching { CROUCH_HEIGHT } else { PLAYER_HEIGHT };
        let min_x = (pos.x - HALF_WIDTH).floor() as i32;
        let max_x = (pos.x + HALF_WIDTH).floor() as i32;
        let min_y = pos.y.floor() as i32;
        let max_y = (pos.y + height - 0.01).floor() as i32;
        let min_z = (pos.z - HALF_WIDTH).floor() as i32;
        let max_z = (pos.z + HALF_WIDTH).floor() as i32;

        for y in min_y..=max_y {
            for z in min_z..=max_z {
                for x in min_x..=max_x {
                    if cm.is_solid(x, y, z) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Snap feet to the top of the ground block below.
    fn snap_to_ground(&mut self, cm: &ChunkManager) {
        let bx = self.position.x.floor() as i32;
        let bz = self.position.z.floor() as i32;
        let start_y = self.position.y.floor() as i32;

        for y in (0..=start_y).rev() {
            if cm.is_solid(bx, y, bz) {
                self.position.y = (y + 1) as f32;
                return;
            }
        }
    }
}

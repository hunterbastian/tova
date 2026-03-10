use std::collections::HashSet;
use glam::Vec3;
use winit::keyboard::KeyCode;

use crate::voxel::chunk::CHUNK_SIZE;
use crate::world::ChunkManager;

// ─── Physics constants ──────────────────────────────────────
const GRAVITY: f32 = -25.0;
const JUMP_VELOCITY: f32 = 7.5;
const WALK_SPEED: f32 = 4.3;
const PLAYER_HEIGHT: f32 = 1.8;
const EYE_HEIGHT: f32 = 1.62;
const HALF_WIDTH: f32 = 0.28;
const TERMINAL_VELOCITY: f32 = -50.0;
const SENSITIVITY: f32 = 0.003;
const STEP_RATE: f32 = 2.8; // footsteps per second at walk speed

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
}

/// First-person player with gravity and voxel collision.
pub struct Player {
    /// Feet position in world space.
    pub position: Vec3,
    pub velocity: Vec3,
    pub on_ground: bool,
    pub yaw: f32,
    pub pitch: f32,
    walk_cycle: f32,
    step_count: u32,
    step_pending: bool,
}

impl Player {
    pub fn new(spawn: Vec3) -> Self {
        Self {
            position: spawn,
            velocity: Vec3::ZERO,
            on_ground: false,
            yaw: 0.0,
            pitch: 0.0,
            walk_cycle: 0.0,
            step_count: 0,
            step_pending: false,
        }
    }

    /// Eye position (camera attaches here).
    pub fn eye_position(&self) -> Vec3 {
        self.position + Vec3::new(0.0, EYE_HEIGHT, 0.0)
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
        self.yaw += dx as f32 * SENSITIVITY;
        self.pitch -= dy as f32 * SENSITIVITY;
        let limit = 89.0_f32.to_radians();
        self.pitch = self.pitch.clamp(-limit, limit);
    }

    /// Run one physics tick.
    pub fn update(&mut self, dt: f32, input: &Input, cm: &ChunkManager) {
        // Clamp dt to avoid physics explosions on lag spikes
        let dt = dt.min(0.05);

        // ─── Horizontal movement from input ─────────────
        let mut move_dir = Vec3::ZERO;
        if input.forward() {
            move_dir += self.forward_flat();
        }
        if input.back() {
            move_dir -= self.forward_flat();
        }
        if input.right() {
            move_dir += self.right();
        }
        if input.left() {
            move_dir -= self.right();
        }

        if move_dir.length_squared() > 0.0 {
            move_dir = move_dir.normalize();
        }

        self.velocity.x = move_dir.x * WALK_SPEED;
        self.velocity.z = move_dir.z * WALK_SPEED;

        // ─── Jump ───────────────────────────────────────
        if input.jump() && self.on_ground {
            self.velocity.y = JUMP_VELOCITY;
            self.on_ground = false;
        }

        // ─── Gravity ────────────────────────────────────
        self.velocity.y += GRAVITY * dt;
        self.velocity.y = self.velocity.y.max(TERMINAL_VELOCITY);

        // ─── Move with collision (axis by axis) ─────────
        // X axis
        let new_x = self.position.x + self.velocity.x * dt;
        if !self.collides_at(Vec3::new(new_x, self.position.y, self.position.z), cm) {
            self.position.x = new_x;
        } else {
            self.velocity.x = 0.0;
        }

        // Z axis
        let new_z = self.position.z + self.velocity.z * dt;
        if !self.collides_at(Vec3::new(self.position.x, self.position.y, new_z), cm) {
            self.position.z = new_z;
        } else {
            self.velocity.z = 0.0;
        }

        // Y axis
        let new_y = self.position.y + self.velocity.y * dt;
        if !self.collides_at(Vec3::new(self.position.x, new_y, self.position.z), cm) {
            self.position.y = new_y;
            self.on_ground = false;
        } else {
            if self.velocity.y < 0.0 {
                // Landed — snap to top of block
                self.position.y = (self.position.y as i32) as f32;
                // Fine-tune: find exact ground level
                self.snap_to_ground(cm);
                self.on_ground = true;
            }
            self.velocity.y = 0.0;
        }

        // ─── Walk cycle (footstep timing + foot bob) ──────
        let moving = move_dir.length_squared() > 0.0 && self.on_ground;
        if moving {
            let old_half = (self.walk_cycle * 2.0) as u32;
            self.walk_cycle += dt * STEP_RATE;
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
    fn collides_at(&self, pos: Vec3, cm: &ChunkManager) -> bool {
        let min_x = (pos.x - HALF_WIDTH).floor() as i32;
        let max_x = (pos.x + HALF_WIDTH).floor() as i32;
        let min_y = pos.y.floor() as i32;
        let max_y = (pos.y + PLAYER_HEIGHT - 0.01).floor() as i32;
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

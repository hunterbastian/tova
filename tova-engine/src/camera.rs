use bytemuck::{Pod, Zeroable};
use glam::{IVec3, Mat4, Vec3};

use crate::voxel::chunk::{SPAWN_CAMERA_Y, SPAWN_X, SPAWN_Z};
use crate::voxel::VoxelWorld;

const WALK_SPEED: f32 = 5.8;
const CAUTIOUS_SPEED_MULTIPLIER: f32 = 0.56;
const GROUND_ACCEL: f32 = 18.0;
const AIR_ACCEL: f32 = 5.0;
const GROUND_FRICTION: f32 = 11.0;
const JUMP_SPEED: f32 = 7.6;
const GRAVITY: f32 = 24.0;
const LOOK_SENSITIVITY: f32 = 0.0024;
const CAMERA_FOV_Y: f32 = 70.0;
const CAMERA_NEAR: f32 = 0.1;
const CAMERA_FAR: f32 = 600.0;
const PLAYER_RADIUS: f32 = 0.34;
const PLAYER_HEIGHT: f32 = 1.82;
const EYE_HEIGHT: f32 = 1.62;
const COLLISION_STEP: f32 = 0.12;
const SUPPORT_EPSILON: f32 = 0.06;

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub _pad: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct MoveIntent {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub cautious: bool,
}

pub struct Camera {
    pub position: Vec3,
    yaw: f32,
    pitch: f32,
    aspect: f32,
    velocity: Vec3,
    grounded: bool,
    fatigue: f32,
    jump_held: bool,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            position: Vec3::new(
                SPAWN_X as f32 + 0.5,
                SPAWN_CAMERA_Y as f32 + 0.5,
                SPAWN_Z as f32 + 0.5,
            ),
            yaw: 0.0,
            pitch: -0.08,
            aspect,
            velocity: Vec3::ZERO,
            grounded: false,
            fatigue: 1.0,
            jump_held: false,
        }
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    pub fn fatigue(&self) -> f32 {
        self.fatigue
    }

    #[cfg(test)]
    pub fn is_grounded(&self) -> bool {
        self.grounded
    }

    pub fn set_aspect(&mut self, aspect: f32) {
        self.aspect = aspect.max(0.1);
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn rotate(&mut self, dx: f64, dy: f64) {
        self.yaw += dx as f32 * LOOK_SENSITIVITY;
        self.pitch -= dy as f32 * LOOK_SENSITIVITY;
        let limit = 89.0_f32.to_radians();
        self.pitch = self.pitch.clamp(-limit, limit);
    }

    pub fn update(&mut self, dt: f32, intent: MoveIntent, world: &VoxelWorld) {
        let dt = dt.clamp(0.0, 0.05);
        if dt == 0.0 {
            return;
        }

        self.grounded = self.is_supported(world);
        if self.grounded && self.velocity.y < 0.0 {
            self.velocity.y = 0.0;
        }

        let forward = self.forward();
        let planar_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right = planar_forward.cross(Vec3::Y).normalize_or_zero();
        let mut wish_dir = Vec3::ZERO;

        if intent.forward {
            wish_dir += planar_forward;
        }
        if intent.back {
            wish_dir -= planar_forward;
        }
        if intent.right {
            wish_dir += right;
        }
        if intent.left {
            wish_dir -= right;
        }

        let is_moving = wish_dir.length_squared() > 0.0;
        if is_moving {
            wish_dir = wish_dir.normalize();
        }

        let speed_multiplier = if intent.cautious {
            CAUTIOUS_SPEED_MULTIPLIER
        } else {
            1.0
        };
        let fatigue_multiplier = 0.82 + self.fatigue * 0.18;
        let target_horizontal_velocity = wish_dir * WALK_SPEED * speed_multiplier * fatigue_multiplier;
        let accel = if self.grounded { GROUND_ACCEL } else { AIR_ACCEL };
        let blend = 1.0 - (-accel * dt).exp();

        let mut horizontal_velocity = Vec3::new(self.velocity.x, 0.0, self.velocity.z)
            .lerp(target_horizontal_velocity, blend);
        if !is_moving && self.grounded {
            let drag = (1.0 - GROUND_FRICTION * dt).max(0.0);
            horizontal_velocity *= drag;
        }
        self.velocity.x = horizontal_velocity.x;
        self.velocity.z = horizontal_velocity.z;

        let jumped = intent.jump && !self.jump_held && self.grounded;
        self.jump_held = intent.jump;
        if jumped {
            self.velocity.y = JUMP_SPEED;
            self.grounded = false;
        }
        if !self.grounded {
            self.velocity.y -= GRAVITY * dt;
        }

        let displacement = self.velocity * dt;
        let max_component = displacement.abs().max_element();
        let steps = ((max_component / COLLISION_STEP).ceil() as usize).clamp(1, 8);
        let step_dt = dt / steps as f32;

        for _ in 0..steps {
            self.integrate_step(step_dt, world);
        }

        self.grounded = self.is_supported(world);
        if self.grounded && self.velocity.y < 0.0 {
            self.velocity.y = 0.0;
        }

        self.update_fatigue(dt, is_moving, jumped);
    }

    pub fn occupies_block(&self, block_pos: IVec3) -> bool {
        let min = self.collision_min(self.position);
        let max = self.collision_max(self.position);
        let block_min = Vec3::new(block_pos.x as f32, block_pos.y as f32, block_pos.z as f32);
        let block_max = block_min + Vec3::ONE;

        min.x < block_max.x
            && max.x > block_min.x
            && min.y < block_max.y
            && max.y > block_min.y
            && min.z < block_max.z
            && max.z > block_min.z
    }

    pub fn uniform(&self) -> CameraUniform {
        let view = Mat4::look_at_rh(self.position, self.position + self.forward(), Vec3::Y);
        let proj = Mat4::perspective_rh(
            CAMERA_FOV_Y.to_radians(),
            self.aspect,
            CAMERA_NEAR,
            CAMERA_FAR,
        );

        CameraUniform {
            view_proj: (proj * view).to_cols_array_2d(),
            camera_pos: self.position.to_array(),
            _pad: 0.0,
        }
    }

    fn integrate_step(&mut self, dt: f32, world: &VoxelWorld) {
        let delta = self.velocity * dt;

        if delta.x != 0.0 {
            let candidate = self.position + Vec3::new(delta.x, 0.0, 0.0);
            if self.collides_at(world, candidate) {
                self.velocity.x = 0.0;
            } else {
                self.position.x = candidate.x;
            }
        }

        if delta.z != 0.0 {
            let candidate = self.position + Vec3::new(0.0, 0.0, delta.z);
            if self.collides_at(world, candidate) {
                self.velocity.z = 0.0;
            } else {
                self.position.z = candidate.z;
            }
        }

        if delta.y != 0.0 {
            let candidate = self.position + Vec3::new(0.0, delta.y, 0.0);
            if self.collides_at(world, candidate) {
                if delta.y < 0.0 {
                    self.grounded = true;
                }
                self.velocity.y = 0.0;
            } else {
                self.position.y = candidate.y;
                self.grounded = false;
            }
        }
    }

    fn update_fatigue(&mut self, dt: f32, is_moving: bool, jumped: bool) {
        if jumped {
            self.fatigue = (self.fatigue - 0.12).max(0.0);
        }

        let drain = if is_moving && self.grounded { 0.075 } else { 0.0 };
        let recover = if is_moving { 0.028 } else { 0.14 };
        self.fatigue = (self.fatigue - drain * dt + recover * dt).clamp(0.18, 1.0);
    }

    fn is_supported(&self, world: &VoxelWorld) -> bool {
        self.collides_at(world, self.position - Vec3::Y * SUPPORT_EPSILON)
    }

    fn collides_at(&self, world: &VoxelWorld, position: Vec3) -> bool {
        let min = self.collision_min(position);
        let max = self.collision_max(position) - Vec3::splat(0.001);
        let min_block = min.floor().as_ivec3();
        let max_block = max.floor().as_ivec3();

        for y in min_block.y..=max_block.y {
            for z in min_block.z..=max_block.z {
                for x in min_block.x..=max_block.x {
                    if world.sample_xyz(x, y, z).is_solid() {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn collision_min(&self, position: Vec3) -> Vec3 {
        Vec3::new(
            position.x - PLAYER_RADIUS,
            position.y - EYE_HEIGHT,
            position.z - PLAYER_RADIUS,
        )
    }

    fn collision_max(&self, position: Vec3) -> Vec3 {
        let min = self.collision_min(position);
        Vec3::new(
            min.x + PLAYER_RADIUS * 2.0,
            min.y + PLAYER_HEIGHT,
            min.z + PLAYER_RADIUS * 2.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Camera, MoveIntent};
    use crate::voxel::VoxelWorld;

    #[test]
    fn camera_moves_forward() {
        let world = VoxelWorld::generate(2);
        let mut camera = Camera::new(16.0 / 9.0);
        for _ in 0..120 {
            camera.update(
                1.0 / 60.0,
                MoveIntent {
                    forward: true,
                    ..MoveIntent::default()
                },
                &world,
            );
        }
        assert!(camera.position.z < 0.5);
    }

    #[test]
    fn camera_lands_after_spawn() {
        let world = VoxelWorld::generate(2);
        let mut camera = Camera::new(16.0 / 9.0);
        let start_y = camera.position.y;

        for _ in 0..180 {
            camera.update(1.0 / 60.0, MoveIntent::default(), &world);
        }

        assert!(camera.is_grounded());
        assert!(camera.position.y < start_y);
    }
}

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,   // radians, 0 = looking along -Z
    pub pitch: f32,  // radians, clamped to [-89°, 89°]
    pub aspect: f32,
    pub fov_y: f32,  // radians
    pub z_near: f32,
    pub z_far: f32,
    pub speed: f32,
    pub sensitivity: f32,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 10.0, 30.0),
            yaw: 0.0,
            pitch: 0.0,
            aspect,
            fov_y: 70.0_f32.to_radians(),
            z_near: 0.1,
            z_far: 1000.0,
            speed: 20.0,
            sensitivity: 0.003,
        }
    }

    /// Direction the camera is facing.
    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }

    /// Horizontal forward (for walking movement — no Y component).
    pub fn forward_flat(&self) -> Vec3 {
        Vec3::new(self.yaw.sin(), 0.0, -self.yaw.cos()).normalize()
    }

    /// Right direction.
    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    /// Apply mouse delta to yaw/pitch.
    pub fn rotate(&mut self, dx: f64, dy: f64) {
        self.yaw += dx as f32 * self.sensitivity;
        self.pitch -= dy as f32 * self.sensitivity;
        let limit = 89.0_f32.to_radians();
        self.pitch = self.pitch.clamp(-limit, limit);
    }

    /// Move the camera based on input flags. Flying mode — Y follows look direction.
    pub fn fly_move(&mut self, dt: f32, forward: bool, back: bool, left: bool, right: bool, up: bool, down: bool) {
        let mut dir = Vec3::ZERO;
        if forward { dir += self.forward(); }
        if back { dir -= self.forward(); }
        if right { dir += self.right(); }
        if left { dir -= self.right(); }
        if up { dir += Vec3::Y; }
        if down { dir -= Vec3::Y; }
        if dir.length_squared() > 0.0 {
            self.position += dir.normalize() * self.speed * dt;
        }
    }

    /// Build the view-projection matrix.
    pub fn build_view_proj(&self) -> CameraUniform {
        let view = Mat4::look_at_rh(self.position, self.position + self.forward(), Vec3::Y);
        let proj = Mat4::perspective_rh(self.fov_y, self.aspect, self.z_near, self.z_far);
        CameraUniform {
            view_proj: (proj * view).to_cols_array_2d(),
        }
    }
}

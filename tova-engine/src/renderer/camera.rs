use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 3],
    pub _pad: f32,
}

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,   // radians, 0 = looking along -Z
    pub pitch: f32,  // radians, clamped to [-89°, 89°]
    pub aspect: f32,
    pub fov_y: f32,  // radians
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            position: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            aspect,
            fov_y: 70.0_f32.to_radians(),
            z_near: 0.1,
            z_far: 1000.0,
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

    /// Build the view-projection matrix.
    pub fn build_view_proj(&self) -> CameraUniform {
        let view = Mat4::look_at_rh(self.position, self.position + self.forward(), Vec3::Y);
        let proj = Mat4::perspective_rh(self.fov_y, self.aspect, self.z_near, self.z_far);
        CameraUniform {
            view_proj: (proj * view).to_cols_array_2d(),
            camera_pos: self.position.to_array(),
            _pad: 0.0,
        }
    }
}

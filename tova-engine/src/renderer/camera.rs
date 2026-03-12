use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
    pub inv_view_proj: [[f32; 4]; 4],
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
        let vp = proj * view;
        CameraUniform {
            view_proj: vp.to_cols_array_2d(),
            inv_view_proj: vp.inverse().to_cols_array_2d(),
            camera_pos: self.position.to_array(),
            _pad: 0.0,
        }
    }

    /// Extract frustum planes from the current view-projection matrix.
    /// Returns 5 planes (left, right, bottom, top, far) as (normal, distance) pairs.
    /// Near plane is omitted since chunks close to the camera should always render.
    pub fn frustum_planes(&self) -> [glam::Vec4; 5] {
        let view = Mat4::look_at_rh(self.position, self.position + self.forward(), Vec3::Y);
        let proj = Mat4::perspective_rh(self.fov_y, self.aspect, self.z_near, self.z_far);
        let vp = proj * view;
        let c = vp.to_cols_array_2d();

        // Extract planes from the combined view-projection matrix rows
        let row = |r: usize| glam::Vec4::new(c[0][r], c[1][r], c[2][r], c[3][r]);
        let r0 = row(0); let r1 = row(1); let r2 = row(2); let r3 = row(3);

        let mut planes = [
            r3 + r0, // left
            r3 - r0, // right
            r3 + r1, // bottom
            r3 - r1, // top
            r3 - r2, // far
        ];

        // Normalize each plane
        for p in &mut planes {
            let len = Vec3::new(p.x, p.y, p.z).length();
            if len > 0.0 { *p /= len; }
        }

        planes
    }
}

use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

// #[rustfmt::skip]
// pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
//     1.0, 0.0, 0.0, 0.0,
//     0.0, 1.0, 0.0, 0.0,
//     0.0, 0.0, 0.5, 0.5,
//     0.0, 0.0, 0.0, 1.0,
// );

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub direction: cgmath::Vector3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn new(window_size: &PhysicalSize<u32>) -> Self {
        let eye_x = window_size.width as f32 / 2.0;
        let eye_y =  window_size.height as f32 / 2.0;
        Camera {
            eye: (eye_x, eye_y, eye_y).into(),
            direction: -cgmath::Vector3::unit_z(),
            up: cgmath::Vector3::unit_y(),
            aspect: window_size.width as f32 / window_size.height as f32,
            fovy: 90.0,
            znear: 0.1,
            zfar: eye_y + 50.0,
        }
    }
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_to_rh(self.eye, self.direction, self.up);

        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        proj * view
    }
}



#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

pub struct CameraState {
    pub camera: Camera,
    pub camera_uniform: CameraUniform,
    pub buffer: wgpu::Buffer
}

impl CameraState {
    pub fn new(camera: Camera, device: &wgpu::Device) -> Self {
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        CameraState {
            camera,
            camera_uniform,
            buffer,
        }
    }
}
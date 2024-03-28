use cgmath::Vector3;
use wgpu::util::DeviceExt;

use winit::event::ElementState;
use winit::event::WindowEvent;
use winit::event::KeyEvent;
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;

use std::sync::Mutex;
use once_cell::sync::Lazy;

use crate::geometry::BoundingBoxUniform;


pub static SIMULATION_PARAMETERS: Lazy<Mutex<SimulationParameters>> = Lazy::new(|| {
    Mutex::new(SimulationParameters::default())
});

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimulationParameters {
    pub particle_mass: f32,
    pub particle_radius: f32,
    pub particles_amount: u32,
    pub collision_damping: f32, 
    pub poly_kernel_radius: f32,
    pub spiky_kernel_radius: f32,
    pub viscosity_kernel_radius: f32,
    pub viscosity: f32,
    pub rest_density: f32,
    pub pressure_multiplier: f32,
    _padding: [u32; 2],
    pub bounding_box: BoundingBoxUniform,
    pub grid_size: f32,
    pub scene_scale_factor: f32,
    _padding2: [u32; 2],
    pub gravity: [f32; 3],
    _padding3: u32
}

impl SimulationParameters {
    pub fn apply_scale(&mut self) {
        self.particle_radius *= self.scene_scale_factor;

        let dimensions: Vector3<f32> = self.bounding_box.dimensions.into();
        self.bounding_box.dimensions = (dimensions * self.scene_scale_factor).into();

        self.grid_size = 2.0 * self.particle_radius + self.poly_kernel_radius;
    }
}

impl Default for SimulationParameters {
    fn default() -> Self {
        let width = 1600.0f32;
        let height = 900.0f32;
        let diagonal = (width * width + height * height).sqrt();
        let scene_scale_factor = 35.0 / diagonal;


        let particle_mass = 1.0;
        let particle_radius = 3.0;
        let particles_amount = 10000;
        let collision_damping = 0.9;
        let poly_kernel_radius = 1.0;
        let spiky_kernel_radius = 0.3;
        let viscosity_kernel_radius = 1.0;
        let viscosity = 0.01;
        let rest_density = 1.57;
        let pressure_multiplier = 250.0;
        let bounding_box = BoundingBoxUniform::new(Vector3::new(0.0, 0.0, 0.0),  Vector3::new(width, height, 1.0));
        let grid_size = poly_kernel_radius;
        SimulationParameters {
            particle_mass,
            particle_radius,
            particles_amount,
            collision_damping,
            poly_kernel_radius,
            spiky_kernel_radius,
            viscosity_kernel_radius,
            viscosity,
            rest_density,
            pressure_multiplier,
            bounding_box,
            grid_size,
            scene_scale_factor,
            gravity: [0.0, -98.0, 0.0],
            _padding3: 0,
            _padding2: [0,0],
            _padding: [0,0]
        }
    }
}

pub struct SimulationParametersState {
   pub buffer: wgpu::Buffer
}

impl SimulationParametersState {
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Simulation parameters"),
            contents: bytemuck::cast_slice(&[*SIMULATION_PARAMETERS.lock().unwrap()]),
            usage: wgpu::BufferUsages::UNIFORM
                |  wgpu::BufferUsages::COPY_DST
        });

        SimulationParametersState {
            buffer
        }
    }
}

#[derive(Default)]
pub struct ParametersControls {
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_a_pressed: bool,
    is_s_pressed: bool,
    is_g_pressed: bool,
    is_p_pressed: bool,
    is_r_pressed: bool,
    is_w_pressed: bool,
    is_z_pressed: bool,
    is_v_pressed: bool,
    is_b_pressed: bool,
}

impl ParametersControls {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent  {
                    state,
                    physical_key: PhysicalKey::Code(keycode),
                    ..
                },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    KeyCode::ArrowLeft => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::ArrowRight => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyA => {
                        self.is_a_pressed = is_pressed;
                        true
                    },
                    KeyCode::KeyS => {
                        self.is_s_pressed = is_pressed;
                        true
                    },
                    KeyCode::KeyG => {
                        self.is_g_pressed = is_pressed;
                        true
                    },
                    KeyCode::KeyP => {
                        self.is_p_pressed = is_pressed;
                        true
                    },
                    KeyCode::KeyR => {
                        self.is_r_pressed = is_pressed;
                        true
                    },
                    KeyCode::KeyW => {
                        self.is_w_pressed = is_pressed;
                        true
                    },
                    KeyCode::KeyZ => {
                        self.is_z_pressed = is_pressed;
                        true
                    },
                    KeyCode::KeyV => {
                        self.is_v_pressed = is_pressed;
                        true
                    },
                    KeyCode::KeyB => {
                        self.is_b_pressed = is_pressed;
                        true
                    },
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update(&self, params: &mut SimulationParameters) {
        let mut dir = 0.0;

        if self.is_left_pressed {
            dir = -1.0
        }
        else if self.is_right_pressed {
            dir = 1.0
        }

        if self.is_a_pressed  {
            params.pressure_multiplier += 0.1 * dir;
            println!("Pressure multi: {}", params.pressure_multiplier);
        } else if self.is_g_pressed  {
            let mut g: Vector3<f32> = params.gravity.into(); 
            g.y += 0.1 * dir;
            params.gravity = g.into();
            println!("Gravity: {:?}", params.gravity);
        } else if self.is_p_pressed  {
            params.poly_kernel_radius += 0.005 * dir;
            println!("Poly kernel radius: {}", params.poly_kernel_radius);
        } else if self.is_s_pressed  {
            params.spiky_kernel_radius += 0.005 * dir;
            println!("Spiky kernel radius: {}", params.spiky_kernel_radius);
        } else if self.is_r_pressed  {
            params.rest_density += 0.01 * dir;
            println!("Rest density: {}", params.rest_density);
        } else if self.is_w_pressed  {
            params.scene_scale_factor += 0.001 * dir;
            println!("Scale factor: {}", params.scene_scale_factor);
        } else if self.is_z_pressed  {
            params.grid_size += 0.01 * dir;
            println!("Grid size: {}", params.grid_size);
        } else if self.is_v_pressed  {
            params.viscosity += 0.01 * dir;
            println!("Viscosity: {}", params.viscosity);
        } else if self.is_b_pressed  {
            params.viscosity_kernel_radius += 0.005 * dir;
            println!("Viscosity kernel radius: {}", params.viscosity_kernel_radius);
        }
    }
}
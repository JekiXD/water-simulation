use cgmath::Vector3;
use wgpu::util::DeviceExt;
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
    pub rest_density: f32,
    pub pressure_multiplier: f32,
    pub bounding_box: BoundingBoxUniform,
    pub grid_size: u32,
    _padding: [u32; 3]
}

impl Default for SimulationParameters {
    fn default() -> Self {
        let particle_mass = 1.0;
        let particle_radius = 1.0;
        let particles_amount = 10000;
        let collision_damping = 0.9;
        let poly_kernel_radius = 1.0;
        let spiky_kernel_radius = 0.9;
        let rest_density = 2.0;
        let pressure_multiplier = 10.0;
        let bounding_box = BoundingBoxUniform::new(Vector3::new(0.0, 0.0, 0.0),  Vector3::new(1600.0, 900.0, 1.0));
        SimulationParameters {
            particle_mass,
            particle_radius,
            particles_amount,
            collision_damping,
            poly_kernel_radius,
            spiky_kernel_radius,
            rest_density,
            pressure_multiplier,
            bounding_box: bounding_box,
            grid_size: (2.0 * particle_radius + 1.0) as u32,
            _padding: [0, 0, 0]
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
        });

        SimulationParametersState {
            buffer
        }
    }
}
use wgpu::util::DeviceExt;
use std::sync::Mutex;
use once_cell::sync::Lazy;


pub static SIMULATION_PARAMETERS: Lazy<Mutex<SimulationParameters>> = Lazy::new(|| {
    Mutex::new(SimulationParameters::default())
});

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SimulationParameters {
    pub particle_mass: f32,
    pub particle_radius: f32,
    pub particles_amount: u32,
    _padding: u32
}

impl Default for SimulationParameters {
    fn default() -> Self {
        SimulationParameters {
            particle_mass: 1.0,
            particle_radius: 3.0,
            particles_amount: 2500,
            _padding: 0
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
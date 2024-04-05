use wgpu::util::DeviceExt;
use std::sync::Mutex;
use once_cell::sync::Lazy;

pub static SIMULATION_PARAMETERS: Lazy<Mutex<settings::SimulationParameters>> = Lazy::new(|| {
    Mutex::new(settings::SimulationParameters::default())
});

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

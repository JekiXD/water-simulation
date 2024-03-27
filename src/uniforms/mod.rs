use cgmath::{Vector3, Zero};
use winit::dpi::PhysicalSize;

use crate::geometry::BoundingBoxUniform;

use self::{camera::{Camera, CameraState}, frame_time::{FrameTime, FrameTimeState}, parameters::SimulationParametersState};


pub mod camera;
pub mod frame_time;
pub mod parameters;

pub struct UniformState {
    camera: CameraState,
    frame_time: FrameTimeState,
    simulation_parameters: SimulationParametersState,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout
}

impl UniformState {
    pub fn new(device: &wgpu::Device, window_size: &PhysicalSize<u32>) -> Self {
        let camera = CameraState::new(Camera::new(window_size), device);
        let frame_time = FrameTimeState::new(device);
        let simulation_parameters = SimulationParametersState::new(device);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                //Camera
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                //Frame time
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                //Simulation parameters
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Uniform bind group layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { 
                label: Some("Uniform bind group"), 
                layout: &bind_group_layout, 
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: camera.buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: frame_time.buffer.as_entire_binding()
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: simulation_parameters.buffer.as_entire_binding()
                    },
                ]
        });

        UniformState {
            camera,
            frame_time,
            simulation_parameters,
            bind_group,
            bind_group_layout
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.frame_time.update(queue);
    }
}
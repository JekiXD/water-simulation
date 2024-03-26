use cgmath::{Vector3, Vector4};
use wgpu::util::DeviceExt;
use wgpu::Label;

pub const SEGMENTS: u32 = 32;

use crate::geometry;
use crate::uniforms::parameters::SIMULATION_PARAMETERS;


#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
    pub color: Vector4<f32>,
}

impl Particle {
    pub fn new(position: Vector3<f32>, velocity: Vector3<f32>, color: Vector4<f32>) -> Self {
        Self {
            position,
            velocity,
            color
        }
    }

    pub fn into_raw(&self) -> ParticleRaw {
        ParticleRaw {
            position: self.position.into(),
            velocity: self.velocity.into(),
            color: self.color.into(),
            ..Default::default()
        }
    }

    pub fn circle_mesh() -> geometry::Mesh {
        let param = SIMULATION_PARAMETERS.lock().unwrap();
        geometry::circle(param.particle_radius, SEGMENTS)
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleRaw {
    position: [f32; 3],
    _padding: u32,
    velocity: [f32; 3],
    _padding2: u32,
    color: [f32; 4]
}

impl ParticleRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout { 
            array_stride: mem::size_of::<ParticleRaw>() as wgpu::BufferAddress, 
            step_mode: wgpu::VertexStepMode::Instance, 
            attributes: &[
                wgpu::VertexAttribute{
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x3
                }, 
                wgpu::VertexAttribute{
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x3
                },
                wgpu::VertexAttribute{
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4
                } 
            ] 
        }
    }
}

pub struct ParticlesState {
    pub particles: Vec<Particle>,
    pub density_field: Vec<f32>,
    pub pressure_field: Vec<f32>,
    pub particles_buffer: wgpu::Buffer,
    pub density_field_buffer: wgpu::Buffer,
    pub pressure_field_buffer: wgpu::Buffer,
    pub particles_bind_group: wgpu::BindGroup,
    pub fields_bind_group: wgpu::BindGroup,
    pub particles_bind_group_layout: wgpu::BindGroupLayout,
    pub fields_bind_group_layout: wgpu::BindGroupLayout
}

impl ParticlesState {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let param = SIMULATION_PARAMETERS.lock().unwrap();

        let mut particles = Vec::new();
        let start_pos = Vector3::new(10.0, 10.0, 0.0);
        let spacing = 1.0;
        let dis = 2.0 * param.particle_radius + spacing;

        let particles_per_row = (param.particles_amount as f32).sqrt() as u32;
        let particles_per_col = (param.particles_amount - 1) / particles_per_row + 1;

        let color = Vector4::new(0.0, 0.71, 0.93, 1.0);
        let velocity = Vector3::new(0.0, 0.0, 0.0);

        for i in 0..param.particles_amount {
            let x = ((i % particles_per_row) as f32 + 0.5) * dis;
            let y = ((i / particles_per_col) as f32 + 0.5) * dis;
            if i == 20032 {
                println!("{x}:{y}");
            }
            let position = start_pos + Vector3::new(x, y, 0.0);
            particles.push(Particle::new(position, velocity, color));
        }

        //println!("{:?}", particles);

        let density_field = vec![1.0; particles.len()];
        let pressure_field = vec![1.0; particles.len()];

        let particles_raw: Vec<_> = particles.iter().map(|p| p.into_raw()).collect();

        let particles_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Particles"),
                contents: bytemuck::cast_slice(&particles_raw),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE
            }
        );
        let density_field_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Density"),
                contents: bytemuck::cast_slice(&density_field),
                usage: wgpu::BufferUsages::STORAGE
            }
        );
        let pressure_field_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Pressure"),
                contents: bytemuck::cast_slice(&pressure_field),
                usage: wgpu::BufferUsages::STORAGE
            }
        );

        let particles_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                //Particles
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("Particles bind group layout")
        });

        let fields_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                //Density field
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                //Pressure field
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("Fields bind group layout")
        });

        let particles_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { 
            label: Some("Particles bind group"), 
            layout: &particles_bind_group_layout, 
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particles_buffer.as_entire_binding(),
                },
                ]
            });

        let fields_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { 
            label: Some("Fields bind group"), 
            layout: &fields_bind_group_layout, 
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: density_field_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: pressure_field_buffer.as_entire_binding()
                },
            ]
        });

        ParticlesState {
            particles,
            density_field,
            pressure_field,
            particles_buffer,
            density_field_buffer,
            pressure_field_buffer,
            particles_bind_group,
            fields_bind_group,
            particles_bind_group_layout,
            fields_bind_group_layout
        }
    }
}
use cgmath::Vector3;
use wgpu::util::DeviceExt;

pub const PARTICLE_MASS: f32 = 1.0;
pub const RADIUS: f32 = 2.0;
pub const SEGEMTS: u32 = 32;

use crate::geometry;


#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub position: Vector3<f32>,
    pub velocity: Vector3<f32>,
}

impl Particle {
    pub fn new(position: Vector3<f32>, velocity: Vector3<f32>) -> Self {
        Self {
            position,
            velocity
        }
    }

    pub fn into_raw(&self) -> ParticleRaw {
        ParticleRaw {
            position: self.position.into(),
            velocity: self.velocity.into()
        }
    }

    pub fn circle_mesh() -> geometry::Mesh {
        geometry::circle(RADIUS, SEGEMTS)
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleRaw {
    position: [f32; 3],
    velocity: [f32; 3]
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
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x3
                } 
            ] 
        }
    }
}

pub struct ParticleList {
    particles: Vec<Particle>
}

impl ParticleList {
    pub fn init(amount: u32, config: &wgpu::SurfaceConfiguration) -> Self {
        let mut particles = Vec::new();
        let start_pos = Vector3::new(10.0, 10.0, 0.0);
        let spacing = 1.0;
        let dis = 2.0 * RADIUS + spacing;
        let numcols = (amount as f32).sqrt() as u32;

        for y in 0..numcols {
            for x in 0..numcols {
                let position = start_pos + Vector3::new(x as f32 * dis, y as f32 * dis, 0.0);
                let velocity = Vector3::new(0.0, 0.0, 0.0);
                particles.push(Particle::new(position, velocity));
            }
        }

        //println!("{:?}", particles);

        ParticleList {
            particles
        }
    }

    pub fn into_buffer(&self, device: &wgpu::Device) -> ParticleBuffer {
        let num_particles = self.particles.len() as u32;
        let particles: Vec<_> = self.particles.iter().map(|p| p.into_raw()).collect();

        ParticleBuffer {
            num_particles,
            buffer: device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Particles"),
                    contents: bytemuck::cast_slice(&particles),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE
                }
            )
        }
    }
}

pub struct ParticleBuffer {
    pub num_particles: u32,
    pub buffer: wgpu::Buffer
}
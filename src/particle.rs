use std::num::NonZeroU32;

use cgmath::{Vector2, Vector3, Vector4};
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
    _padding: f32,
    velocity: [f32; 3],
    _padding2: f32,
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
    pub near_density_field: Vec<f32>,
    pub predicted_positions: Vec<[f32;4]>,
    pub surface_normals: Vec<[f32;4]>,

    pub particles_buffer: wgpu::Buffer,
    pub density_field_buffer: wgpu::Buffer,
    pub near_density_field_buffer: wgpu::Buffer,
    pub predicted_positions_buffer: wgpu::Buffer,
    pub surface_normals_buffer: wgpu::Buffer,

    pub particles_bind_group: wgpu::BindGroup,
    pub fields_bind_group: wgpu::BindGroup,
    pub particles_bind_group_layout: wgpu::BindGroupLayout,
    pub fields_bind_group_layout: wgpu::BindGroupLayout
}

impl ParticlesState {
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let sim = SIMULATION_PARAMETERS.lock().unwrap();
        let screen_size = sim.bounding_box.dimensions;

        let mut particles = Vec::new();

        let spacing = 3.0;
        let dis = 2.0 * sim.particle_radius + spacing;

        let particles_per_row = (sim.particles_amount as f32).sqrt() as u32;
        let particles_per_col = (sim.particles_amount - 1) / particles_per_row + 1;

        let start_pos = Vector3::new(screen_size[0] / 2.0, screen_size[1] / 2.0, 0.0);

        let color = Vector4::new(0.0, 0.71, 0.93, 1.0);
        let velocity = Vector3::new(0.0, 0.0, 0.0);

        for i in 0..sim.particles_amount  {
            let x: f32 = ((i % particles_per_row) as f32 - particles_per_row as f32 / 2.0 + 0.5) * dis;
            let y: f32 = ((i / particles_per_row) as f32 - particles_per_col as f32 / 2.0 + 0.5) * dis;
            let position = start_pos + Vector3::new(x, y, 0.0);
            particles.push(Particle::new(position, velocity, color));
        }

        // sim.apply_scale();

        // println!("{particles_per_row}:{particles_per_col}");
        
        // for i in 0..particles.len() {
        //     println!("{i}:{:?}", particles[i]);
        // }

        let density_field = vec![0.0; particles.len()];
        let near_density_field = vec![0.0; particles.len()];
        let predicted_positions = vec![[0.0, 0.0, 0.0, 0.0]; particles.len()];
        let surface_normals = vec![[0.0, 0.0, 0.0, 0.0]; particles.len()];

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
        let near_density_field_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Near Density"),
                contents: bytemuck::cast_slice(&near_density_field),
                usage: wgpu::BufferUsages::STORAGE
            }
        );
        let predicted_positions_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Predicted position"),
                contents: bytemuck::cast_slice(&predicted_positions),
                usage: wgpu::BufferUsages::STORAGE
            }
        );
        let surface_normals_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Surface normal"),
                contents: bytemuck::cast_slice(&surface_normals),
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
                //Predicted positions
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
                //Surface normal
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::all(),
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                 //Near density
                 wgpu::BindGroupLayoutEntry {
                    binding: 3,
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
                    resource: predicted_positions_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: surface_normals_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: near_density_field_buffer.as_entire_binding()
                },
            ]
        });

        ParticlesState {
            particles,
            density_field,
            near_density_field,
            predicted_positions,
            surface_normals,
            particles_buffer,
            density_field_buffer,
            near_density_field_buffer,
            predicted_positions_buffer,
            surface_normals_buffer,
            particles_bind_group,
            fields_bind_group,
            particles_bind_group_layout,
            fields_bind_group_layout
        }
    }
}


pub struct NeighbourSearchGridState {
    //All buffers are Vec<u32>
    pub key_cell_hash_buffer: wgpu::Buffer,
    pub value_particle_id_buffer: wgpu::Buffer,
    pub cell_start_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout
}

impl NeighbourSearchGridState {
    pub fn new(device: &wgpu::Device, length: u32) -> Self {
        let key_cell_hash_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Key cell hash buffer"),
            size: (std::mem::size_of::<u32>() * length as usize) as u64,
            usage: wgpu::BufferUsages::STORAGE
                |  wgpu::BufferUsages::COPY_SRC
                |  wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false  
        });

        let value_particle_id_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Key cell hash buffer"),
            size: (std::mem::size_of::<u32>() * length as usize) as u64,
            usage: wgpu::BufferUsages::STORAGE
                |  wgpu::BufferUsages::COPY_SRC
                |  wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false  
        });

        let cell_start_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Key cell hash buffer"),
            size: (std::mem::size_of::<u32>() * length as usize) as u64,
            usage: wgpu::BufferUsages::STORAGE |  wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false  
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor{
            label: Some("NeighbourSearchGridState bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry{
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { 
                        ty: wgpu::BufferBindingType::Storage { read_only: false }, 
                        has_dynamic_offset: false, 
                        min_binding_size: None 
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { 
                        ty: wgpu::BufferBindingType::Storage { read_only: false }, 
                        has_dynamic_offset: false, 
                        min_binding_size: None 
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry{
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { 
                        ty: wgpu::BufferBindingType::Storage { read_only: false }, 
                        has_dynamic_offset: false, 
                        min_binding_size: None 
                    },
                    count: None
                },
            ]
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor { 
            label: Some("NeighbourSearchGridState bind group"), 
            layout: &bind_group_layout, 
            entries: &[
                wgpu::BindGroupEntry{
                    binding: 0,
                    resource: key_cell_hash_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry{
                    binding: 1,
                    resource: value_particle_id_buffer.as_entire_binding()
                },
                wgpu::BindGroupEntry{
                    binding: 2,
                    resource: cell_start_buffer.as_entire_binding()
                },
            ] 
        });

        NeighbourSearchGridState {
            key_cell_hash_buffer, 
            value_particle_id_buffer,
            cell_start_buffer,
            bind_group,
            bind_group_layout
        }
    }
}

pub struct NeighbourSearchSortState {
    pub grid_state: NeighbourSearchGridState,
    pub sorter: wgpu_sort::GPUSorter,
    pub sort_buffers: wgpu_sort::SortBuffers
}

impl NeighbourSearchSortState {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, subgroup_size: u32) -> Self {
        let sim = SIMULATION_PARAMETERS.lock().unwrap();
        let grid_state = NeighbourSearchGridState::new(device, sim.particles_amount);
        let sorter = wgpu_sort::GPUSorter::new(device, subgroup_size);
        let sort_buffers = sorter.create_sort_buffers(device, NonZeroU32::new(sim.particles_amount).unwrap());

        NeighbourSearchSortState {
            grid_state,
            sorter,
            sort_buffers
        }
    }

    pub fn sort(&self, encoder: &mut wgpu::CommandEncoder, queue: &wgpu::Queue) {
        //Copy keys
        encoder.copy_buffer_to_buffer(
            &self.grid_state.key_cell_hash_buffer, 
            0, 
            self.sort_buffers.keys(), 
            0,
            self.grid_state.key_cell_hash_buffer.size());
        //Copy values
        encoder.copy_buffer_to_buffer(
            &self.grid_state.value_particle_id_buffer, 
            0, 
            self.sort_buffers.values(), 
            0,
            self.grid_state.value_particle_id_buffer.size());

        self.sorter.sort(encoder, queue, &self.sort_buffers, None);

        //Copy keys back
        encoder.copy_buffer_to_buffer(
            self.sort_buffers.keys(), 
            0, 
            &self.grid_state.key_cell_hash_buffer, 
            0,
            self.grid_state.key_cell_hash_buffer.size());
        //Copy values back
        encoder.copy_buffer_to_buffer(
            self.sort_buffers.values(), 
            0, 
            &self.grid_state.value_particle_id_buffer, 
            0,
            self.grid_state.value_particle_id_buffer.size());
    }
}

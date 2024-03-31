
use cgmath::{Vector3, Vector4};
use wgpu::util::DeviceExt;

use  crate::{vertex::Vertex};

pub struct Mesh {
    pub indices:  Vec<u16>,
    pub vertices: Vec<Vertex>,
    pub normals: Vec<Vector3<f32>>
}

pub struct MeshBuffer {
    pub num_indices: u32,
    pub num_vertices: u32,
    pub indices:  wgpu::Buffer,
    pub vertices: wgpu::Buffer,
    pub normals: wgpu::Buffer
}

impl Mesh {
    pub fn into_buffer(&self, device: &wgpu::Device) -> MeshBuffer {
        let num_indices = self.indices.len() as u32;
        let num_vertices = self.vertices.len() as u32;
        let vertices: Vec<_> = self.vertices.iter().map(|v| v.into_raw()).collect();
        let normals: Vec<[f32; 3]> = self.normals.iter().map(|&n| n.into()).collect();

        MeshBuffer {
            num_indices,
            num_vertices,
            indices: device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh indices"),
                    contents: bytemuck::cast_slice(&self.indices),
                    usage: wgpu::BufferUsages::INDEX
                }
            ),
            vertices: device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh vertices"),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX
                }
            ),
            normals: device.create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: Some("Mesh normals"),
                    contents: bytemuck::cast_slice(&normals),
                    usage: wgpu::BufferUsages::VERTEX
                }
            )
        }
    }
}

pub fn circle(radius: f32, segments: u32) -> Mesh {
    let mut vertices = vec![];
    let mut indices = vec![];
    let mut normals = vec![];
    let theta_start = 0.0;
    let theta_length = std::f32::consts::PI * 2.0;

    let mut vertex = Vertex::new(Vector3::new(0.0, 0.0, 0.0), Vector4::new(0.0, 0.71, 0.93, 1.0));

    vertices.push(vertex);
    normals.push(Vector3::new(0.0, 0.0, 1.0));

    let mut i = 0;
    for s in 0..=segments {
        let segment = theta_start + s as f32 / segments as f32 * theta_length;

        vertex.position.x = radius * (segment as f32).cos();
        vertex.position.y = radius * (segment as f32).sin();

        vertices.push(vertex);
        normals.push(Vector3::new(0.0, 0.0, 1.0));

        i += 3;
    }

    for i in 1..=segments {
        let i = i as u16;
        indices.push(i);
        indices.push(i + 1);
        indices.push(0);
    }

    Mesh {
        indices,
        vertices,
        normals
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BoundingBoxUniform{
    pub position: [f32; 3],
    _padding: u32,
    pub dimensions: [f32; 3],
    _padding1: u32,
}

impl BoundingBoxUniform {
    pub fn new(position: Vector3<f32>, dimensions: Vector3<f32>) -> Self {
        BoundingBoxUniform {
            position: position.into(),
            dimensions: dimensions.into(),
            ..Default::default()
        }
    }
}


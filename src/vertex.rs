use cgmath::{Vector3, Vector4};

pub fn get_screen_scquare_vertices(config: &wgpu::SurfaceConfiguration) -> [VertexRaw; 6] {
    let width = config.width as f32;
    let height = config.height as f32;
    [   
        VertexRaw { position: [0.0, 0.0, 0.0], color: [0.2, 0.2, 0.2, 0.1]},
        VertexRaw { position: [width, 0.0, 0.0], color: [0.2, 0.2, 0.2, 0.1]},
        VertexRaw { position: [width, height, 0.0], color: [0.2, 0.2, 0.2, 0.1]},
        VertexRaw { position: [0.0, 0.0, 0.0], color: [0.2, 0.2, 0.2, 0.1]},
        VertexRaw { position: [width, height, 0.0], color: [0.2, 0.2, 0.2, 0.1]},
        VertexRaw { position: [0.0, height, 0.0], color: [0.2, 0.2, 0.2, 0.1]},
    ]
}

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub color:  Vector4<f32>,
}

impl Vertex {
    pub fn new(position: Vector3<f32>, color: Vector4<f32>) -> Self {
        Vertex {
            position,
            color
        }
    }

    pub fn into_raw(&self) -> VertexRaw {
        VertexRaw {
            position: self.position.into(),
            color: self.color.into()
        }
    }
}


#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexRaw {
    pub position: [f32; 3],
    pub color: [f32; 4],
}

impl VertexRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<VertexRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                }
            ]
        }
    }
}
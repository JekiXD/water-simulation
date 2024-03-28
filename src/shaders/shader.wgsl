struct Particle {
    @location(5) position: vec3<f32>,
    @location(6) velocity: vec3<f32>,
    @location(7) color: vec4<f32>,
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

struct BoundingBox {
  position: vec3<f32>,
  dimensions: vec3<f32>,
}

struct SimulationParameters {
  particle_mass: f32,
  particle_radius: f32,
  particles_amount: u32,
  collision_damping: f32, 
  poly_kernel_radius: f32,
  spiky_kernel_radius: f32,
  viscosity_kernel_radius: f32,
  viscosity: f32,
  rest_density: f32,
  pressure_multiplier: f32,
  bounding_box: BoundingBox,
  grid_size: f32,
  scene_scale_factor: f32,
  gravity: vec3<f32>
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(2) var<uniform> sim: SimulationParameters;

///
//Vertex
///
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) pos: vec4<f32>
};

@vertex
fn vs_main(
    vertex: VertexInput,
    particle: Particle
)
-> VertexOutput {
    var out: VertexOutput;

    let pos = vertex.position + particle.position;

    out.clip_position = camera.view_proj * vec4<f32>(pos, 1.0);

    out.color = particle.color;
    out.pos =  vec4<f32>(pos, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    return in.color;
}
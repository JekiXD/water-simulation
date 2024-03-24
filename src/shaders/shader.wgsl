struct Particle {
    @location(5) position: vec3<f32>,
    @location(6) velocity: vec3<f32>
}

struct Particles {
  particles : array<Particle>,
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;

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
    out.color = vertex.color;
    out.pos =  vec4<f32>(pos, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32>{
    return in.color;
}
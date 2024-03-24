const g: vec3<f32> = vec3<f32>(0.0, -9.8, 0.0);
const scene_scale_factor: f32 = 100.0;

struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>
}

struct Particles {
  particles : array<Particle>,
}

@group(0) @binding(0) var<storage, read_write> data : Particles;
@group(1) @binding(1) var<uniform> elapsed_secs: f32;

@compute @workgroup_size(64)
fn simulate(@builtin(global_invocation_id) global_invocation_id : vec3u) {
    let idx = global_invocation_id.x;

    var particle = scale_particle_value(data.particles[idx], scene_scale_factor);

    particle.velocity = particle.velocity + elapsed_secs * g;
    particle.position = particle.position + elapsed_secs * particle.velocity;

    data.particles[idx] = scale_particle_value(particle, 1/scene_scale_factor);
}

fn scale_particle_value(particle: Particle, scale_factor: f32) -> Particle {
  var p: Particle;

  p.position = particle.position / scale_factor;
  p.velocity = particle.velocity / scale_factor;

  return p;
}
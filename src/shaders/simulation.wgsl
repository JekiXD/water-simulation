const scene_scale_factor: f32 = 50.0;
const g: vec3<f32> = vec3<f32>(0.0, -9.8, 0.0);
const collision_damping = 0.9;
const kernel_radius = 1.0;
const rest_density = 1.567;
const pressure_multiplier = 10.0;

var<private> rand_seed : vec2f;

fn init_rand(invocation_id : u32, seed : vec4f) {
  rand_seed = seed.xz;
  rand_seed = fract(rand_seed * cos(35.456+f32(invocation_id) * seed.yw));
  rand_seed = fract(rand_seed * cos(41.235+f32(invocation_id) * seed.xw));
}

fn rand() -> f32 {
  rand_seed.x = fract(cos(dot(rand_seed, vec2f(23.14077926, 232.61690225))) * 136.8168);
  rand_seed.y = fract(cos(dot(rand_seed, vec2f(54.47856553, 345.84153136))) * 534.7645);
  return rand_seed.y;
}

struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    color: vec4<f32>
}

struct Particles {
  particles : array<Particle>,
}

struct BoundingBox {
  position: vec3<f32>,
  dimensions: vec3<f32>,
}

struct SimulationParameters {
  particle_mass: f32,
  particle_radius: f32,
  particles_amount: u32,
}

@group(0) @binding(0) var<storage, read_write> data : Particles;
@group(1) @binding(0) var<storage, read_write> density_field : array<f32>;
@group(1) @binding(1) var<storage, read_write> pressure_field : array<f32>;
@group(2) @binding(1) var<uniform> elapsed_secs: f32;
@group(2) @binding(2) var<uniform> bounding_box: BoundingBox;
@group(2) @binding(3) var<uniform> simulation_parameters: SimulationParameters;

@compute @workgroup_size(64)
fn simulate(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  init_rand(idx, vec4f(elapsed_secs));

  var particle = data.particles[idx];
  scale_particle_value(&particle, scene_scale_factor);

  //Apply gravity
  //particle.velocity += elapsed_secs * g;
  //Apply pressure
  let pressure_accel = -compute_pressure_force(idx, &particle) / density_field[idx];
  particle.velocity += elapsed_secs * pressure_accel;
  //some(idx);

  particle.position += elapsed_secs * particle.velocity;

  scale_particle_value(&particle, 1/scene_scale_factor);
  compute_collisions(&particle);

  //scale_particle_value(&particle, 1/scene_scale_factor);

  data.particles[idx] = particle;
}

fn some(idx: u32) {
  let s = pressure_field[idx];
}

fn compute_pressure_force(idx: u32, particle: ptr<function, Particle>) -> vec3<f32>{
  var pressure_force = vec3<f32>(0.0, 0.0, 0.0);

  for(var i = u32(0); i < simulation_parameters.particles_amount; i++) {
    if (i == idx) { continue; }

    var particle2 = data.particles[i];
    scale_particle_value(&particle2, scene_scale_factor);

    let vector = particle2.position - (*particle).position;
    let distance = length(vector);
    var dir = normalize(vector);

    if (distance == 0.0) { dir = vec3<f32>(rand(), rand(), 0.0); }

    let average_pressure = (pressure_field[idx] + pressure_field[i]) / 2.0;
    let res = simulation_parameters.particle_mass * average_pressure * spiky_kernel_derivative(distance, kernel_radius) / density_field[i];

    pressure_force +=  -dir * res;
  }

  return pressure_force;
}

fn compute_collisions(particle: ptr<function, Particle>)  {
  let dimensions = bounding_box.dimensions;
  let particle_radius = simulation_parameters.particle_radius;

  var p1 = bounding_box.position + particle_radius + 5.0;
  var p2 = bounding_box.position + dimensions - particle_radius - 5.0;

  var pos = (*particle).position;
  var vel = (*particle).velocity;
  
  if pos.x < p1.x || pos.x > p2.x {
    pos.x = clamp(pos.x, p1.x, p2.x);
    vel.x *= -collision_damping;
  }

  if pos.y < p1.y || pos.y > p2.y {
    pos.y = clamp(pos.y, p1.y, p2.y);
    vel.y *= -collision_damping;
  }

  (*particle).position = pos;
  (*particle).velocity = vel;
}

@compute @workgroup_size(64)
fn compute_density_and_pressure(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;

  var particle = data.particles[idx];
  scale_particle_value(&particle, scene_scale_factor);

  var density = 0.0;

  for(var i = u32(0); i < simulation_parameters.particles_amount; i++) {
    var particle2 = data.particles[i];
    scale_particle_value(&particle2, scene_scale_factor);

    let distance = distance(particle.position, particle2.position);
    density += simulation_parameters.particle_mass * poly_kernel(distance, kernel_radius);
  }

  density_field[idx] = density;
  pressure_field[idx] = density_to_pressure(density);
}

fn density_to_pressure(density: f32) -> f32 {
  return (density - rest_density) * pressure_multiplier;
}

fn scale_particle_value(p: ptr<function, Particle>, scale_factor: f32) {
  (*p).position = (*p).position / scale_factor;
  (*p).velocity = (*p).velocity / scale_factor;
}


///
/// Kernels
///
fn poly_kernel(r: f32, h: f32) -> f32 {
  if r > h {
    return 0.0;
  }

  return pow(h*h-r*r,3.0) * 315.0 / (64.0*radians(180.0)*pow(h,9.0));
}


fn spiky_kernel(r: f32, h: f32) -> f32 {
  if r > h {
    return 0.0;
  }

  return pow(h-r,3.0) * 15.0 / (radians(180.0)*pow(h,6.0));
}

fn spiky_kernel_derivative(r: f32, h: f32) -> f32 {
  if r > h {
    return 0.0;
  }

  let volume = radians(180.0) * pow(h,6.0) / 15.0;

  return -3.0 * pow(h-r, 2.0) / volume;
}

fn viscosity_kernel(r: f32, h: f32) -> f32 {
  if r > h {
    return 0.0;
  }

  let a1 = -r*r*r/(2*h*h*h);
  let a2 = r*r/(h*h);
  let a3 = h/(2*r);

  return (a1+a2+a3-1.0) * 15.0 / (radians(360.0)*pow(h,3.0));
}
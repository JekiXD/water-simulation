const g: vec3<f32> = vec3<f32>(0.0, -9.8, 0.0);
const MAX_U32: u32 = 0xFFFFFFFF;

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

@group(0) @binding(0) var<storage, read_write> particles : array<Particle>;
@group(1) @binding(0) var<storage, read_write> density_field : array<f32>;
@group(1) @binding(1) var<storage, read_write> predicted_positions : array<vec3<f32>>;
@group(2) @binding(1) var<uniform> elapsed_secs: f32;
@group(2) @binding(2) var<uniform> sim: SimulationParameters;
@group(3) @binding(0) var<storage, read_write> cell_hash : array<u32>;
@group(3) @binding(1) var<storage, read_write> particle_id : array<u32>;
@group(3) @binding(2) var<storage, read_write> cell_start : array<u32>;

@compute @workgroup_size(64)
fn predict_positions(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  if(idx >= sim.particles_amount) { return; }

  var particle = particles[idx];
  //Apply gravity
  particle.velocity += elapsed_secs * sim.gravity;
  predicted_positions[idx] = particle.position + particle.velocity * elapsed_secs;

  particles[idx] = particle;
}

@compute @workgroup_size(64)
fn simulate(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  if(idx >= sim.particles_amount) { return; }

  init_rand(idx, vec4f(elapsed_secs));
  var particle = particles[idx];

  //Apply pressure and viscosity
  let accel = compute_press_and_visc(idx);
  particle.velocity += elapsed_secs * accel;

  particle.position += elapsed_secs * particle.velocity;
  compute_collisions(&particle);
  particles[idx] = particle;
}

fn compute_press_and_visc(idx: u32) -> vec3<f32>{
  var pressure_force = vec3<f32>(0.0, 0.0, 0.0);
  var viscosity_force = vec3<f32>(0.0, 0.0, 0.0);

  let p1_velocity = particles[idx].velocity;
  let p1_pos = predicted_positions[idx];
  let p1_density = density_field[idx];

  let center = get_cell_coord(p1_pos);
  //Neighbour search
  for(var x = -1; x <= 1; x++) {
    for(var y = -1; y <= 1; y++) {
      let cur_pos = center + vec3i(x, y, 0); 

      let hash = get_key_from_hash(z_order_hash(cur_pos.x, cur_pos.y));

      var i = cell_start[hash];
      for(;; i++) {
        let cell = cell_hash[i];
        if(cell != hash) { break; }

        let id2 = particle_id[i];
        if (id2 == idx) { continue; }

        let p2_pos = predicted_positions[id2];
        let p2_velocity = particles[id2].velocity;
        let p2_density = density_field[id2];

        //Calculate pressure force
        let vector = p1_pos - p2_pos;
        //let distance = max(length(vector) - 2 * sim.particle_radius, 0.0);
        let distance = length(vector);
        var dir = normalize(vector);
        if (distance == 0.0) { dir = normalize(vec3<f32>(rand() - 0.5, rand() - 0.5, 0.0)); }

        //Calculate pressure
        let average_pressure = (density_to_pressure(p1_density) + density_to_pressure(p2_density)) / 2.0;
        let press = -sim.particle_mass * average_pressure * spiky_kernel_derivative(distance, sim.spiky_kernel_radius) / p2_density;
        pressure_force += dir * press;

        //Calculate viscosity
        let velocity_dif = p2_velocity - p1_velocity;
        let visc = sim.viscosity * sim.particle_mass * velocity_dif * viscosity_kernel_laplace(distance, sim.viscosity_kernel_radius) / p2_density;
        viscosity_force += visc;
      }
    }
  }

  return (-pressure_force + viscosity_force) / p1_density;
}

fn compute_collisions(particle: ptr<function, Particle>)  {
  let dimensions = sim.bounding_box.dimensions;
  let particle_radius = sim.particle_radius;

  var p1 = sim.bounding_box.position + particle_radius;
  var p2 = sim.bounding_box.position + dimensions - particle_radius;

  var pos = (*particle).position;
  var vel = (*particle).velocity;
  
  if pos.x < p1.x || pos.x > p2.x {
    pos.x = clamp(pos.x, p1.x, p2.x);
    vel.x *= -sim.collision_damping;
  }

  if pos.y < p1.y || pos.y > p2.y {
    pos.y = clamp(pos.y, p1.y, p2.y);
    vel.y *= -sim.collision_damping;
  }

  (*particle).position = pos;
  (*particle).velocity = vel;
}

@compute @workgroup_size(64)
fn compute_density_and_pressure(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  if(idx >= sim.particles_amount) { return; }

  let p1_pos = predicted_positions[idx];

  var density = 0.0;

  let center = get_cell_coord(p1_pos);
  //Neighbour search
  for(var x = -1; x <= 1; x++) {
    for(var y = -1; y <= 1; y++) {
      let cur_pos = center + vec3i(x, y, 0); 

      let hash = get_key_from_hash(z_order_hash(cur_pos.x, cur_pos.y));

      var i = cell_start[hash];
      for(;; i++) {
        let cell = cell_hash[i];
        if(cell != hash) { break; }

        let id2 = particle_id[i];
        let p2_pos = predicted_positions[id2];

        //Calulate dnesity
        let vector = p1_pos - p2_pos;
        //let distance = max(length(vector) - 2 * sim.particle_radius, 0.0);
        let distance = length(vector);

        density += sim.particle_mass * poly_kernel(distance, sim.poly_kernel_radius);
      }
    }
  }

  density_field[idx] = density;
}

fn density_to_pressure(density: f32) -> f32 {
  return (density - sim.rest_density) * sim.pressure_multiplier;
}

fn scale_particle_value(p: ptr<function, Particle>, scale_factor: f32) {
  (*p).position = (*p).position / scale_factor;
  (*p).velocity = (*p).velocity / scale_factor;
}


///
/// Kernels
///
fn poly_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  return pow(h*h-r*r,3.0) * 315.0 / (64.0*radians(180.0)*pow(h,9.0));
}


fn spiky_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  return pow(h-r,3.0) * 15.0 / (radians(180.0)*pow(h,6.0));
}

fn spiky_kernel_derivative(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  let volume = radians(180.0) * pow(h,6.0) / 15.0;

  return 3.0 * pow(h-r, 2.0) / volume;
}

// fn viscosity_kernel(r: f32, h: f32) -> f32 {
//   if r > h {
//     return 0.0;
//   }

//   let a1 = -r*r*r/(2*h*h*h);
//   let a2 = r*r/(h*h);
//   let a3 = h/(2*r);

//   return (a1+a2+a3-1.0) * 15.0 / (radians(360.0)*pow(h,3.0));
// }

fn viscosity_kernel_laplace(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  return (h - r) * 45 / (radians(180.0) * pow(h,6.0));
}

fn z_order_hash(x: i32, y: i32) -> u32 {
    let a = u32(x) * 15823;
    let b = u32(y) * 9737333;

    return a + b;
}

fn get_cell_coord(pos: vec3f) -> vec3i {
    return vec3i(floor((pos * sim.scene_scale_factor) / sim.grid_size));
}

fn get_key_from_hash(hash: u32) -> u32 {
    return hash % sim.particles_amount;
}
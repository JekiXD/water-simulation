const scene_scale_factor: f32 = 50.0;
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
  rest_density: f32,
  pressure_multiplier: f32,
  bounding_box: BoundingBox,
  grid_size: u32
}

@group(0) @binding(0) var<storage, read_write> particles : array<Particle>;
@group(1) @binding(0) var<storage, read_write> density_field : array<f32>;
@group(1) @binding(1) var<storage, read_write> pressure_field : array<f32>;
@group(2) @binding(1) var<uniform> elapsed_secs: f32;
@group(2) @binding(2) var<uniform> sim: SimulationParameters;
@group(3) @binding(0) var<storage, read_write> cell_hash : array<u32>;
@group(3) @binding(1) var<storage, read_write> particle_id : array<u32>;
@group(3) @binding(2) var<storage, read_write> cell_start : array<u32>;

@compute @workgroup_size(64)
fn simulate(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  if(idx >= sim.particles_amount) { return; }

  init_rand(idx, vec4f(elapsed_secs));
  var particle = particles[idx];

  //Apply gravity
  particle.velocity += elapsed_secs * g * scene_scale_factor;
  //Apply pressure
  let pressure_accel = -compute_pressure_force(idx) / density_field[idx] * scene_scale_factor;
  particle.velocity += elapsed_secs * pressure_accel;
  //Apply viscosity

  particle.position += elapsed_secs * particle.velocity;
  compute_collisions(&particle);

  particles[idx] = particle;
}

fn some(idx: u32) {
  let s = pressure_field[idx];
}

fn compute_pressure_force(idx: u32) -> vec3<f32>{
  var pressure_force = vec3<f32>(0.0, 0.0, 0.0);
  let particle = particles[idx];


  let center = get_cell_coord(particle.position);;
  //Neighbour search
  for(var x = -1; x <= 1; x++) {
    for(var y = -1; y <= 1; y++) {
      let cur_pos = vec3i(center) + vec3i(x, y, 0); 

      if(cur_pos.x < 0 || cur_pos.y < 0 || cur_pos.z < 0) { continue; }
      let hash = get_key_from_hash(z_order_hash(u32(cur_pos.x), u32(cur_pos.y)));

      var i = cell_start[hash];
      for(;; i++) {
        let cell = cell_hash[i];
        if(cell != hash) { break; }

        let id2 = particle_id[i];
        if (id2 == idx) { continue; }

        let particle2 = particles[id2];

        //Calculate pressure force
        let vector = particle2.position - particle.position;
        let distance = max(length(vector) - 2 * sim.particle_radius, 0.0);
        var dir = normalize(vector);

        if (distance == 0.0) { dir = normalize(vec3<f32>(rand() - 0.5, rand() - 0.5, 0.0)); }
        let average_pressure = (pressure_field[idx] + pressure_field[id2]) / 2.0;
        let res = sim.particle_mass * average_pressure * spiky_kernel_derivative(distance, sim.spiky_kernel_radius) / density_field[id2];

        pressure_force += dir * res;
      }
    }
  }

  return pressure_force;
}

fn compute_collisions(particle: ptr<function, Particle>)  {
  let dimensions = sim.bounding_box.dimensions;
  let particle_radius = sim.particle_radius;

  var p1 = sim.bounding_box.position + particle_radius + 1.0;
  var p2 = sim.bounding_box.position + dimensions - particle_radius - 1.0;

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

  var particle = particles[idx];

  var density = 0.0;

  let center = get_cell_coord(particle.position);
  //Neighbour search
  for(var x = -1; x <= 1; x++) {
    for(var y = -1; y <= 1; y++) {
      let cur_pos = vec3i(center) + vec3i(x, y, 0); 

      if(cur_pos.x < 0 || cur_pos.y < 0 || cur_pos.z < 0) { continue; }
      let hash = get_key_from_hash(z_order_hash(u32(cur_pos.x), u32(cur_pos.y)));

      var i = cell_start[hash];
      //if(i == MAX_U32) { continue; }
      for(;; i++) {
        let cell = cell_hash[i];
        if(cell != hash) { break; }

        let id2 = particle_id[i];
        let particle2 = particles[id2];

        //Calulate dnesity
        let vector = distance(particle.position, particle2.position);
        let distance = max(length(vector) - 2 * sim.particle_radius, 0.0);

        density += sim.particle_mass * poly_kernel(distance, sim.poly_kernel_radius);
      }
    }
  }

  density_field[idx] = density;
  pressure_field[idx] = density_to_pressure(density);
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
  let r = dst / scene_scale_factor;
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

fn spiky_kernel_derivative(dst: f32, h: f32) -> f32 {
  let r = dst / scene_scale_factor;
  if r > h {
    return 0.0;
  }

  let volume = radians(180.0) * pow(h,6.0) / 15.0;

  return 3.0 * pow(h-r, 2.0) / volume;
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

fn z_order_hash(x: u32, y: u32) -> u32 {
    var z = 0u;
    for (var i = 0u; i < 16u; i++) {
        let x_bit = (x >> i) & 1u;
        let y_bit = (y >> i) & 1u;
        z |= (x_bit << (2u * i)) | (y_bit << (2u * i + 1u));
    }
    return z;
}

fn get_cell_coord(pos: vec3f) -> vec3u {
    return vec3u(pos / f32(sim.grid_size));
}

fn get_key_from_hash(hash: u32) -> u32 {
    return hash % sim.particles_amount;
}
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

struct Predicted {
    position: vec3<f32>,
    velocity: vec3<f32>
}

struct BoundingBox {
  position1: vec3<f32>,
  position2: vec3<f32>,
}

struct SimulationParameters {
  bounding_box: BoundingBox,
  gravity: vec3<f32>,
  particle_mass: f32,
  particle_radius: f32,
  particles_amount: u32,
  collision_damping: f32, 
  poly_kernel_radius: f32,
  pressure_kernel_radius: f32,
  near_pressure_kernel_radius: f32,
  viscosity_kernel_radius: f32,
  viscosity: f32,
  cohesion_coef: f32,
  curvature_cef: f32, 
  adhesion_cef: f32,
  rest_density: f32,
  pressure_multiplier: f32,
  near_pressure_multiplier: f32,
  grid_size: f32,
  scene_scale_factor: f32,
  vorticity_kernel_radius: f32,
  vorticity_inensity: f32,
  cohesion_kernel_radius: f32,
  adhesion_kernel_radius: f32,
  surface_normal_kernel_radius: f32,
  time_step: f32,
  velocity_smoothing_scale: f32
}

@group(0) @binding(0) var<storage, read_write> particles : array<Particle>;
@group(1) @binding(0) var<storage, read_write> density_field : array<f32>;
@group(1) @binding(1) var<storage, read_write> predicted : array<Predicted>;
@group(1) @binding(2) var<storage, read_write> surface_normals : array<vec3<f32>>;
@group(1) @binding(3) var<storage, read_write> near_density_field : array<f32>;
@group(1) @binding(4) var<storage, read_write> vorticity_field : array<vec3<f32>>;
@group(2) @binding(1) var<uniform> sim: SimulationParameters;
@group(3) @binding(0) var<storage, read_write> cell_hash : array<u32>;
@group(3) @binding(1) var<storage, read_write> particle_id : array<u32>;
@group(3) @binding(2) var<storage, read_write> cell_start : array<u32>;

@compute @workgroup_size(64)
fn predict_positions(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  if(idx >= sim.particles_amount) { return; }

  var particle = particles[idx];
  //Apply gravity
  predicted[idx].velocity = particle.velocity + sim.time_step * sim.gravity / sim.scene_scale_factor;
  predicted[idx].position = particle.position + sim.time_step * predicted[idx].velocity;
}

@compute @workgroup_size(64)
fn update_positions(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  if(idx >= sim.particles_amount) { return; }

  var p1 = particles[idx];
  var smoothed_vel = vec3f(0.0);
  
  //Smooth velocities
  let center = get_cell_coord(p1.position);
  for(var x = -1; x <= 1; x++) {
    for(var y = -1; y <= 1; y++) {
      let cur_pos = center + vec3i(x, y, 0); 

      let hash = get_key_from_hash(z_order_hash(cur_pos.x, cur_pos.y));

      var i = cell_start[hash];
      for(; i < sim.particles_amount; i++) {
        let cell = cell_hash[i];
        if(cell != hash) { break; }

        let id2 = particle_id[i];
        if (id2 == idx) { continue; }

        let p2 = particles[id2];
        let p2_density = density_field[id2];

        let distance = distance(p2.position, p1.position);
        let vel_vector = p2.velocity - p1.velocity;

        smoothed_vel += sim.particle_mass * vel_vector * poly_kernel(distance, sim.grid_size) / p2_density;
      }
    }
  }

  p1.velocity += sim.velocity_smoothing_scale * smoothed_vel;
  p1.position += sim.time_step * p1.velocity;
  compute_collisions(&p1);

  storageBarrier();
  particles[idx] = p1;
}


@compute @workgroup_size(64)
fn calculate_forces(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  if(idx >= sim.particles_amount) { return; }

  init_rand(idx, vec4f(sim.time_step * f32(idx)));
  var particle = particles[idx];

  //Apply forces
  let accel = compute_accel(idx);
  particle.velocity = predicted[idx].velocity +  sim.time_step * accel / sim.scene_scale_factor;
  particles[idx] = particle;
}

fn compute_accel(idx: u32) -> vec3<f32>{
  var pressure_force = vec3<f32>(0.0);
  var viscosity_force = vec3<f32>(0.0);
  var surface_tension_force = vec3<f32>(0.0);
  var adhesion_force = vec3<f32>(0.0);
  var corrective_vorticity = vec3<f32>(0.0);

  let p1_vel = predicted[idx].velocity;
  let p1_pos = predicted[idx].position;
  let p1_density = density_field[idx];
  let p1_near_density = near_density_field[idx];
  let p1_normal = surface_normals[idx];
  let p1_vorticity = vorticity_field[idx];

  surface_normals[idx] = vec3f(0.0);

  let center = get_cell_coord(p1_pos);
  //Neighbour search
  for(var x = -1; x <= 1; x++) {
    for(var y = -1; y <= 1; y++) {
      let cur_pos = center + vec3i(x, y, 0); 

      let hash = get_key_from_hash(z_order_hash(cur_pos.x, cur_pos.y));

      var i = cell_start[hash];
      for(; i < sim.particles_amount; i++) {

        let cell = cell_hash[i];
        if(cell != hash) { break; }

        let id2 = particle_id[i];
        if (id2 == idx) { continue; }

        let p2_pos = predicted[id2].position;
        let p2_vel = predicted[id2].velocity;
        let p2_density = density_field[id2];
        let p2_near_density = near_density_field[id2];
        let p2_normal = surface_normals[id2];
        let p2_vorticity = vorticity_field[id2];

        let pos_vector = p2_pos - p1_pos;
        let vel_vector = p2_vel - p1_vel;
        let distance = length(pos_vector);
        var dir = vec3f(0.0);
        if (distance == 0.0) { dir = normalize(vec3<f32>(rand() - 0.5, rand() - 0.5, 0.0)); }
        else { dir = normalize(pos_vector); }

        //Calculate pressure
        let average_pressure = (density_to_pressure(p1_density) + density_to_pressure(p2_density)) / 2.0;
        let average_near_pressure = (near_density_to_pressure(p1_near_density) + near_density_to_pressure(p2_near_density)) / 2.0;

        pressure_force += dir * sim.particle_mass * average_pressure * d1_spiky_2_kernel(distance, sim.pressure_kernel_radius) / p2_density;
        pressure_force += dir * sim.particle_mass * average_near_pressure * d1_spiky_3_kernel(distance, sim.near_pressure_kernel_radius) / p2_near_density;

        //Calculate viscosity
        let visc = sim.viscosity * sim.particle_mass * vel_vector * viscosity_kernel(distance, sim.viscosity_kernel_radius) / p2_density;
        viscosity_force += visc;

        //Calculate surface tension forces
        let cohesion_force = dir * sim.cohesion_coef * pow(sim.particle_mass, 2.0) * cohesion_kernel(distance, sim.cohesion_kernel_radius);
        let curvature_force = -sim.curvature_cef * sim.particle_mass * (p1_normal - p2_normal);
        surface_tension_force += (cohesion_force + curvature_force) * 2.0 * sim.rest_density / (p1_density + p2_density);
        adhesion_force += dir * sim.adhesion_cef * pow(sim.particle_mass, 2.0) * adhesion_kernel(distance, sim.adhesion_kernel_radius);

        //Calculate corrective vorticity
        let vort_grad = vec3f(vec2f(d1_spiky_2_kernel(distance, sim.vorticity_kernel_radius)), 0.0);
        corrective_vorticity += sim.particle_mass * length(p2_vorticity) * vort_grad / p2_density;
      }
    }
  }

  var vorticity_force = vec3f(0.0);
  if length(corrective_vorticity) != 0.0 {
    vorticity_force = sim.vorticity_inensity * cross(normalize(corrective_vorticity), p1_vorticity);
  }

  return (vorticity_force + viscosity_force + adhesion_force + surface_tension_force - pressure_force) / p1_density;
}

fn compute_collisions(particle: ptr<function, Particle>)  {
  let position2 = sim.bounding_box.position2;
  let particle_radius = sim.particle_radius;

  let p1 = sim.bounding_box.position1;
  let p2 = sim.bounding_box.position2;

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
fn compute_density(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  if(idx >= sim.particles_amount) { return; }

  let p1_pos = predicted[idx].position;

  var density = 0.0;
  var near_density = 0.0;

  let center = get_cell_coord(p1_pos);
  //Neighbour search
  for(var x = -1; x <= 1; x++) {
    for(var y = -1; y <= 1; y++) {
      let cur_pos = center + vec3i(x, y, 0); 

      let hash = get_key_from_hash(z_order_hash(cur_pos.x, cur_pos.y));

      var i = cell_start[hash];
      for(; i < sim.particles_amount; i++) {
        let cell = cell_hash[i];
        if(cell != hash) { break; }

        let id2 = particle_id[i];
        let p2_pos = predicted[id2].position;

        //Calulate density
        let distance = length(p2_pos - p1_pos);

        density += sim.particle_mass * spiky_2_kernel(distance, sim.poly_kernel_radius);
        near_density += sim.particle_mass * spiky_3_kernel(distance, sim.poly_kernel_radius);
      }
    }
  }

  density_field[idx] = density;
  near_density_field[idx] = near_density;
}

@compute @workgroup_size(64)
fn compute_intermediate_values(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  if(idx >= sim.particles_amount) { return; }

  let p1_pos = predicted[idx].position;
  let p1_vel = predicted[idx].velocity;

  var surface_normal = vec3<f32>(0.0);
  var vorticity = vec3<f32>(0.0);

  let center = get_cell_coord(p1_pos);
  //Neighbour search
  for(var x = -1; x <= 1; x++) {
    for(var y = -1; y <= 1; y++) {
      let cur_pos = center + vec3i(x, y, 0); 

      let hash = get_key_from_hash(z_order_hash(cur_pos.x, cur_pos.y));

      var i = cell_start[hash];
      for(; i < sim.particles_amount; i++) {
        let cell = cell_hash[i];
        if(cell != hash) { break; }

        let id2 = particle_id[i];
        if (id2 == idx) { continue; }

        let p2_pos = predicted[id2].position;
        let p2_vel = predicted[id2].velocity;

        let pos_vector = p2_pos - p1_pos;
        let vel_vector = p2_vel - p1_vel;

        let distance = length(pos_vector);
        var dir = vec3f(0.0);
        if distance != 0.0 { dir = normalize(pos_vector); }

        //Calulate surface normals
        surface_normal += dir * sim.particle_mass * d1_poly_kernel(distance, sim.surface_normal_kernel_radius) / density_field[id2];

        //Calculate vorticity
        let vort_grad = vec3f(vec2f(d1_spiky_2_kernel(distance, sim.vorticity_kernel_radius)), 0.0);
        vorticity += -sim.particle_mass * cross(vel_vector, vort_grad) / density_field[id2];
      }
    }
  }
 
  surface_normals[idx] = surface_normal;
  vorticity_field[idx] = vorticity;
}

fn density_to_pressure(density: f32) -> f32 {
  return (density - sim.rest_density) * sim.pressure_multiplier;
}

fn near_density_to_pressure(near_density: f32) -> f32 {
  return near_density * sim.near_pressure_multiplier;
}

///
/// Kernels
///
fn spiky_2_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  let volume = radians(180.0)*h*h/6;

  return pow(1-r/h,2.0)/ volume;
}

fn spiky_3_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  let volume = radians(180.0)*h*h/10.0;

  return pow(1-r/h,3.0) / volume;
}

fn d1_spiky_2_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  let alpha = 12.0 / (radians(180.0)*pow(h, 4.0));

  return (h-r) * alpha;
}

fn d1_spiky_3_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  let alpha = 30.0 / (radians(180.0)*pow(h, 5.0));

  return pow(h-r,2.0) * alpha;
}


fn poly_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  let volume = radians(180.0) * pow(h, 8.0) / 4.0;

  return pow(h*h-r*r,3.0) / volume;
}

fn d1_poly_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  let volume = radians(180.0) * pow(h, 8.0) / 4.0;

  return 6 * r * pow(h*h-r*r,2.0) / volume;
}

fn viscosity_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  let volume = radians(180.0) * pow(h, 6.0) / 30.0;

  return (h-r) / volume;
}

fn cohesion_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;

  let k = 32 / (radians(180.0) * pow(h, 9.0));
  let fun = pow(h-r,3.0) * pow(r, 3.0);

  if (2*r > h && r <= h) {
    return k * fun;
  } 
  else if (r > 0 && 2*r <= h) {
    return k * (2*fun - pow(h, 6.0) / 64.0);
  }

  return 0.0;
}

fn adhesion_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;

  let k = 0.007 / pow(h, 3.25);
  let e = 0.001;

  if 2*r > h+e && r <= h-e {
    return k * pow(-4.0*r*r/h + 6*r - 2*h, 0.25);
  } 

  return 0.0;
}


const B: array<u32, 4> = array<u32, 4>(0x55555555, 0x33333333, 0x0F0F0F0F, 0x00FF00FF);
const S: array<u32, 4> = array<u32, 4>(1, 2, 4, 8);

fn z_order_hash(x_in: i32, y_in: i32) -> u32 {
    var x = u32(x_in);
    var y = u32(y_in);

    x = (x | (x << S[3])) & B[3];
    x = (x | (x << S[2])) & B[2];
    x = (x | (x << S[1])) & B[1];
    x = (x | (x << S[0])) & B[0];

    y = (y | (y << S[3])) & B[3];
    y = (y | (y << S[2])) & B[2];
    y = (y | (y << S[1])) & B[1];
    y = (y | (y << S[0])) & B[0];

    return x | (y << 1);
}

fn get_cell_coord(pos: vec3f) -> vec3i {
    return vec3i((pos * sim.scene_scale_factor) / sim.grid_size);
}

fn get_key_from_hash(hash: u32) -> u32 {
    return hash % sim.particles_amount;
}





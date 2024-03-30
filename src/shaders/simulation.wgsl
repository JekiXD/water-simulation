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
  pressure_kernel_radius: f32,
  near_pressure_kernel_radius: f32,
  viscosity_kernel_radius: f32,
  viscosity: f32,
  surface_tension: f32,
  cohesion_coef: f32,
  curvature_cef: f32, 
  adhesion_cef: f32,
  rest_density: f32,
  pressure_multiplier: f32,
  near_pressure_multiplier: f32,
  bounding_box: BoundingBox,
  grid_size: f32,
  scene_scale_factor: f32,
  gravity: vec3<f32>
}

@group(0) @binding(0) var<storage, read_write> particles : array<Particle>;
@group(1) @binding(0) var<storage, read_write> density_field : array<f32>;
@group(1) @binding(1) var<storage, read_write> predicted_positions : array<vec3<f32>>;
@group(1) @binding(2) var<storage, read_write> surface_normals : array<vec3<f32>>;
@group(1) @binding(3) var<storage, read_write> near_density_field : array<f32>;
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
  particle.velocity += elapsed_secs * sim.gravity / sim.scene_scale_factor;
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
  let accel = compute_accel(idx);
  particle.velocity += elapsed_secs * accel / sim.scene_scale_factor;

  particle.position += elapsed_secs * particle.velocity;
  compute_collisions(&particle);
  particles[idx] = particle;
}

fn compute_accel(idx: u32) -> vec3<f32>{
  var pressure_force = vec3<f32>(0.0, 0.0, 0.0);
  var viscosity_force = vec3<f32>(0.0, 0.0, 0.0);

  var surface_normal = vec3<f32>(0.0, 0.0, 0.0);
  var surface_curvature = 0.0;

  var surface_tension = vec3<f32>(0.0, 0.0, 0.0);

  let p1_velocity = particles[idx].velocity;
  let p1_pos = predicted_positions[idx];
  let p1_density = density_field[idx];
  let p1_near_density = near_density_field[idx];
  let p1_normal = surface_normals[idx];

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

        let p2_pos = predicted_positions[id2];
        let p2_velocity = particles[id2].velocity;
        let p2_density = density_field[id2];
        let p2_near_density = near_density_field[id2];
        let p2_normal = surface_normals[id2];

        //Calculate pressure force
        let vector = p2_pos - p1_pos;
        //let distance = max(length(vector) - 2 * sim.particle_radius, 0.0);
        let distance = length(vector);
        var dir = normalize(vector);
        if (distance == 0.0) { dir = normalize(vec3<f32>(rand() - 0.5, rand() - 0.5, 0.0)); }




        //Calculate pressure
        let average_pressure = (density_to_pressure(p1_density) + density_to_pressure(p2_density)) / 2.0;
        let average_near_pressure = (near_density_to_pressure(p1_near_density) + near_density_to_pressure(p2_near_density)) / 2.0;

        pressure_force += dir * sim.particle_mass * average_pressure * dx_density_2_kernel(distance, sim.pressure_kernel_radius) / p2_density;
        pressure_force += dir * sim.particle_mass * average_near_pressure * dx_density_3_kernel(distance, sim.near_pressure_kernel_radius) / p2_near_density;

        //Calculate viscosity
        let velocity_dif = p2_velocity - p1_velocity;
        let visc = sim.viscosity * sim.particle_mass * velocity_dif * viscosity_kernel_laplace(distance, sim.viscosity_kernel_radius) / p2_density;
        viscosity_force += visc;

        //Surface tension
        // surface_normal += sim.cohesion_coef * dir * sim.particle_mass * poly_d1_kernel(distance, 0.3) / p2_density;
        // surface_curvature += sim.curvature_cef * sim.particle_mass * poly_laplace_kernel(distance, 0.3) / p2_density;

        let cohesion_force = -dir * sim.cohesion_coef * pow(sim.particle_mass, 2.0) * cohesion_kernel(distance, 1.0);
        let curvature_force = -sim.curvature_cef * sim.particle_mass * (p1_normal - p2_normal);
        let adhesion_force = -dir * sim.adhesion_cef * pow(sim.particle_mass, 2.0) * adhesion_kernel(distance, 1.0);
        surface_tension += (cohesion_force + curvature_force + adhesion_force) * 2.0 * sim.rest_density / (p1_density + p2_density);
      }
    }
  }

  // if(length(surface_normal) > 0.01) {
  //   surface_force = -sim.surface_tension * surface_curvature * normalize(surface_normal);
  // }

  return (viscosity_force + surface_tension - pressure_force) / p1_density;
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
fn compute_density(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  if(idx >= sim.particles_amount) { return; }

  let p1_pos = predicted_positions[idx];

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
        let p2_pos = predicted_positions[id2];

        //Calulate density
        let vector = p2_pos - p1_pos;
        //let distance = max(length(vector) - 2 * sim.particle_radius, 0.0);
        let distance = length(vector);

        density += sim.particle_mass * density_2_kernel(distance, sim.poly_kernel_radius);
        near_density += sim.particle_mass * density_3_kernel(distance, sim.poly_kernel_radius);
      }
    }
  }

  density_field[idx] = density;
  near_density_field[idx] = near_density;
}

@compute @workgroup_size(64)
fn compute_surface_normals(@builtin(global_invocation_id) global_invocation_id : vec3u) {
  let idx = global_invocation_id.x;
  if(idx >= sim.particles_amount) { return; }

  let p1_pos = predicted_positions[idx];

  var surface_normal = vec3<f32>(0.0, 0.0, 0.0);

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

        let p2_pos = predicted_positions[id2];

        //Calulate dnesity
        let vector = p2_pos - p1_pos;
        //let distance = max(length(vector) - 2 * sim.particle_radius, 0.0);
        let distance = length(vector);
        var dir = normalize(vector);

        if (distance == 0.0) { dir = normalize(vec3<f32>(rand() - 0.5, rand() - 0.5, 0.0)); }

        let normal_scale = sim.surface_tension * sim.particle_mass * poly_d1_kernel(distance, 1.0) / density_field[id2];
        surface_normal += dir * normal_scale;
      }
    }
  }
 
  surface_normals[idx] = surface_normal;
}

fn density_to_pressure(density: f32) -> f32 {
  return (density - sim.rest_density) * sim.pressure_multiplier;
}

fn near_density_to_pressure(near_density: f32) -> f32 {
  return near_density * sim.near_pressure_multiplier;
}

fn scale_particle_value(p: ptr<function, Particle>, scale_factor: f32) {
  (*p).position = (*p).position / scale_factor;
  (*p).velocity = (*p).velocity / scale_factor;
}


///
/// Kernels
///
fn density_2_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  return pow(1-r/h,2.0);
}

fn density_3_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  return pow(1-r/h,3.0);
}

fn dx_density_2_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  return 2*(h-r)/(h*h);
}

fn dx_density_3_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  return 3*pow(h-r,2.0)/(h*h*h);
}






fn poly_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  return pow(h*h-r*r,3.0) * 315.0 / (64.0*radians(180.0)*pow(h,9.0));
}

fn poly_d1_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  return r * pow(h*h-r*r,2.0) * 315.0 * 3 / (32.0*radians(180.0)*pow(h,9.0));
}

fn poly_laplace_kernel(dst: f32, h: f32) -> f32 {
  let r = dst * sim.scene_scale_factor;
  if r > h {
    return 0.0;
  }

  let k = -6.0 * 315.0 / (64.0 * radians(180.0) * pow(h,9.0));
  return k * (pow(h, 4.0) - 6.0*h*h*r*r + 5*pow(r,4.0));
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

  if (2*r > h && r <= h) {
    return k * pow(-4.0*r*r/h + 6*r - 2*h, 0.25);
  } 

  return 0.0;
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
    return vec3i((pos * sim.scene_scale_factor) / sim.grid_size);
}

fn get_key_from_hash(hash: u32) -> u32 {
    return hash % sim.particles_amount;
}





const MAX_U32: u32 = 0xFFFFFFFF;

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

struct Predicted {
    position: vec3<f32>,
    velocity: vec3<f32>
}

@group(0) @binding(0) var<storage, read_write> particles : array<Particle>;
@group(1) @binding(1) var<storage, read_write> predicted : array<Predicted>;
@group(2) @binding(1) var<uniform> sim: SimulationParameters;
@group(3) @binding(0) var<storage, read_write> cell_hash : array<u32>;
@group(3) @binding(1) var<storage, read_write> particle_id : array<u32>;
@group(3) @binding(2) var<storage, read_write> cell_start : array<u32>;

@compute @workgroup_size(64)
fn calcHash(@builtin(global_invocation_id) global_invocation_id : vec3u) {
    let idx = global_invocation_id.x;
    if(idx >= sim.particles_amount) { return; }

    let pos = get_cell_coord(predicted[idx].position);
    let hash = get_key_from_hash(z_order_hash(pos.x, pos.y));
    cell_hash[idx] = hash;
    particle_id[idx] = idx;
    cell_start[idx] = MAX_U32; 
}

@compute @workgroup_size(64)
fn findCellStart(@builtin(global_invocation_id) global_invocation_id : vec3u) {
    let idx = global_invocation_id.x;
    if(idx >= sim.particles_amount) { return; }

    let key = cell_hash[idx];
    var key_prev = u32(0);
    if( idx == 0 ) {
        key_prev = MAX_U32;
    } else {
        key_prev = cell_hash[idx - 1];
    }

    if( key != key_prev ) {
        cell_start[key] = idx;
    }
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
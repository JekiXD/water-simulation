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
@group(1) @binding(0) var<storage, read_write> cell_hash : array<u32>;
@group(1) @binding(1) var<storage, read_write> particle_id : array<u32>;
@group(1) @binding(2) var<storage, read_write> cell_start : array<u32>;
@group(2) @binding(2) var<uniform> sim: SimulationParameters;

@compute @workgroup_size(64)
fn calcHash(@builtin(global_invocation_id) global_invocation_id : vec3u) {
    let idx = global_invocation_id.x;
    if(idx >= sim.particles_amount) { return; }

    let pos = get_cell_coord(particles[idx].position);
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
    if( key == 0 ) {
        key_prev = MAX_U32;
    } else {
        key_prev = cell_hash[idx - 1];
    }

    if( key != key_prev ) {
        cell_start[key] = idx;
    }
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
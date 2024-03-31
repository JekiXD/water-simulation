use cgmath::Vector3;
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct SimulationParameters {
    pub particle_mass: f32,
    pub particle_radius: f32,
    pub particles_amount: u32,
    pub collision_damping: f32, 
    pub poly_kernel_radius: f32,
    pub pressure_kernel_radius: f32,
    pub near_pressure_kernel_radius: f32,
    pub viscosity_kernel_radius: f32,
    pub viscosity: f32,
    pub surface_tension: f32,
    pub cohesion_coef: f32,
    pub curvature_cef: f32, 
    pub adhesion_cef: f32,
    pub rest_density: f32,
    pub pressure_multiplier: f32,
    pub near_pressure_multiplier: f32,
    //_padding: [u32; 1],
    pub bounding_box: BoundingBoxUniform,
    pub grid_size: f32,
    pub scene_scale_factor: f32,
    _padding2: [u32; 2],
    pub gravity: [f32; 3],
    _padding3: u32
}

impl Default for SimulationParameters {
    fn default() -> Self {
        let width = 1600.0f32;
        let height = 900.0f32;
        let diagonal = (width * width + height * height).sqrt();
        let scene_scale_factor = 50.0 / diagonal;


        let particle_mass = 1.0;
        let particle_radius = 3.0;
        let particles_amount = 10000;
        let collision_damping = 0.9;
        let poly_kernel_radius = 1.5;
        let pressure_kernel_radius = 1.3;
        let near_pressure_kernel_radius = 0.83;
        let viscosity_kernel_radius = 0.5;
        let viscosity = 0.1;
        let surface_tension = 1.0;
        let cohesion_coef = 1.0;
        let curvature_cef = 1.0; 
        let adhesion_cef = 1.0;
        let rest_density = 18.0;
        let pressure_multiplier = 1000.0;
        let near_pressure_multiplier = 300.0;
        let bounding_box = BoundingBoxUniform::new(Vector3::new(0.0, 0.0, 0.0),  Vector3::new(width, height, 1.0));
        let grid_size = 2.0;
        let gravity = [0.0, -10.0, 0.0];

        SimulationParameters {
            particle_mass,
            particle_radius,
            particles_amount,
            collision_damping,
            poly_kernel_radius,
            pressure_kernel_radius,
            near_pressure_kernel_radius,
            viscosity_kernel_radius,
            viscosity,
            surface_tension,
            cohesion_coef,
            curvature_cef,
            adhesion_cef,
            rest_density,
            pressure_multiplier,
            near_pressure_multiplier,
            bounding_box,
            grid_size,
            scene_scale_factor,
            gravity,
            _padding3: 0,
            _padding2: [0,0],
            //_padding: [0]
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct BoundingBoxUniform{
    pub position: [f32; 3],
    _padding: u32,
    pub dimensions: [f32; 3],
    _padding1: u32,
}

impl BoundingBoxUniform {
    pub fn new(position: Vector3<f32>, dimensions: Vector3<f32>) -> Self {
        BoundingBoxUniform {
            position: position.into(),
            dimensions: dimensions.into(),
            ..Default::default()
        }
    }
}
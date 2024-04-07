use cgmath::Vector3;
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct SimulationParameters {
    pub bounding_box: BoundingBoxUniform,
    pub gravity: [f32; 3],
    //0
    pub particle_mass: f32,
    pub particle_radius: f32,
    pub particles_amount: u32,
    pub collision_damping: f32, 
    //4
    pub poly_kernel_radius: f32,
    pub pressure_kernel_radius: f32,
    pub near_pressure_kernel_radius: f32,
    pub viscosity_kernel_radius: f32,
    //8
    pub viscosity: f32,
    pub cohesion_coef: f32,
    pub curvature_cef: f32, 
    pub adhesion_cef: f32,
    //12
    pub rest_density: f32,
    pub pressure_multiplier: f32,
    pub near_pressure_multiplier: f32,
    pub grid_size: f32,
    //16
    pub scene_scale_factor: f32,
    pub vorticity_kernel_radius: f32,
    pub vorticity_inensity: f32,
    pub cohesion_kernel_radius: f32,
    //20
    pub adhesion_kernel_radius: f32,
    pub surface_normal_kernel_radius: f32,
    pub time_scale: f32,
    pub velocity_smoothing_scale: f32,
    _padding: [f32; 1]
}

impl Default for SimulationParameters {
    fn default() -> Self {
        let width = 1600.0f32;
        let height = 900.0f32;
        let diagonal = (width * width + height * height).sqrt();
        let scene_scale_factor = 50.0 / diagonal;

        let particle_mass = 1.0;
        let particle_radius = 1.5;
        let particles_amount = 16384;
        let collision_damping = 0.9;
        let viscosity = 0.05;
        let cohesion_coef = 1.0;
        let curvature_cef = 1.0; 
        let adhesion_cef = 1.0;
        let rest_density = 35.0;
        let pressure_multiplier = 1300.0;
        let near_pressure_multiplier = 110.0;
        let bounding_box = BoundingBoxUniform::new(Vector3::new(0.0, 0.0, 0.0),  Vector3::new(width, height, 1.0));
        let grid_size = 0.6;
        let gravity = [0.0, -15.0, 0.0];
        let vorticity_inensity = 0.5;
        let time_scale = 1.0 / 120.0;
        let velocity_smoothing_scale = 0.035;

        let poly_kernel_radius = grid_size;
        let pressure_kernel_radius = grid_size;
        let near_pressure_kernel_radius = grid_size;
        let viscosity_kernel_radius = grid_size;
        let vorticity_kernel_radius = grid_size;
        let cohesion_kernel_radius = grid_size;
        let adhesion_kernel_radius = grid_size;
        let surface_normal_kernel_radius = grid_size;

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
            vorticity_kernel_radius,
            vorticity_inensity,
            cohesion_kernel_radius,
            adhesion_kernel_radius,
            surface_normal_kernel_radius,
            time_scale,
            velocity_smoothing_scale,
            _padding: Default::default(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct BoundingBoxUniform{
    pub position1: [f32; 3],
    _padding: u32,
    pub position2: [f32; 3],
    _padding1: u32,
}

impl BoundingBoxUniform {
    pub fn new(position: Vector3<f32>, dimensions: Vector3<f32>) -> Self {
        BoundingBoxUniform {
            position1: position.into(),
            position2: dimensions.into(),
            ..Default::default()
        }
    }
}
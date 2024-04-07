use std::sync::Arc;
use winit::window::Window;
use winit::event:: WindowEvent;

use crate::particle::NeighbourSearchSortState;
use crate::particle::ParticlesState;
use crate::particle::{Particle, ParticleRaw};
use crate::uniforms::parameters::SIMULATION_PARAMETERS;
use crate::uniforms::UniformState;
use crate::vertex::*;
use crate::geometry;

pub struct State {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: Arc<Window>,
    uniform_state: UniformState,
    render_pipeline: wgpu::RenderPipeline,
    particles_state: ParticlesState,
    circle_mesh_buffer: geometry::MeshBuffer,
    forces_pipeline: wgpu::ComputePipeline,
    d_pipeline: wgpu::ComputePipeline,
    sort_state: NeighbourSearchSortState,
    calc_hash_pipeline: wgpu::ComputePipeline,
    cell_start_pipeline: wgpu::ComputePipeline,
    pre_pos_pipeline: wgpu::ComputePipeline,
    sn_pipeline: wgpu::ComputePipeline,
    move_pipeline: wgpu::ComputePipeline
}

impl State {
    pub async fn new(window: Arc<Window>) -> Self {
        //
        // Start of window surface configuration
        //
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::VERTEX_WRITABLE_STORAGE,
                required_limits: wgpu::Limits::default(),
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps.formats.iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        //
        // End of window surface configuration
        //

        let particles_state = ParticlesState::new(&device, &config);
        let circle_mesh_buffer = Particle::circle_mesh().into_buffer(&device);
        let uniform_state = UniformState::new(&device, &size);

        let subgroup_size = wgpu_sort::utils::guess_workgroup_size(&device, &queue).await.unwrap();
        let sort_state = NeighbourSearchSortState::new(&device, &queue, subgroup_size);

        //
        // Render pipeline
        //
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &uniform_state.bind_group_layout,
                    &particles_state.fields_bind_group_layout
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    VertexRaw::desc(),
                    ParticleRaw::desc()
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1, 
                mask: !0, 
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        //
        // Pipelines for simulation
        //
        let simulate_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Simulation Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/simulation.wgsl").into()),
        });

        let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor 
            { 
                label: Some("Compute pipeline layout"), 
                bind_group_layouts: &[
                    &particles_state.particles_bind_group_layout,
                    &particles_state.fields_bind_group_layout,
                    &uniform_state.bind_group_layout,
                    &sort_state.grid_state.bind_group_layout,
                ], 
                push_constant_ranges: &[] 
            }
        );

        let move_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Update particle's positions pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &simulate_shader,
            entry_point: "update_positions"
        });

        let forces_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Simulation pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &simulate_shader,
            entry_point: "calculate_forces"
        });

        let d_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Density pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &simulate_shader,
            entry_point: "compute_density"
        });

        let pre_pos_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Predict positions"),
            layout: Some(&compute_pipeline_layout),
            module: &simulate_shader,
            entry_point: "predict_positions"
        });

        let sn_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Intermediate values"),
            layout: Some(&compute_pipeline_layout),
            module: &simulate_shader,
            entry_point: "compute_intermediate_values"
        });

        //
        // Pipeline to prepare resources for the sort
        //
        let sort_prep_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor { 
            label: Some("Sort preperation shader"), 
            source:  wgpu::ShaderSource::Wgsl(include_str!("shaders/sort_prep.wgsl").into())
        });

        let sort_prep_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { 
            label: Some("Sort preperation pipeline layout"), 
            bind_group_layouts: &[
                &particles_state.particles_bind_group_layout,
                &particles_state.fields_bind_group_layout,
                &uniform_state.bind_group_layout,
                &sort_state.grid_state.bind_group_layout,
            ], 
            push_constant_ranges: &[]
        });

        let calc_hash_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Sort preperation pipeline"),
            layout: Some(&sort_prep_pipeline_layout),
            module: &sort_prep_shader,
            entry_point: "calcHash"
        });

        let cell_start_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor{
            label: Some("Sort preperation pipeline"),
            layout: Some(&sort_prep_pipeline_layout),
            module: &sort_prep_shader,
            entry_point: "findCellStart"
        });

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            uniform_state,
            render_pipeline,
            particles_state,
            circle_mesh_buffer,
            forces_pipeline,
            d_pipeline,
            sort_state,
            calc_hash_pipeline,
            cell_start_pipeline,
            pre_pos_pipeline,
            sn_pipeline,
            move_pipeline
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        false
    }

    pub fn update(&mut self) {
        self.uniform_state.update(&self.queue);
        self.queue.write_buffer(&self.uniform_state.simulation_parameters.buffer, 0, bytemuck::cast_slice(&[*SIMULATION_PARAMETERS.lock().unwrap()]));
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        let particles_amount = SIMULATION_PARAMETERS.lock().unwrap().particles_amount;

        {
            //Predict particle's positions
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.pre_pos_pipeline);
            self.set_compute_bind_groups(&mut compute_pass);
            compute_pass.dispatch_workgroups(particles_amount.div_ceil(64), 1, 1);
        }

        {
            //Prepare data for the sort
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.calc_hash_pipeline);
            self.set_compute_bind_groups(&mut compute_pass);
            compute_pass.dispatch_workgroups(particles_amount.div_ceil(64), 1, 1);
        }

        {
            //Sort for neighbour search
            self.sort_state.sort(&mut encoder, &self.queue);
        }

        {
            //Find start for each cell in the grid
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.cell_start_pipeline);
            self.set_compute_bind_groups(&mut compute_pass);
            compute_pass.dispatch_workgroups(particles_amount.div_ceil(64), 1, 1);
        }

        {
            //Precompute densities for each particle
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.d_pipeline);
            self.set_compute_bind_groups(&mut compute_pass);
            compute_pass.dispatch_workgroups(particles_amount.div_ceil(64), 1, 1);
        }

        {
            //Find surface normals and vorticity
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.sn_pipeline);
            self.set_compute_bind_groups(&mut compute_pass);
            compute_pass.dispatch_workgroups(particles_amount.div_ceil(64), 1, 1);
        }

        {
            //Calculate forces
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.forces_pipeline);
            self.set_compute_bind_groups(&mut compute_pass);
            compute_pass.dispatch_workgroups(particles_amount.div_ceil(64), 1, 1);
        }

        {
            //Smooth velocities and update positions
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            compute_pass.set_pipeline(&self.move_pipeline);
            self.set_compute_bind_groups(&mut compute_pass);
            compute_pass.dispatch_workgroups(particles_amount.div_ceil(64), 1, 1);
        }

        
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                //location(0)
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_state.bind_group, &[]);
            render_pass.set_bind_group(1, &self.particles_state.fields_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.circle_mesh_buffer.vertices.slice(..));
            render_pass.set_vertex_buffer(1, self.particles_state.particles_buffer.slice(..));
            render_pass.set_index_buffer(self.circle_mesh_buffer.indices.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.draw_indexed(0..self.circle_mesh_buffer.num_indices, 0, 0..(self.particles_state.particles.len() as u32));
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn set_compute_bind_groups<'cp>(&'cp self, compute_pass: &mut wgpu::ComputePass<'cp>) {
        compute_pass.set_bind_group(0, &self.particles_state.particles_bind_group, &[]);
        compute_pass.set_bind_group(1, &self.particles_state.fields_bind_group, &[]);
        compute_pass.set_bind_group(2, &self.uniform_state.bind_group, &[]);
        compute_pass.set_bind_group(3, &self.sort_state.grid_state.bind_group, &[]);
    }
}
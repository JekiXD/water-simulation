use log::debug;

use wgpu::util::RenderEncoder;
use wgpu::Label;
use winit::dpi::PhysicalPosition;
use winit::keyboard::PhysicalKey;
use winit::keyboard::KeyCode;
use winit::window::Window;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use wgpu::util::DeviceExt;

use cgmath::prelude::*;
use crate::particle::{Particle, ParticleBuffer, ParticleList, ParticleRaw};
use crate::vertex::*;
use crate::geometry;
use crate::camera::*;

pub struct State<'window> {
    pub surface: wgpu::Surface<'window>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub window: &'window Window,
    camera_state: CameraState,
    render_pipeline: wgpu::RenderPipeline,
    particles_buffer: ParticleBuffer,
    particles_bind_group: wgpu::BindGroup,
    circle_mesh_buffer: geometry::MeshBuffer,
    screen_rect_pipeline: wgpu::RenderPipeline,
    scren_square_buffer: wgpu::Buffer
}

impl<'window> State<'window> {
    pub async fn new(window: &'window Window) -> Self {
        //
        // Start of window surface configuration
        //
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        let surface = instance.create_surface(window).unwrap();

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

        let camera = Camera {
            eye: (config.width as f32 / 2.0, config.height as f32 / 2.0, 300.0).into(),
            direction: (0.0, 0.0, -1.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 90.0,
            znear: 0.1,
            zfar: 401.0,
        };

        let camera_state = CameraState::new(camera, &device);

        let particles_buffer = ParticleList::init(10000, &config).into_buffer(&device);
        let circle_mesh_buffer = Particle::circle_mesh().into_buffer(&device);

        let particles_bind_group_layout = &device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor { 
                label: None, 
                entries: &[
                    wgpu::BindGroupLayoutEntry{
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer { 
                            ty: wgpu::BufferBindingType::Storage { read_only: false }, 
                            has_dynamic_offset: false, 
                            min_binding_size: None 
                        },
                        count: None
                    }
                ] 
            }
        );

        let particles_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &particles_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: particles_buffer.buffer.as_entire_binding()
                    }
                ],
                label: None
            },
        );


        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    //&particles_bind_group_layout,
                    &camera_state.camera_bind_group_layout,
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
                cull_mode: None,//Some(wgpu::Face::Back),
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

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("tmp.wgsl").into()),
        });

        let screen_rect_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    VertexRaw::desc(),
                    //ParticleRaw::desc()
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
                cull_mode: None,//Some(wgpu::Face::Back),
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
        
        let scren_square_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Particles"),
                contents: bytemuck::cast_slice(&SCREEN_SQUARE_VERTICES),
                usage: wgpu::BufferUsages::VERTEX
            }
        );

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            camera_state,
            render_pipeline,
            particles_buffer,
            particles_bind_group,
            circle_mesh_buffer,
            screen_rect_pipeline,
            scren_square_buffer
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
        
        match event {
            WindowEvent::CursorMoved { device_id: _, position } => {
                let PhysicalPosition {x, y} = position;

                true
            },
            _ => false
        }
    }

    pub fn update(&mut self) {
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

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

            render_pass.set_pipeline(&self.screen_rect_pipeline);
            render_pass.set_bind_group(0, &self.camera_state.camera_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.scren_square_buffer.slice(..));
            render_pass.draw(0..6, 0..2);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                //location(0)
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,//wgpu::LoadOp::Clear(wgpu::Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            //render_pass.set_bind_group(0, &self.particles_bind_group, &[]);
            render_pass.set_bind_group(0, &self.camera_state.camera_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.circle_mesh_buffer.vertices.slice(..));
            render_pass.set_vertex_buffer(1, self.particles_buffer.buffer.slice(..));
            render_pass.set_index_buffer(self.circle_mesh_buffer.indices.slice(..), wgpu::IndexFormat::Uint16);

            render_pass.draw_indexed(0..self.circle_mesh_buffer.num_indices, 0, 0..self.particles_buffer.num_particles);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
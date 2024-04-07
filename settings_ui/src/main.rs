use std::{io::Write, net::TcpStream};

use eframe::egui;
use log::info;
use settings::settings;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_resizable(false)
            .with_inner_size([360.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native("Settings", native_options, Box::new(|cc| Box::new(SettingsUI::new(cc))))?;

    Ok(())
}

struct SettingsUI {
    settings: settings::SimulationParameters,
    start_bound: settings::BoundingBoxUniform,
    stream: TcpStream,
    last_instant: std::time::Instant
}

impl SettingsUI {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let stream = TcpStream::connect("127.0.0.1:12345").unwrap();
        let settings: settings::SimulationParameters =  Default::default();

        SettingsUI { 
            settings,
            stream: stream,
            last_instant: std::time::Instant::now(),
            start_bound: settings.bounding_box
        }
    }
}

impl SettingsUI {
    fn ui(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("General parameters")
        .num_columns(2)
        .spacing([50.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            self.general_params(ui);

            ui.label(egui::RichText::new("Kernels").strong());
            ui.end_row();

            self.kernels(ui);
        });
    }

    fn general_params(&mut self,  ui: &mut egui::Ui) {
        ui.label("Scene scale:");
        ui.add(egui::DragValue::new(&mut self.settings.scene_scale_factor).speed(0.001).clamp_range(0.0..=1.0));
        ui.end_row();

        ui.label("Time scale:");
        ui.add(egui::DragValue::new(&mut self.settings.time_scale).speed(0.0001).clamp_range(0.0..=1.0));
        ui.end_row();

        ui.label("Particle's mass:");
        ui.add(egui::DragValue::new(&mut self.settings.particle_mass).speed(0.1).clamp_range(0.0..=100.0));
        ui.end_row();

        ui.label("Particles's (draw) radius:");
        ui.add(egui::DragValue::new(&mut self.settings.particle_radius).speed(0.1).clamp_range(0.5..=100.0));
        ui.end_row();

        ui.label("Collision damping:");
        ui.add(egui::DragValue::new(&mut self.settings.collision_damping).speed(0.01).clamp_range(0.0..=1.0));
        ui.end_row();

        ui.label("Viscosity:");
        ui.add(egui::DragValue::new(&mut self.settings.viscosity).speed(0.01).clamp_range(0.0..=100.0));
        ui.end_row();

        ui.label("Cohesion coef.:");
        ui.add(egui::DragValue::new(&mut self.settings.cohesion_coef).speed(0.1).clamp_range(0.0..=50000.0));
        ui.end_row();

        ui.label("Curvature coef.:");
        ui.add(egui::DragValue::new(&mut self.settings.curvature_cef).speed(0.1).clamp_range(0.0..=50000.0));
        ui.end_row();

        ui.label("Adhesion coef.:");
        ui.add(egui::DragValue::new(&mut self.settings.adhesion_cef).speed(0.1).clamp_range(0.0..=50000.0));
        ui.end_row();

        ui.label("Rest density:");
        ui.add(egui::DragValue::new(&mut self.settings.rest_density).speed(0.1).clamp_range(0.0..=1000.0));
        ui.end_row();

        ui.label("Intensity of vorticity:");
        ui.add(egui::DragValue::new(&mut self.settings.vorticity_inensity).speed(0.01).clamp_range(0.0..= 1.0));
        ui.end_row();

        ui.label("Pressure multiplier:");
        ui.add(egui::DragValue::new(&mut self.settings.pressure_multiplier).speed(0.1).clamp_range(0.0..=10000.0));
        ui.end_row();

        ui.label("Near pressure multiplier:");
        ui.add(egui::DragValue::new(&mut self.settings.near_pressure_multiplier).speed(0.1).clamp_range(0.0..=10000.0));
        ui.end_row();

        ui.label("Grid size:");
        ui.add(egui::DragValue::new(&mut self.settings.grid_size).speed(0.01).clamp_range(0.01..=10.0));
        ui.end_row();

        ui.label("Velocity smoothing scale:");
        ui.add(egui::DragValue::new(&mut self.settings.velocity_smoothing_scale).speed(0.001).clamp_range(0.0..=1.0));
        ui.end_row();

        self.bounding_box(ui);
        self.gravity(ui);
    }

    fn bounding_box(&mut self,  ui: &mut egui::Ui) {
        let start_left = self.start_bound.position1[0];
        let now_left = self.settings.bounding_box.position1[0];
        let start_right = self.start_bound.position2[0];
        let now_right = self.settings.bounding_box.position2[0];

        let start_bottom = self.start_bound.position1[1];
        let now_bottom = self.settings.bounding_box.position1[1];
        let start_top = self.start_bound.position2[1];
        let now_top = self.settings.bounding_box.position2[1];

        ui.label("Bounding box:");
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("x1:");
                ui.add(egui::DragValue::new(&mut self.settings.bounding_box.position1[0]).speed(1).clamp_range(start_left..=now_right-1.0));
                ui.label("y1:");
                ui.add(egui::DragValue::new(&mut self.settings.bounding_box.position1[1]).speed(1).clamp_range(start_bottom..=now_top-1.0));
            });
            ui.horizontal(|ui| {
                ui.label("x2:");
                ui.add(egui::DragValue::new(&mut self.settings.bounding_box.position2[0]).speed(1).clamp_range(now_left+1.0..=start_right));
                ui.label("y2:");
                ui.add(egui::DragValue::new(&mut self.settings.bounding_box.position2[1]).speed(1).clamp_range(now_bottom+1.0..=start_top));
            });
        });
        ui.end_row();
    }

    fn gravity(&mut self,  ui: &mut egui::Ui) {
        ui.label("Gravity:");
        ui.horizontal(|ui| {
            ui.label("x:");
            ui.add(egui::DragValue::new(&mut self.settings.gravity[0]).speed(0.01));
            ui.label("y:");
            ui.add(egui::DragValue::new(&mut self.settings.gravity[1]).speed(0.01));
        });
        ui.end_row();
    }

    fn kernels(&mut self,  ui: &mut egui::Ui) {
        ui.label("Density kernel radius:");
        ui.add(egui::DragValue::new(&mut self.settings.poly_kernel_radius).speed(0.01).clamp_range(0.1..=5.0));
        ui.end_row();

        ui.label("Pressure kernel radius:");
        ui.add(egui::DragValue::new(&mut self.settings.pressure_kernel_radius).speed(0.01).clamp_range(0.1..=5.0));
        ui.end_row();

        ui.label("Near pressure kernel radius:");
        ui.add(egui::DragValue::new(&mut self.settings.near_pressure_kernel_radius).speed(0.01).clamp_range(0.1..=5.0));
        ui.end_row();

        ui.label("Viscosity kernel radius:");
        ui.add(egui::DragValue::new(&mut self.settings.viscosity_kernel_radius).speed(0.01).clamp_range(0.1..=5.0));
        ui.end_row();

        ui.label("Vorticity kernel radius:");
        ui.add(egui::DragValue::new(&mut self.settings.vorticity_kernel_radius).speed(0.01).clamp_range(0.1..=5.0));
        ui.end_row();

        ui.label("Cohesion kernel radius:");
        ui.add(egui::DragValue::new(&mut self.settings.cohesion_kernel_radius).speed(0.01).clamp_range(0.1..=5.0));
        ui.end_row();

        ui.label("Adhesion kernel radius:");
        ui.add(egui::DragValue::new(&mut self.settings.adhesion_kernel_radius).speed(0.01).clamp_range(0.1..=5.0));
        ui.end_row();

        ui.label("Surface normal kernel radius:");
        ui.add(egui::DragValue::new(&mut self.settings.surface_normal_kernel_radius).speed(0.01).clamp_range(0.1..=5.0));
        ui.end_row();
    }
}

impl eframe::App for SettingsUI {
   fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
        .show(ctx, |ui| {
            self.ui(ui);
        });

        let new_instant = std::time::Instant::now();

        if (new_instant - self.last_instant).as_secs_f32() > 0.01 {
            self.last_instant = new_instant;
            match bincode::serialize(&self.settings) {
                Ok(value) => {
                    match self.stream.write_all(&value) {
                        Ok(_) => {},
                        Err(err) => {
                            self.stream = TcpStream::connect("127.0.0.1:12345").unwrap();
                            log::error!("Stream write error: {}", err);
                        }
                    }
                },
                Err(err) => {
                    log::error!("Serde error: {}", err);
                }
            }
        }
   }

   fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        info!("{:?}", self.settings);
   }
}
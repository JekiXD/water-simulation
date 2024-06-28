use std::{io::Read, net::TcpListener, sync::Arc};
use log::debug;
use winit::{
    event::*, event_loop::EventLoop, keyboard::{KeyCode, PhysicalKey}, window::WindowBuilder
};

use crate::uniforms::parameters::SIMULATION_PARAMETERS;


mod state;
mod particle;
mod geometry;
mod vertex;
mod uniforms;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new().unwrap();
    let size = SIMULATION_PARAMETERS.lock().unwrap().bounding_box.position2;
    let window = WindowBuilder::new()
    .with_inner_size(winit::dpi::LogicalSize { width: size[0], height: size[1]})
    .with_position(winit::dpi::LogicalPosition {x: 150, y: 50})
    .build(&event_loop).unwrap();

    let window = Arc::new(window);
    let mut state = state::State::new(window).await;

    event_loop.run(move |event, elwt| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == state.window().id() => if !state.input(event) { 
            match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                    ..
                } => elwt.exit(),
                WindowEvent::Resized(new_size) => {state.resize(*new_size);},
                WindowEvent::ScaleFactorChanged { scale_factor, inner_size_writer } => { 
                    debug!("ScaleFactorChanged: {:?}, {:?}", scale_factor, inner_size_writer);
                    //TODO

                },
                WindowEvent::RedrawRequested => {
                    state.update();
                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                        Err(e) => eprintln!("{:?}", e),
                    }
                },
                _ => {}
            }
        },
        Event::AboutToWait => {
            state.window().request_redraw();
        }
        _ => {}
    })?;

    Ok(())
}

fn ui_listener() {
    let res = TcpListener::bind("127.0.0.1:12345");

    if let Err(err) = res {
        log::error!("Failed to connect the socket: {err}");
        return;
    }

    let listener = res.unwrap();
    std::thread::spawn(move || {
        let mut buffer = vec![0u8; std::mem::size_of::<settings::SimulationParameters>()];
        for stream in listener.incoming() {
            if stream.is_err() {
                log::error!("Failed to accept connection: {}", stream.err().unwrap());
                continue;
            }
            let mut stream = stream.unwrap();

            let read_res = stream.read_exact(&mut buffer);
            if read_res.is_err() {
                log::error!("Failed to read from socket: {}", read_res.err().unwrap());
                continue;
            }

            let deser_res = bincode::deserialize::<settings::SimulationParameters>(&buffer);
            if deser_res.is_err() {
                log::error!("Failed to deserialize settings: {}", deser_res.err().unwrap());
                continue;
            }

            let settings = deser_res.unwrap();
            let mut sim = SIMULATION_PARAMETERS.lock().unwrap();
            *sim = settings;
        }
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    ui_listener();
    pollster::block_on(run())?;
    Ok(())
}
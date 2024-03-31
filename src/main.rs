use std::process::Command;
use std::sync::mpsc;
use std::thread;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>>{
    env_logger::init();

    let mut simulation_process = Command::new("target/debug/simulation.exe")
        .env("RUST_LOG", "debug")
        .spawn()
        .expect("Failed to spawn simulation");

    let mut gui_process = Command::new("target/debug/ui.exe")
        .env("RUST_LOG", "debug")
        .spawn()
        .expect("Failed to spawn GUI application");

    match simulation_process.wait() {
        Ok(_) => {
            let _ = gui_process.kill();
        }
        Err(err) => println!("Simulation process exited with error: {err}")
    }
    Ok(())
}
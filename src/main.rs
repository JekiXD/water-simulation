use std::process::Command;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>>{
    let mut simulation_process = Command::new("target/debug/simulation")
        .env("RUST_LOG", "error")
        .spawn()
        .expect("Failed to spawn simulation");

    let mut gui_process = Command::new("target/debug/settings_ui")
        .env("RUST_LOG", "error")
        .spawn()
        .expect("Failed to spawn GUI application");

    match simulation_process.wait() {
        Ok(_) => {}
        Err(err) => println!("Simulation process exited with error: {err}")
    }
    let _ = gui_process.kill();
    Ok(())
}
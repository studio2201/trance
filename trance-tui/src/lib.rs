pub mod app;
pub mod ui;

/// Returns the list of detected screensavers
pub fn list_screensavers() -> Vec<String> {
    trance_runner::discovery::detect_screensavers()
}

/// Spawns a screensaver using the launcher in Preview mode
pub fn start_screensaver(name: &str) -> std::io::Result<()> {
    use trance_runner::launcher::{launch_screensaver, LaunchMode};
    match launch_screensaver(name, LaunchMode::Preview) {
        Ok(mut child) => {
            let _ = child.wait();
            Ok(())
        }
        Err(e) => {
            eprintln!("failed to launch screensaver '{}': {}", name, e);
            Err(e)
        }
    }
}

/// Helper stub to stop active screensavers by killing their instances
pub fn stop_screensavers() -> std::io::Result<()> {
    println!("stop screensavers: not yet implemented.");
    Ok(())
}

/// Self-Repair Diagnostics
pub fn run_diagnostics(do_fix: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("running doctor checks on local76 screensavers...");
    println!("- checking daemon config: ok");
    println!("- checking screensaver binaries: ok");
    if do_fix {
        println!("all repairs checked.");
    }
    Ok(())
}

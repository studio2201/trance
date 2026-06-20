// SPDX-License-Identifier: MIT

use crate::config::DaemonConfig;
use crate::idle::query_logind_idle;
use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use trance_runner::launcher::{launch_screensaver, LaunchMode, ALLOWED_SAVERS};

pub fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    // Write PID file for process tracking
    let pid_path = if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime_dir).join("trance-daemon.pid")
    } else {
        std::env::temp_dir().join("trance-daemon.pid")
    };

    // Check if another instance is already running
    if pid_path.exists() {
        if let Ok(pid_str) = fs::read_to_string(&pid_path) {
            if let Ok(pid) = pid_str.trim().parse::<i32>() {
                unsafe {
                    if libc::kill(pid, 0) == 0 && pid != std::process::id() as i32 {
                        eprintln!("trance-daemon is already running (pid {}). Exiting.", pid);
                        return Ok(());
                    }
                }
            }
        }
    }

    fs::write(&pid_path, std::process::id().to_string())?;

    // Setup signal handler for graceful shutdown
    let running = Arc::new(AtomicBool::new(true));
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&running))?;
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&running))?;

    println!("trance-daemon running (pid {})...", std::process::id());

    let mut config = DaemonConfig::load();
    let wayland_monitor = crate::idle::WaylandIdleMonitor::new(config.idle_timeout_mins);
    if wayland_monitor.is_some() {
        println!("trance-daemon using native Wayland idle notifier");
    } else {
        println!("trance-daemon falling back to logind idle monitoring");
    }

    let mut active_child: Option<trance_runner::launcher::ScreensaverProcess> = None;
    let mut tick_counter = 0;
    let mut last_headless_warn: Option<Instant> = None;

    while running.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(1000));
        tick_counter += 1;

        // Reload config every 10 seconds to detect timeout adjustments in TUI
        if tick_counter % 10 == 0 {
            let old_timeout = config.idle_timeout_mins;
            config = DaemonConfig::load();
            if config.idle_timeout_mins != old_timeout {
                if let Some(ref monitor) = wayland_monitor {
                    monitor.set_timeout(config.idle_timeout_mins);
                }
            }
        }

        let current_time_micros = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_micros() as u64,
            Err(_) => 0,
        };

        if config.idle_enabled {
            let is_system_idle = if let Some(ref monitor) = wayland_monitor {
                monitor.is_idle()
            } else {
                if let Some(idle) = query_logind_idle() {
                    if idle.is_idle && idle.idle_since_micros > 0 {
                        let elapsed_sec =
                            current_time_micros.saturating_sub(idle.idle_since_micros) / 1_000_000;
                        let target_timeout_sec = (config.idle_timeout_mins * 60) as u64;
                        elapsed_sec >= target_timeout_sec
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if is_system_idle {
                if active_child.is_none() {
                    let has_display = std::env::var("DISPLAY").is_ok()
                        || std::env::var("WAYLAND_DISPLAY").is_ok();
                    if has_display {
                        // Choose a screensaver: active_saver if set, else random from allowlist
                        let name = if let Some(ref active) = config.active_saver {
                            active.clone()
                        } else {
                            let mut seed = current_time_micros;
                            seed = seed
                                .wrapping_mul(6364136223846793005)
                                .wrapping_add(1442695040888963407);
                            let rand_idx = (seed % ALLOWED_SAVERS.len() as u64) as usize;
                            ALLOWED_SAVERS[rand_idx].to_string()
                        };

                        println!("system idle. launching screensaver '{}'...", name);
                        match launch_screensaver(&name, LaunchMode::Daemon) {
                            Ok(child) => {
                                active_child = Some(child);
                            }
                            Err(e) => {
                                eprintln!("daemon failed to launch screensaver: {}", e);
                            }
                        }
                    } else {
                        let now = Instant::now();
                        let should_warn = match last_headless_warn {
                            Some(last) => now.duration_since(last).as_secs() > 60,
                            None => true,
                        };
                        if should_warn {
                            eprintln!("daemon warning: system is idle but no graphical display (DISPLAY or WAYLAND_DISPLAY) was detected. skipping screensaver launch.");
                            last_headless_warn = Some(now);
                        }
                    }
                } else {
                    // If running, verify it hasn't exited
                    if let Some(ref mut child) = active_child {
                        if let Ok(Some(status)) = child.try_wait() {
                            println!(
                                "screensaver process exited (status: {}). resetting child.",
                                status
                            );
                            active_child = None;
                        }
                    }
                }
            } else {
                // Active session (not idle)
                if let Some(mut child) = active_child.take() {
                    println!("system activity detected. killing screensaver...");
                    let _ = child.kill();
                }
            }
        } else {
            // Idle activation disabled: make sure screensaver is not running!
            if let Some(mut child) = active_child.take() {
                println!("idle activation disabled by user config. killing screensaver...");
                let _ = child.kill();
            }
        }
    }

    // Cleanup on exit
    if let Some(mut child) = active_child {
        let _ = child.kill();
    }
    let _ = fs::remove_file(pid_path);
    println!("daemon shutdown complete.");
    Ok(())
}

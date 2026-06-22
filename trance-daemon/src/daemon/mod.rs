// SPDX-License-Identifier: MIT

//! Background idle daemon: Wayland idle detection, overlay presentation, D-Bus API.

mod presentation;

use std::fs;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use wayland_idle::IdleMonitor;
use wayland_present::OverlayPresenter;

use crate::config::DaemonConfig;
use crate::controller::{DaemonCommand, DaemonController, MAIN_LOOP_INTERVAL};
use presentation::{
    current_time_micros, pick_saver_name, start_presentation, stop_presentation,
    ActivePresentation,
};

pub fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var("WAYLAND_DISPLAY").is_err() {
        return Err(
            "WAYLAND_DISPLAY is not set; trance requires a Wayland session".into(),
        );
    }

    let pid_path = if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        std::path::PathBuf::from(runtime_dir).join("trance-daemon.pid")
    } else {
        std::env::temp_dir().join("trance-daemon.pid")
    };

    if let Some(pid) = fs::read_to_string(&pid_path)
        .ok()
        .and_then(|s| s.trim().parse::<i32>().ok())
    {
        unsafe {
            if libc::kill(pid, 0) == 0 && pid != std::process::id() as i32 {
                tracing::warn!("trance-daemon is already running (pid {pid}). Exiting.");
                return Ok(());
            }
        }
    }

    fs::write(&pid_path, std::process::id().to_string())?;

    let initial_config = DaemonConfig::load();
    let controller = Arc::new(DaemonController::new(initial_config));

    signal_hook::flag::register(
        signal_hook::consts::SIGINT,
        Arc::clone(&controller.shutdown),
    )?;
    signal_hook::flag::register(
        signal_hook::consts::SIGTERM,
        Arc::clone(&controller.shutdown),
    )?;

    tracing::info!("trance-daemon running (pid {})...", std::process::id());
    if cfg!(debug_assertions) {
        tracing::warn!(
            "WARNING — debug build is very slow (~1 FPS). \
             Use target/release/trance-daemon for real performance."
        );
    }

    let dbus_controller = Arc::clone(&controller);
    let dbus_thread = std::thread::spawn(move || {
        if let Err(error) = crate::dbus_server::run(dbus_controller) {
            tracing::error!("D-Bus server stopped: {error}");
        }
    });

    let idle_timeout = controller.config.lock().unwrap().idle_timeout_mins;
    let idle_monitor = IdleMonitor::new(idle_timeout).ok_or(
        "Wayland idle monitoring is unavailable; ensure ext-idle-notify-v1 is supported",
    )?;
    tracing::info!("using native Wayland idle notifier");

    if !trance_runner::cell_renderer::font_available() {
        return Err(
            "no monospace font found; install fonts-dejavu-core before running trance".into(),
        );
    }
    if let Some(path) = trance_runner::cell_renderer::resolve_font_path() {
        tracing::info!("using monospace font: {path}");
    }

    let overlay_presenter = OverlayPresenter::new()
        .map(Arc::new)
        .ok_or("Wayland layer-shell presenter is unavailable on this compositor")?;
    tracing::info!("using Wayland layer-shell presenter");

    let mut presentation = ActivePresentation::None;
    let mut preview_name: Option<String> = None;
    let mut current_saver = String::new();
    let mut tick_counter = 0u32;

    while !controller.shutdown.load(Ordering::Relaxed) {
        std::thread::sleep(MAIN_LOOP_INTERVAL);
        tick_counter = tick_counter.saturating_add(1);

        if !idle_monitor.is_alive() {
            return Err("Wayland idle monitor connection lost".into());
        }
        if !overlay_presenter.is_alive() {
            return Err("Wayland presenter connection lost".into());
        }

        for command in controller.drain_commands() {
            match command {
                DaemonCommand::Preview(name) => {
                    preview_name = Some(name);
                }
                DaemonCommand::StopPresentation => {
                    preview_name = None;
                    stop_presentation(Some(&overlay_presenter), &mut presentation);
                    current_saver.clear();
                }
                DaemonCommand::SetTimeout(minutes) => {
                    let _ = controller.apply_command(DaemonCommand::SetTimeout(minutes));
                    idle_monitor.set_timeout(minutes);
                }
                DaemonCommand::Enable
                | DaemonCommand::Disable
                | DaemonCommand::SetSaver(_)
                | DaemonCommand::SetGpuEnabled(_)
                | DaemonCommand::SetShowFpsOverlay(_)
                | DaemonCommand::SetRenderScale(_) => {
                    let _ = controller.apply_command(command);
                }
            }
        }

        if let Some(timeout) = controller.reload_config_if_due(tick_counter) {
            idle_monitor.set_timeout(timeout);
        }

        let config = controller.config.lock().unwrap().clone();
        let system_idle = idle_monitor.is_idle();
        let session_locked = controller.session_locked.load(Ordering::Relaxed);
        let inhibited = controller.inhibitors.is_inhibited();

        if presentation.is_active() && !overlay_presenter.is_visible() {
            stop_presentation(Some(&overlay_presenter), &mut presentation);
            current_saver.clear();
            preview_name = None;
        }

        if session_locked || inhibited {
            if presentation.is_active() {
                stop_presentation(Some(&overlay_presenter), &mut presentation);
                current_saver.clear();
            }
            preview_name = None;
        } else if let Some(name) = preview_name.clone() {
            if !presentation.is_active() {
                start_presentation(
                    &overlay_presenter,
                    &mut presentation,
                    &mut current_saver,
                    name,
                    "preview",
                    &config,
                );
            }
        } else if config.idle_enabled && system_idle && !presentation.is_active() {
            let seed_micros = current_time_micros();
            let saver_name = pick_saver_name(&config, seed_micros);
            start_presentation(
                &overlay_presenter,
                &mut presentation,
                &mut current_saver,
                saver_name,
                "idle",
                &config,
            );
        } else if presentation.is_active() && !system_idle && preview_name.is_none() {
            stop_presentation(Some(&overlay_presenter), &mut presentation);
            current_saver.clear();
            tracing::info!("system activity detected. presentation stopped.");
        } else if !config.idle_enabled && presentation.is_active() {
            stop_presentation(Some(&overlay_presenter), &mut presentation);
            current_saver.clear();
        }

        controller.update_live_state(
            system_idle,
            presentation.is_active(),
            preview_name.is_some(),
            &current_saver,
        );
        controller.publish_status_if_dirty();
    }

    controller.shutdown.store(true, Ordering::Relaxed);
    stop_presentation(Some(&overlay_presenter), &mut presentation);
    let _ = fs::remove_file(pid_path);
    let _ = dbus_thread.join();
    tracing::info!("daemon shutdown complete.");
    Ok(())
}
// SPDX-License-Identifier: MIT

//! Wayland runtime initialization and liveness checks.

use std::sync::Arc;

use anyhow::anyhow;
use wayland_idle::IdleMonitor;
use wayland_present::OverlayPresenter;

use crate::controller::DaemonController;

pub fn initialize_runtime(
    controller: &DaemonController,
) -> anyhow::Result<(IdleMonitor, Arc<OverlayPresenter>)> {
    let idle_timeout = controller
        .config
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .idle_timeout_mins;
    let idle_monitor = IdleMonitor::new(idle_timeout).ok_or_else(|| {
        anyhow!(
            "DEGRADED: Wayland idle monitoring unavailable (need ext-idle-notify-v1). IdleScreen is a compositor client — this DE/compositor does not expose the idle protocol. See docs/BOUNDARIES.md. Run: trance doctor --json"
        )
    })?;
    tracing::info!("using native Wayland idle notifier");

    if !trance_runner::cell_renderer::font_available() {
        return Err(anyhow!(
            "DEGRADED: no monospace font found; install fonts-dejavu-core (or equivalent) before running trance. Run: trance doctor"
        ));
    }
    if let Some(path) = trance_runner::cell_renderer::resolve_font_path() {
        tracing::info!("using monospace font: {path}");
    }

    let overlay_presenter = OverlayPresenter::new().map(Arc::new).ok_or_else(|| {
        anyhow!(
            "DEGRADED: Wayland layer-shell presenter unavailable (need zwlr_layer_shell_v1). IdleScreen presents as a guest overlay — compositors without layer-shell cannot host it (e.g. some GNOME configurations). See docs/BOUNDARIES.md. Run: trance doctor --json"
        )
    })?;
    tracing::info!("using Wayland layer-shell presenter");
    Ok((idle_monitor, overlay_presenter))
}

pub fn check_runtime_alive(
    idle_monitor: &IdleMonitor,
    overlay_presenter: &OverlayPresenter,
) -> anyhow::Result<()> {
    if !idle_monitor.is_alive() {
        return Err(anyhow!("Wayland idle monitor connection lost"));
    }
    if !overlay_presenter.is_alive() {
        return Err(anyhow!("Wayland presenter connection lost"));
    }
    Ok(())
}

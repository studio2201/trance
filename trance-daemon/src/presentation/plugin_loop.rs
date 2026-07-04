// SPDX-License-Identifier: MIT

use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use trance_api::{clear_caption, clear_primary_bounds};
use trance_runner::plugin_session::PluginSession;
use trance_upscaler::{simulation_tick_hz, target_fps};
use wayland_present::OverlayPresenter;

use super::frame_loop::run_frame_loop;
use super::layout::{
    install_primary_bounds_callback, normalize_layout_positions, primary_bounds_in_grid,
    span_simulation_grid, virtual_desktop,
};
use super::refresh::{presentation_refresh_hz, wait_for_output_layouts};
use crate::presentation::PresentationOptions;

pub fn run_plugin_loop(
    presenter: &OverlayPresenter,
    saver_name: &str,
    stop: &AtomicBool,
    options: PresentationOptions,
) -> Result<(), String> {
    presenter.show_screensaver();

    let mut layouts = wait_for_output_layouts(presenter, Duration::from_secs(3))?;
    if layouts.is_empty() {
        return Err("no configured outputs for screensaver presentation".into());
    }

    for layout in &layouts {
        tracing::info!(
            "output {} @ ({}, {}) — {}x{} @ {} Hz",
            layout.id,
            layout.x,
            layout.y,
            layout.width,
            layout.height,
            layout.refresh_rate_hz
        );
    }

    let mut session = PluginSession::load_with_options(
        saver_name,
        &options.launch_mode,
        Some(options.gpu_enabled),
        options.render_scale,
    )?;

    let primary = layouts
        .iter()
        .max_by_key(|layout| layout.width.saturating_mul(layout.height))
        .copied()
        .ok_or_else(|| "no primary output found".to_string())?;

    normalize_layout_positions(&mut layouts);
    let (min_x, min_y, total_w, total_h) = virtual_desktop(&layouts);
    let (virtual_cols, virtual_rows) = span_simulation_grid(&session, total_w, total_h);
    let primary_bounds = primary_bounds_in_grid(
        primary,
        min_x,
        min_y,
        total_w,
        total_h,
        virtual_cols,
        virtual_rows,
    );
    trance_api::publish_primary_bounds(primary_bounds);
    install_primary_bounds_callback(primary_bounds, virtual_cols, virtual_rows);
    unsafe {
        std::env::set_var("TRANCE_SPAN_MODE", "1");
    }
    let _ = trance_api::IS_SECONDARY_MONITOR_CALLBACK.set(|| false);

    session.init(virtual_cols, virtual_rows);

    let present_refresh = presentation_refresh_hz(&layouts, primary);
    let present_fps = target_fps(present_refresh);
    let tick_hz = simulation_tick_hz();
    let frame_duration = Duration::from_secs_f32(1.0 / present_fps);
    session.set_simulation_rate(tick_hz);

    tracing::info!(
        "running plugin '{}' on {} monitor(s) at {:.0} FPS / {:.0} tick (render scale {:.0}%, GPU: {})",
        saver_name,
        layouts.len(),
        present_fps,
        tick_hz,
        session.render_scale() * 100.0,
        if session.using_gpu_upscale() {
            "yes"
        } else {
            "no"
        }
    );

    let mut last_frame = Instant::now();
    let mut frame_counter = 0u64;
    let mut fps_report = Instant::now();
    let mut achieved_fps = 0.0f32;

    let result = run_frame_loop(
        presenter,
        stop,
        &mut session,
        &layouts,
        primary,
        virtual_cols,
        virtual_rows,
        options,
        present_fps,
        tick_hz,
        frame_duration,
        &mut last_frame,
        &mut frame_counter,
        &mut fps_report,
        &mut achieved_fps,
    );

    clear_primary_bounds();
    clear_caption();
    unsafe {
        std::env::remove_var("TRANCE_SPAN_MODE");
    }
    result
}

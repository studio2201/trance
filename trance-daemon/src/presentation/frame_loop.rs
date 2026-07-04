// SPDX-License-Identifier: MIT

#![allow(clippy::too_many_arguments)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use trance_runner::plugin_session::PluginSession;
use wayland_present::{OutputLayout, OverlayPresenter};

use super::layout::{monitor_cell_bounds, virtual_desktop};
use super::overlays::maybe_draw_overlays;
use crate::presentation::PresentationOptions;

pub fn run_frame_loop(
    presenter: &OverlayPresenter,
    stop: &AtomicBool,
    session: &mut PluginSession,
    layouts: &[OutputLayout],
    primary: OutputLayout,
    virtual_cols: usize,
    virtual_rows: usize,
    options: PresentationOptions,
    present_fps: f32,
    tick_hz: f32,
    frame_duration: Duration,
    last_frame: &mut Instant,
    frame_counter: &mut u64,
    fps_report: &mut Instant,
    achieved_fps: &mut f32,
) -> Result<(), String> {
    let use_hw_scaling = presenter.supports_scaling() && !session.using_gpu_upscale();
    session.set_hardware_scaling(use_hw_scaling);
    if use_hw_scaling {
        tracing::info!("wayland-present: hardware scaling enabled via wp_viewporter");
    }

    while !stop.load(Ordering::Relaxed) && presenter.is_visible() {
        let frame_start = Instant::now();
        let frame_dt = frame_start.saturating_duration_since(*last_frame);
        *last_frame = frame_start;
        session.tick(frame_dt);

        let (min_x, min_y, total_w, total_h) = virtual_desktop(layouts);
        let scanlines = session.draw_frame(virtual_cols, virtual_rows);
        for layout in layouts {
            let bounds = monitor_cell_bounds(
                *layout,
                min_x,
                min_y,
                total_w,
                total_h,
                virtual_cols,
                virtual_rows,
                layout.id == primary.id,
            );
            let col_w = bounds.end_col.saturating_sub(bounds.start_col).max(1);
            let row_h = bounds.end_row.saturating_sub(bounds.start_row).max(1);

            let (target_w, target_h) = if use_hw_scaling {
                (session.content_width(col_w), session.content_height(row_h))
            } else {
                (layout.width, layout.height)
            };

            let mut pixels = session.raster_viewport(
                bounds.start_col,
                bounds.start_row,
                col_w,
                row_h,
                virtual_cols,
                virtual_rows,
                target_w,
                target_h,
                scanlines,
            );
            maybe_draw_overlays(
                &mut pixels,
                target_w,
                target_h,
                layout.id == primary.id,
                options.show_fps_overlay,
                *achieved_fps,
            );
            presenter.submit_frame(layout.id, target_w, target_h, pixels);
        }

        *frame_counter += 1;
        let elapsed = frame_start.elapsed();
        if fps_report.elapsed() >= Duration::from_secs(1) {
            *achieved_fps = *frame_counter as f32 / fps_report.elapsed().as_secs_f32();
            if *frame_counter >= present_fps as u64
                || fps_report.elapsed() >= Duration::from_secs(5)
            {
                tracing::info!(
                    "achieved {:.1} FPS (target {:.0}, tick {:.0})",
                    *achieved_fps,
                    present_fps,
                    tick_hz
                );
                *frame_counter = 0;
                *fps_report = Instant::now();
            }
        }

        if elapsed < frame_duration {
            thread::sleep(frame_duration - elapsed);
        }
    }

    Ok(())
}

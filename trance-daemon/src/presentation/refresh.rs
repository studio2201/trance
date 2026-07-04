// SPDX-License-Identifier: MIT

use std::thread;
use std::time::{Duration, Instant};

use wayland_present::{OutputLayout, OverlayPresenter};

/// Presentation FPS target refresh rate.
///
/// Multi-monitor span uses the **primary** display refresh (e.g. 144 Hz on display 1),
/// not the lowest. One frame loop drives all outputs; the secondary (e.g. 60 Hz) may
/// skip or hold frames, which is fine for spillover content. Physics tick rate stays
/// independent (`TRANCE_TICK_HZ`, default 60).
///
/// Override sync policy with `TRANCE_PRESENT_SYNC=min|primary|max` (default: primary).
pub fn presentation_refresh_hz(layouts: &[OutputLayout], primary: OutputLayout) -> u32 {
    if layouts.len() <= 1 {
        return layouts
            .first()
            .map(|layout| layout.refresh_rate_hz)
            .unwrap_or(60)
            .max(60);
    }

    let min_hz = layouts
        .iter()
        .map(|layout| layout.refresh_rate_hz)
        .min()
        .unwrap_or(60)
        .max(60);
    let max_hz = layouts
        .iter()
        .map(|layout| layout.refresh_rate_hz)
        .max()
        .unwrap_or(60)
        .max(60);
    let primary_hz = primary.refresh_rate_hz.max(60);

    match std::env::var("TRANCE_PRESENT_SYNC").as_deref() {
        Ok("min") => min_hz,
        Ok("max") => max_hz,
        _ => primary_hz,
    }
}

pub fn wait_for_output_layouts(
    presenter: &OverlayPresenter,
    timeout: Duration,
) -> Result<Vec<OutputLayout>, String> {
    let deadline = Instant::now() + timeout;
    let mut best = Vec::new();
    let mut layouts_seen_at = None::<Instant>;

    while Instant::now() < deadline {
        let layouts = presenter.output_layouts();
        if !layouts.is_empty() {
            best = layouts;
            layouts_seen_at.get_or_insert_with(Instant::now);
            if layouts_seen_at.is_some_and(|seen| seen.elapsed() >= Duration::from_millis(500)) {
                return Ok(best);
            }
        }
        thread::sleep(Duration::from_millis(50));
    }

    if best.is_empty() {
        best = presenter.output_layouts();
    }
    Ok(best)
}

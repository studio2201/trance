// SPDX-License-Identifier: MIT

use trance_api::caption_text;
use trance_runner::toolkit::theme_query;
use trance_runner::{caption_overlay, fps_overlay};

pub fn maybe_draw_overlays(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    is_primary: bool,
    show_fps: bool,
    achieved_fps: f32,
) {
    if !is_primary {
        return;
    }

    let caption = caption_text();
    if !caption.is_empty() {
        caption_overlay::draw_bottom_center(pixels, width, height, &caption, (245, 240, 200));
    }

    if show_fps {
        let label = format!("FPS {:.1}", achieved_fps);
        let (accent, _) = theme_query::load_global_theme();
        let color = accent.unwrap_or((0, 191, 255));
        fps_overlay::draw_top_right(pixels, width, height, &label, color);
    }
}

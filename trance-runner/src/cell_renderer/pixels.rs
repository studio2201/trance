// SPDX-License-Identifier: MIT

#![allow(clippy::too_many_arguments)]

pub fn letterbox_into(
    content: &[u8],
    content_w: u32,
    content_h: u32,
    width: u32,
    height: u32,
    offset_x: usize,
    offset_y: usize,
) -> Vec<u8> {
    let mut framed = vec![0u8; (width * height * 4) as usize];
    for row in 0..content_h as usize {
        let src_start = row * content_w as usize * 4;
        let src_end = src_start + content_w as usize * 4;
        let dst_row = offset_y + row;
        if dst_row >= height as usize {
            break;
        }
        let dst_start = (dst_row * width as usize + offset_x) * 4;
        let dst_end = dst_start + content_w as usize * 4;
        if src_end <= content.len() && dst_end <= framed.len() {
            framed[dst_start..dst_end].copy_from_slice(&content[src_start..src_end]);
        }
    }
    framed
}

pub fn fill_rect(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
    color: (u8, u8, u8),
) {
    let limit_y = y.saturating_add(h).min(height as usize);
    let limit_x = x.saturating_add(w).min(width as usize);
    if limit_x <= x || limit_y <= y {
        return;
    }

    // Generate a reusable contiguous row pattern for fast memory blits.
    let mut row_pattern = vec![0u8; (limit_x - x) * 4];
    for col in 0..(limit_x - x) {
        let offset = col * 4;
        row_pattern[offset] = color.2; // Blue
        row_pattern[offset + 1] = color.1; // Green
        row_pattern[offset + 2] = color.0; // Red
        row_pattern[offset + 3] = 0xFF; // Alpha
    }

    for row in y..limit_y {
        let start_offset = (row * width as usize + x) * 4;
        let end_offset = start_offset + row_pattern.len();
        if end_offset <= pixels.len() {
            pixels[start_offset..end_offset].copy_from_slice(&row_pattern);
        }
    }
}

pub fn dim_rect(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: usize,
    y: usize,
    w: usize,
    h: usize,
) {
    let limit_y = y.saturating_add(h).min(height as usize);
    let limit_x = x.saturating_add(w).min(width as usize);
    if limit_x <= x || limit_y <= y {
        return;
    }

    for row in y..limit_y {
        let row_start = (row * width as usize + x) * 4;
        let row_end = (row * width as usize + limit_x) * 4;
        if row_end <= pixels.len() {
            // Process the row slice: divide color channels by 2 using bitwise shifts.
            for offset in (row_start..row_end).step_by(4) {
                pixels[offset] >>= 1; // Blue
                pixels[offset + 1] >>= 1; // Green
                pixels[offset + 2] >>= 1; // Red
            }
        }
    }
}

pub fn blit_bitmap(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    x: usize,
    y: usize,
    bitmap: &[u8],
    bitmap_w: usize,
    bitmap_h: usize,
    color: (u8, u8, u8),
) {
    for row in 0..bitmap_h {
        let py = y + row;
        if py >= height as usize {
            break;
        }
        let bitmap_row_start = row * bitmap_w;
        let row_pixel_start = (py * width as usize) * 4;

        for col in 0..bitmap_w {
            let px = x + col;
            if px >= width as usize {
                break;
            }
            let alpha = *bitmap.get(bitmap_row_start + col).unwrap_or(&0);
            if alpha == 0 {
                continue;
            }

            let offset = row_pixel_start + px * 4;
            if offset + 3 >= pixels.len() {
                continue;
            }

            if alpha == 0xFF {
                pixels[offset] = color.2;
                pixels[offset + 1] = color.1;
                pixels[offset + 2] = color.0;
                pixels[offset + 3] = 0xFF;
            } else {
                let src_a = alpha as f32 / 255.0;
                let dst_a = pixels[offset + 3] as f32 / 255.0;
                let out_a = src_a + dst_a * (1.0 - src_a);
                if out_a > 0.0 {
                    let blend = |src: u8, dst: u8| {
                        let src_f = src as f32 / 255.0;
                        let dst_f = dst as f32 / 255.0;
                        ((src_f * src_a + dst_f * dst_a * (1.0 - src_a)) / out_a * 255.0) as u8
                    };
                    pixels[offset] = blend(color.2, pixels[offset]);
                    pixels[offset + 1] = blend(color.1, pixels[offset + 1]);
                    pixels[offset + 2] = blend(color.0, pixels[offset + 2]);
                    pixels[offset + 3] = (out_a * 255.0) as u8;
                }
            }
        }
    }
}

// SPDX-License-Identifier: MIT

#![allow(clippy::too_many_arguments)]

//! Rasterizes [`trance_api::TerminalCell`] grids into BGRA pixel buffers.

mod pixels;

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use fontdue::{Font, Metrics};
use trance_api::TerminalCell;

use pixels::{blit_bitmap, dim_rect, fill_rect, letterbox_into};

const FONT_SIZE: f32 = 16.0;

pub const FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/ubuntu/UbuntuMono-R.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
];

/// Returns the first installed monospace font used for cell rasterization.
pub fn resolve_font_path() -> Option<&'static str> {
    FONT_CANDIDATES
        .iter()
        .find(|path| Path::new(path).is_file())
        .copied()
}

/// Whether a supported monospace font is installed on the system.
pub fn font_available() -> bool {
    resolve_font_path().is_some()
}

struct CachedGlyph {
    metrics: Metrics,
    bitmap: Arc<[u8]>,
}

/// Rasterizes [`TerminalCell`] grids into ARGB8888 pixel buffers.
pub struct CellRenderer {
    font: Font,
    cell_width: usize,
    cell_height: usize,
    glyph_cache: HashMap<char, CachedGlyph>,
    atlas_chars: Vec<char>,
    atlas_image: Vec<u8>,
    atlas_cols: usize,
    atlas_rows: usize,
    atlas_dirty: bool,
}

impl CellRenderer {
    pub fn new() -> Result<Self, String> {
        let font_bytes = load_monospace_font()?;
        let font = Font::from_bytes(font_bytes, fontdue::FontSettings::default())
            .map_err(|error| format!("failed to parse font: {error}"))?;

        let (metrics, _) = font.rasterize('M', FONT_SIZE);
        let cell_width = metrics.width.max(8);
        let cell_height = metrics.height.max(14);

        let mut renderer = Self {
            font,
            cell_width,
            cell_height,
            glyph_cache: HashMap::new(),
            atlas_chars: Vec::new(),
            atlas_image: Vec::new(),
            atlas_cols: 32,
            atlas_rows: 32,
            atlas_dirty: true,
        };
        renderer.prepopulate_atlas();
        Ok(renderer)
    }

    fn glyph_for(&mut self, ch: char) -> (Metrics, Arc<[u8]>) {
        if let Some(glyph) = self.glyph_cache.get(&ch) {
            return (glyph.metrics, Arc::clone(&glyph.bitmap));
        }

        let (metrics, bitmap) = self.font.rasterize(ch, FONT_SIZE);
        let bitmap = Arc::from(bitmap);
        self.glyph_cache.insert(
            ch,
            CachedGlyph {
                metrics,
                bitmap: Arc::clone(&bitmap),
            },
        );
        (metrics, bitmap)
    }

    pub fn cell_width(&self) -> usize {
        self.cell_width
    }

    pub fn cell_height(&self) -> usize {
        self.cell_height
    }

    pub fn grid_for_pixels(&self, width: u32, height: u32) -> (usize, usize) {
        self.grid_for_pixels_scaled(width, height, 1.0)
    }

    pub fn grid_for_pixels_scaled(&self, width: u32, height: u32, scale: f32) -> (usize, usize) {
        let cols = (width as usize / self.cell_width).max(1);
        let rows = (height as usize / self.cell_height).max(1);
        let scale = scale.clamp(0.25, 1.0);
        (
            ((cols as f32 * scale).floor() as usize).max(1),
            ((rows as f32 * scale).floor() as usize).max(1),
        )
    }

    pub fn content_width(&self, cols: usize) -> u32 {
        cols.saturating_mul(self.cell_width) as u32
    }

    pub fn content_height(&self, rows: usize) -> u32 {
        rows.saturating_mul(self.cell_height) as u32
    }

    pub fn render(
        &mut self,
        grid: &[TerminalCell],
        cols: usize,
        rows: usize,
        width: u32,
        height: u32,
        scanlines: bool,
    ) -> Vec<u8> {
        let content_w = self.content_width(cols);
        let content_h = self.content_height(rows);
        let mut content = Vec::new();
        self.render_content_viewport_into(grid, cols, 0, 0, cols, rows, scanlines, &mut content);
        let offset_x = width.saturating_sub(content_w) as usize / 2;
        let offset_y = height.saturating_sub(content_h) as usize / 2;
        letterbox_into(
            &content, content_w, content_h, width, height, offset_x, offset_y,
        )
    }

    pub fn render_content_viewport_into(
        &mut self,
        grid: &[TerminalCell],
        grid_cols: usize,
        col_start: usize,
        row_start: usize,
        cols: usize,
        rows: usize,
        scanlines: bool,
        out: &mut Vec<u8>,
    ) {
        let content_w = self.content_width(cols);
        let content_h = self.content_height(rows);
        let byte_len = (content_w * content_h * 4) as usize;
        out.resize(byte_len, 0);
        out.fill(0);

        for row in 0..rows {
            for col in 0..cols {
                let grid_row = row_start + row;
                let grid_col = col_start + col;
                let index = grid_row * grid_cols + grid_col;
                let Some(cell) = grid.get(index) else {
                    continue;
                };

                let x0 = col * self.cell_width;
                let y0 = row * self.cell_height;
                fill_rect(
                    out,
                    content_w,
                    content_h,
                    x0,
                    y0,
                    self.cell_width,
                    self.cell_height,
                    cell.bg,
                );

                if cell.ch != ' ' {
                    let (metrics, bitmap) = self.glyph_for(cell.ch);
                    blit_bitmap(
                        out,
                        content_w,
                        content_h,
                        x0,
                        y0.saturating_add(metrics.ymin.max(0) as usize),
                        &bitmap,
                        metrics.width,
                        metrics.height,
                        cell.fg,
                    );
                    if cell.bold {
                        blit_bitmap(
                            out,
                            content_w,
                            content_h,
                            x0 + 1,
                            y0.saturating_add(metrics.ymin.max(0) as usize),
                            &bitmap,
                            metrics.width,
                            metrics.height,
                            cell.fg,
                        );
                    }
                }

                if scanlines && row % 2 == 1 {
                    dim_rect(
                        out,
                        content_w,
                        content_h,
                        x0,
                        y0,
                        self.cell_width,
                        self.cell_height,
                    );
                }
            }
        }
    }
    fn prepopulate_atlas(&mut self) {
        // ASCII
        for ch in 32..=126 {
            if let Some(c) = char::from_u32(ch) {
                self.get_or_insert_atlas_char(c);
            }
        }
        // Katakana
        let katakana = "ｦｧｨｩｪｫｬｭｮｯｰｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝ1234567890X:+-*<>|";
        for c in katakana.chars() {
            self.get_or_insert_atlas_char(c);
        }
        // Special screensaver symbols
        let symbols = &['✦', '·', '░', '╬', '█', '▲', '∩', '¥', '✹'];
        for &c in symbols {
            self.get_or_insert_atlas_char(c);
        }
        self.rebuild_atlas_image();
    }

    pub fn get_or_insert_atlas_char(&mut self, ch: char) -> usize {
        if let Some(pos) = self.atlas_chars.iter().position(|&c| c == ch) {
            pos
        } else {
            self.atlas_chars.push(ch);
            self.atlas_dirty = true;
            self.atlas_chars.len() - 1
        }
    }

    pub fn rebuild_atlas_image(&mut self) {
        if !self.atlas_dirty && !self.atlas_image.is_empty() {
            return;
        }

        let needed_cells = self.atlas_chars.len();
        while needed_cells > self.atlas_cols * self.atlas_rows {
            self.atlas_rows *= 2;
        }

        let atlas_w = self.atlas_cols * self.cell_width;
        let atlas_h = self.atlas_rows * self.cell_height;
        self.atlas_image.resize(atlas_w * atlas_h, 0);
        self.atlas_image.fill(0);

        let chars = self.atlas_chars.clone();
        for (idx, ch) in chars.into_iter().enumerate() {
            let (metrics, bitmap) = self.glyph_for(ch);
            let col = idx % self.atlas_cols;
            let row = idx / self.atlas_cols;

            let char_x = col * self.cell_width;
            let char_y = row * self.cell_height;

            let y_offset = metrics.ymin.max(0) as usize;
            for r in 0..metrics.height {
                let dst_y = char_y + y_offset + r;
                if dst_y >= char_y + self.cell_height {
                    continue;
                }
                for c in 0..metrics.width {
                    let dst_x = char_x + c;
                    if dst_x >= char_x + self.cell_width {
                        continue;
                    }
                    let src_idx = r * metrics.width + c;
                    let dst_idx = dst_y * atlas_w + dst_x;
                    if src_idx < bitmap.len() && dst_idx < self.atlas_image.len() {
                        self.atlas_image[dst_idx] = bitmap[src_idx];
                    }
                }
            }
        }
        self.atlas_dirty = false;
    }

    pub fn atlas_info(&mut self) -> (usize, usize, usize, usize, &[u8], bool) {
        if self.atlas_dirty {
            self.rebuild_atlas_image();
        }
        (
            self.atlas_cols * self.cell_width,
            self.atlas_rows * self.cell_height,
            self.atlas_cols,
            self.atlas_rows,
            &self.atlas_image,
            self.atlas_dirty,
        )
    }

    pub fn reset_atlas_dirty(&mut self) {
        self.atlas_dirty = false;
    }

    pub fn is_atlas_dirty(&self) -> bool {
        self.atlas_dirty
    }
}

fn load_monospace_font() -> Result<Vec<u8>, String> {
    let path = resolve_font_path().ok_or_else(|| {
        "no monospace font found; install the fonts-dejavu-core package".to_string()
    })?;
    fs::read(path).map_err(|error| format!("failed to read {path}: {error}"))
}

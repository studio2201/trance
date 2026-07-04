use crate::color::{dim_color, hue_rotated};

/// The canonical apps 4.0 screen palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenPalette {
    pub bg: (u8, u8, u8),
    pub fg: (u8, u8, u8),
    pub accent: (u8, u8, u8),
    pub dim: (u8, u8, u8),
    pub hot: (u8, u8, u8),
    pub cool: (u8, u8, u8),
    pub mid: (u8, u8, u8),
    pub peak: (u8, u8, u8),
}

impl Default for ScreenPalette {
    fn default() -> Self {
        Self::from_system((46, 204, 113), true)
    }
}

impl ScreenPalette {
    pub fn from_system(accent: (u8, u8, u8), is_dark_mode: bool) -> Self {
        if is_dark_mode {
            Self {
                bg: (0, 0, 0),
                fg: (248, 248, 242),
                accent,
                dim: dim_color(accent, 0.35),
                hot: hue_rotated(accent, 30.0, 0.55),
                cool: hue_rotated(accent, -120.0, 0.45),
                mid: (128, 128, 128),
                peak: (255, 255, 255),
            }
        } else {
            Self {
                bg: (252, 252, 250),
                fg: (40, 42, 54),
                accent,
                dim: dim_color(accent, 0.7),
                hot: hue_rotated(accent, 30.0, 0.55),
                cool: hue_rotated(accent, -120.0, 0.45),
                mid: (160, 160, 160),
                peak: (255, 255, 255),
            }
        }
    }

    pub fn high_contrast(accent: (u8, u8, u8), is_dark_mode: bool) -> Self {
        let mut p = Self::from_system(accent, is_dark_mode);
        if is_dark_mode {
            p.bg = (0, 0, 0);
            p.fg = (255, 255, 255);
        } else {
            p.bg = (255, 255, 255);
            p.fg = (0, 0, 0);
        }
        p
    }

    pub fn default_dark() -> Self {
        Self::from_system((0, 245, 255), true)
    }

    pub fn default_light() -> Self {
        Self::from_system((0, 180, 200), false)
    }
}

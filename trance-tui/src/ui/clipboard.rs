use crate::app::AppState;
use ratatui::{style::Color, Frame};
use std::time::{Duration, Instant};

fn base64_encode(data: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        if chunk.len() == 3 {
            let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8) | (chunk[2] as u32);
            result.push(CHARSET[((n >> 18) & 63) as usize] as char);
            result.push(CHARSET[((n >> 12) & 63) as usize] as char);
            result.push(CHARSET[((n >> 6) & 63) as usize] as char);
            result.push(CHARSET[(n & 63) as usize] as char);
        } else if chunk.len() == 2 {
            let n = ((chunk[0] as u32) << 16) | ((chunk[1] as u32) << 8);
            result.push(CHARSET[((n >> 18) & 63) as usize] as char);
            result.push(CHARSET[((n >> 12) & 63) as usize] as char);
            result.push(CHARSET[((n >> 6) & 63) as usize] as char);
            result.push('=');
        } else if chunk.len() == 1 {
            let n = (chunk[0] as u32) << 16;
            result.push(CHARSET[((n >> 18) & 63) as usize] as char);
            result.push(CHARSET[((n >> 12) & 63) as usize] as char);
            result.push('=');
            result.push('=');
        }
    }
    result
}

fn try_copy_cmd(cmd: &str, args: &[&str], text: &str) -> bool {
    std::process::Command::new(cmd)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
            }
            child.wait()
        })
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn copy_text_to_clipboard(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }

    // 1. Try Wayland wl-copy first
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        && try_copy_cmd("wl-copy", &["--trim-newline"], text)
    {
        return Ok(());
    }

    // 2. Try X11 xclip
    if try_copy_cmd("xclip", &["-selection", "clipboard", "-i"], text) {
        return Ok(());
    }

    // 3. Fallback to X11 xsel
    if try_copy_cmd("xsel", &["--clipboard", "--input"], text) {
        return Ok(());
    }

    // 4. Terminal-native clipboard copy OSC 52 fallback
    let b64 = base64_encode(text.as_bytes());
    use std::io::Write;
    let mut stdout = std::io::stdout();
    stdout
        .write_all(format!("\x1b]52;c;{}\x1b\\", b64).as_bytes())
        .map_err(|e| e.to_string())?;
    stdout.flush().map_err(|e| e.to_string())?;

    Ok(())
}

pub fn render_mouse_selection(f: &mut Frame, state: &mut AppState) {
    if let (Some(start), Some(end)) = (state.selection_start, state.selection_end) {
        let buf = f.buffer_mut();
        let width = buf.area().width;
        let height = buf.area().height;

        let (col1, row1) = start;
        let (col2, row2) = end;

        let (min_r, max_r) = (row1.min(row2), row1.max(row2));

        // 1. Draw Highlight
        for y in min_r..=max_r {
            if y >= height {
                continue;
            }
            let (c_start, c_end) = if row1 == row2 {
                (col1.min(col2), col1.max(col2))
            } else if row1 < row2 {
                if y == row1 {
                    (col1, width.saturating_sub(1))
                } else if y == row2 {
                    (0, col2)
                } else {
                    (0, width.saturating_sub(1))
                }
            } else {
                if y == row2 {
                    (col2, width.saturating_sub(1))
                } else if y == row1 {
                    (0, col1)
                } else {
                    (0, width.saturating_sub(1))
                }
            };
            for x in c_start..=c_end {
                if x < width {
                    if let Some(cell) = buf.cell_mut((x, y)) {
                        cell.set_bg(Color::Rgb(0, 120, 215));
                        cell.set_fg(Color::White);
                    }
                }
            }
        }

        // 2. Perform Copy on Release
        if state.selection_pending_copy {
            let mut selected_text = String::new();
            let mut current_row: Option<u16> = None;
            let mut current_line = String::new();

            for y in min_r..=max_r {
                if y >= height {
                    continue;
                }
                let (c_start, c_end) = if row1 == row2 {
                    (col1.min(col2), col1.max(col2))
                } else if row1 < row2 {
                    if y == row1 {
                        (col1, width.saturating_sub(1))
                    } else if y == row2 {
                        (0, col2)
                    } else {
                        (0, width.saturating_sub(1))
                    }
                } else {
                    if y == row2 {
                        (col2, width.saturating_sub(1))
                    } else if y == row1 {
                        (0, col1)
                    } else {
                        (0, width.saturating_sub(1))
                    }
                };

                if current_row != Some(y) {
                    if current_row.is_some() {
                        selected_text.push_str(current_line.trim_end());
                        selected_text.push('\n');
                        current_line.clear();
                    }
                    current_row = Some(y);
                }

                for x in c_start..=c_end {
                    if x < width {
                        if let Some(cell) = buf.cell_mut((x, y)) {
                            current_line.push_str(cell.symbol());
                        }
                    }
                }
            }
            if !current_line.is_empty() {
                selected_text.push_str(current_line.trim_end());
            }

            if !selected_text.is_empty() {
                match copy_text_to_clipboard(&selected_text) {
                    Ok(()) => {
                        state.copied_toast = Some("✓ text copied to clipboard".to_string());
                    }
                    Err(e) => {
                        state.copied_toast = Some(format!("✗ copy failed: {}", e));
                    }
                }
                state.copied_toast_until = Some(Instant::now() + Duration::from_millis(2200));
            }
            state.selection_start = None;
            state.selection_end = None;
            state.selection_pending_copy = false;
        }
    }
}

pub fn render_copied_toast(
    f: &mut Frame,
    state: &mut AppState,
    target_rect: ratatui::layout::Rect,
    accent: Color,
    fg: Color,
    bg: Color,
) {
    if let Some(ref msg) = state.copied_toast {
        let now = Instant::now();
        if let Some(until) = state.copied_toast_until {
            if now > until {
                state.copied_toast = None;
                return;
            }
        } else {
            state.copied_toast = None;
            return;
        }

        let content = msg.as_str();
        let toast_w = (content.len() as u16 + 4).clamp(22, 55);
        let toast_h = 3u16;

        let toast_x = target_rect.x + (target_rect.width.saturating_sub(toast_w)) / 2;
        let toast_y = (target_rect.y + target_rect.height)
            .saturating_sub(toast_h + 1)
            .max(target_rect.y + 1);

        let toast_rect = ratatui::layout::Rect {
            x: toast_x,
            y: toast_y,
            width: toast_w.min(target_rect.width.saturating_sub(2)),
            height: toast_h.min(target_rect.height.saturating_sub(2)),
        };

        use ratatui::widgets::{Block, Borders, Paragraph};
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(ratatui::style::Style::default().fg(accent))
            .style(ratatui::style::Style::default().bg(bg));

        let p = Paragraph::new(ratatui::text::Span::styled(
            content,
            ratatui::style::Style::default().fg(fg),
        ))
        .block(block)
        .alignment(ratatui::layout::Alignment::Center);

        f.render_widget(ratatui::widgets::Clear, toast_rect);
        f.render_widget(p, toast_rect);
    }
}

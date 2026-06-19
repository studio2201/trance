use crate::app::AppState;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

pub fn handle_mouse(
    app: &mut AppState,
    mouse_event: MouseEvent,
    term_width: u16,
    term_height: u16,
) {
    // Calculate layout regions matching draw_ui
    // chunks[0]: Header (Length 3)
    // chunks[1]: Body (Min 5)
    // chunks[2]: Footer (Length 3)
    let body_y_start = 3;
    let body_y_end = term_height.saturating_sub(3);

    // Body horizontal split: Left 45%, Right 55%
    let left_width = ((term_width as u32) * 45) / 100;
    let left_width_u16 = left_width as u16;

    match mouse_event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Text selection start
            app.selection_start = Some((mouse_event.column, mouse_event.row));
            app.selection_end = Some((mouse_event.column, mouse_event.row));
            app.selection_pending_copy = false;
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if app.selection_start.is_some() {
                app.selection_end = Some((mouse_event.column, mouse_event.row));
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if let (Some(start), Some(end)) = (app.selection_start, app.selection_end) {
                let dx = (start.0 as i32 - end.0 as i32).abs();
                let dy = (start.1 as i32 - end.1 as i32).abs();

                if dx > 1 || dy > 0 {
                    // It was a drag operation, copy the selection
                    app.selection_pending_copy = true;
                } else {
                    // It was a simple click: handle selection logic or toggle controls
                    let col = start.0;
                    let row = start.1;

                    // Did user click on the screensavers list (Left Box)?
                    // List block boundary: x: 0..left_width_u16, y: body_y_start..body_y_end
                    if col < left_width_u16 && row > body_y_start && row < body_y_end {
                        // Offset by 1 for list borders
                        let clicked_row = row.saturating_sub(body_y_start + 1) as usize;
                        if clicked_row < app.savers.len() {
                            app.selected_idx = clicked_row;
                        }
                    }

                    app.selection_start = None;
                    app.selection_end = None;
                }
            }
        }
        MouseEventKind::ScrollUp => {
            // Scroll screensaver list up
            if app.selected_idx > 0 {
                app.selected_idx -= 1;
            } else {
                app.selected_idx = app.savers.len() - 1;
            }
        }
        MouseEventKind::ScrollDown => {
            // Scroll screensaver list down
            if app.selected_idx < app.savers.len() - 1 {
                app.selected_idx += 1;
            } else {
                app.selected_idx = 0;
            }
        }
        _ => {}
    }
}

use crate::app::AppState;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
};
use std::env;
use std::fs;

pub mod clipboard;
pub mod helpers;

pub fn draw_ui(f: &mut ratatui::Frame, state: &mut AppState) {
    let size = f.area();
    let is_dark = trance_runner::toolkit::sys_info::query_dark_mode();
    state.dark_mode = is_dark;
    state.accent_color = AppState::get_accent_by_index(state.theme_idx, is_dark);

    let theme_palette = trance_runner::core::screen_palette::ScreenPalette::from_system(
        state.accent_color,
        is_dark,
    );
    let accent = Color::Rgb(
        theme_palette.accent.0,
        theme_palette.accent.1,
        theme_palette.accent.2,
    );
    let dim = Color::Rgb(
        theme_palette.dim.0,
        theme_palette.dim.1,
        theme_palette.dim.2,
    );
    let bg = Color::Rgb(theme_palette.bg.0, theme_palette.bg.1, theme_palette.bg.2);
    let fg = Color::Rgb(theme_palette.fg.0, theme_palette.fg.1, theme_palette.fg.2);

    // Set page background
    f.render_widget(Block::default().style(Style::default().bg(bg)), size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Body
            Constraint::Length(3), // Footer
        ])
        .split(size);

    // 1. Draw Header
    let sys_info = trance_runner::toolkit::sys_info::get_system_info();
    let username = env::var("USER").unwrap_or_else(|_| "user".to_string());
    let hostname = fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "localhost".to_string());

    let mut header_title = vec![
        ratatui::text::Span::styled(" ", Style::default()),
        ratatui::text::Span::styled("t", Style::default().fg(fg).add_modifier(Modifier::BOLD)),
        ratatui::text::Span::styled(
            "(r)",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ),
        ratatui::text::Span::styled("ance", Style::default().fg(fg).add_modifier(Modifier::BOLD)),
        ratatui::text::Span::styled(" | ", Style::default().fg(dim)),
        ratatui::text::Span::styled(
            format!("v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::Rgb(128, 128, 128)),
        ),
        ratatui::text::Span::styled(" | ", Style::default().fg(dim)),
    ];
    header_title.extend(helpers::parse_quote_spans(
        helpers::get_quote(state.quote_idx),
        accent,
        fg,
    ));
    header_title.push(ratatui::text::Span::styled(" ", Style::default()));

    let header_block = Block::default()
        .title(ratatui::text::Line::from(header_title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(dim))
        .border_type(BorderType::Rounded);

    let header_line = ratatui::text::Line::from(vec![
        ratatui::text::Span::styled(
            format!(" {}@{}", username, hostname),
            Style::default().fg(Color::Rgb(128, 128, 128)),
        ),
        ratatui::text::Span::styled(" │ ", Style::default().fg(Color::Rgb(128, 128, 128))),
        ratatui::text::Span::styled(
            sys_info.os.clone(),
            Style::default().fg(Color::Rgb(128, 128, 128)),
        ),
        ratatui::text::Span::styled(" │ ", Style::default().fg(Color::Rgb(128, 128, 128))),
        ratatui::text::Span::styled(
            sys_info.kernel.clone(),
            Style::default().fg(Color::Rgb(128, 128, 128)),
        ),
    ]);

    let header = Paragraph::new(header_line).block(header_block);
    f.render_widget(header, chunks[0]);

    // 2. Draw Body (Split Left / Right)
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[1]);

    // Left Box: Screensaver List
    let items: Vec<ListItem> = state
        .savers
        .iter()
        .enumerate()
        .map(|(idx, name)| {
            let is_hovered = idx == state.selected_idx;
            let is_selected = state.active_saver.as_ref() == Some(name);

            if is_hovered {
                ListItem::new(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(
                        " >>  ",
                        Style::default().fg(accent).add_modifier(Modifier::BOLD),
                    ),
                    ratatui::text::Span::styled(
                        name.to_lowercase(),
                        Style::default().fg(fg).add_modifier(Modifier::BOLD),
                    ),
                ]))
            } else if is_selected {
                ListItem::new(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled("  *  ", Style::default().fg(accent)),
                    ratatui::text::Span::styled((*name).to_string(), Style::default().fg(accent)),
                ]))
            } else {
                ListItem::new(ratatui::text::Line::from(vec![
                    ratatui::text::Span::styled(
                        "     ",
                        Style::default().fg(Color::Rgb(128, 128, 128)),
                    ),
                    ratatui::text::Span::styled(
                        (*name).to_string(),
                        Style::default().fg(Color::Rgb(128, 128, 128)),
                    ),
                ]))
            }
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" available screensavers ")
            .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(accent))
            .border_type(BorderType::Rounded),
    );
    f.render_widget(list, body_chunks[0]);

    // Right Box Split Vertically: Detail on top, Settings on bottom
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(body_chunks[1]);

    // Detail Panel
    let highlighted_saver = &state.savers[state.selected_idx];
    let description = helpers::get_screensaver_description(highlighted_saver);
    let detail_paragraph = Paragraph::new(description)
        .block(
            Block::default()
                .title(format!(" screensaver: {} ", highlighted_saver))
                .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(dim))
                .border_type(BorderType::Rounded),
        )
        .style(Style::default().fg(Color::Rgb(128, 128, 128)))
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(detail_paragraph, right_chunks[0]);

    // Settings Panel
    let daemon_status_text = if state.daemon_running {
        "enabled"
    } else {
        "disabled"
    };

    let settings_block = Block::default()
        .title(" system settings & daemon ")
        .title_style(Style::default().fg(accent).add_modifier(Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(dim))
        .border_type(BorderType::Rounded);

    let settings_spans = ratatui::text::Text::from(vec![
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("  daemon status   :  "),
            ratatui::text::Span::styled(
                daemon_status_text,
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
        ]),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("  idle activation :  "),
            ratatui::text::Span::styled(
                if state.idle_enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
        ]),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("  idle timeout    :  "),
            ratatui::text::Span::styled(
                format!("{} minutes", state.idle_timeout_mins),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
        ]),
        ratatui::text::Line::from(vec![
            ratatui::text::Span::raw("  idle method     :  "),
            ratatui::text::Span::styled(
                "systemd-logind",
                Style::default().fg(Color::Rgb(128, 128, 128)),
            ),
        ]),
    ]);

    let settings_paragraph = Paragraph::new(settings_spans)
        .block(settings_block)
        .style(Style::default().fg(Color::Rgb(128, 128, 128)));
    f.render_widget(settings_paragraph, right_chunks[1]);

    // 3. Draw Footer
    let footer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(dim))
        .border_type(BorderType::Rounded)
        .title(ratatui::text::Span::styled(
            " shortcuts ",
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));

    let shortcuts = vec![
        "(q)uit",
        "(p)review",
        "(s)elect",
        "(d)aemon",
        "(i)dle",
        "(m)ore time",
        "(l)ess time",
        "(t)heme",
    ];

    let mut footer_spans =
        helpers::parse_shortcut_spans(&shortcuts, accent, Color::Rgb(128, 128, 128), dim);

    let shortcuts_len: usize =
        shortcuts.iter().map(|s| s.len()).sum::<usize>() + (shortcuts.len() - 1) * 3;
    let status = &state.status_message;
    let pad_len = (chunks[2].width as usize).saturating_sub(shortcuts_len + status.len() + 5);
    if pad_len > 0 {
        footer_spans.push(ratatui::text::Span::raw(" ".repeat(pad_len)));
        footer_spans.push(ratatui::text::Span::styled(
            status.clone(),
            Style::default().fg(accent),
        ));
    }

    let footer = Paragraph::new(ratatui::text::Line::from(footer_spans))
        .block(footer_block)
        .style(Style::default().fg(Color::Rgb(128, 128, 128)));
    f.render_widget(footer, chunks[2]);

    // Draw particle overlays
    if !state.particles.is_empty() {
        let area = f.area();
        let buf = f.buffer_mut();
        for p in &state.particles {
            let px = p.x.round() as u16;
            let py = p.y.round() as u16;
            if px < area.width && py < area.height {
                if let Some(cell) = buf.cell_mut((px, py)) {
                    cell.set_char(p.char);
                    let color = match p.color_offset % 4 {
                        0 => accent,
                        1 => dim,
                        2 => Color::Yellow,
                        _ => fg,
                    };
                    cell.set_fg(color);
                    if p.color_offset % 3 == 0 {
                        cell.set_style(Style::default().fg(color).add_modifier(Modifier::BOLD));
                    } else {
                        cell.set_style(Style::default().fg(color));
                    }
                }
            }
        }
    }

    // Render selection highlight & handle copy checks
    clipboard::render_mouse_selection(f, state);

    // Draw toast notification overlay
    clipboard::render_copied_toast(f, state, body_chunks[0], accent, fg, bg);
}

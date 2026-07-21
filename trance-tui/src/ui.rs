// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 crateria

use super::app::{ActivePane, App};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style, Stylize},
    text::Line,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

pub fn display_saver_name(raw: &str) -> String {
    if raw.eq_ignore_ascii_case("random") || raw == "Random selection" {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let mut cap = true;
    for ch in raw.chars() {
        if ch == '-' || ch == '_' {
            out.push(' ');
            cap = true;
            continue;
        }
        if cap {
            out.extend(ch.to_uppercase());
            cap = false;
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    out
}

pub fn render_ui(f: &mut ratatui::Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(f.area());

    let battery_status = if app.on_battery {
        " [Battery · 30 FPS]".yellow().bold()
    } else {
        "".into()
    };
    let daemon_hint = if app.daemon_running {
        " · daemon live".green()
    } else {
        " · daemon stopped (Space on Daemon starts + enables)".dark_gray()
    };
    let title = Line::from(vec![
        " Trance Screensaver ".cyan().bold(),
        battery_status,
        daemon_hint,
    ]);
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let title_paragraph = Paragraph::new(title).block(title_block);
    f.render_widget(title_paragraph, chunks[0]);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(chunks[1]);

    let mut settings_list = [
        format!(
            "Daemon Service:       {}",
            if app.daemon_running {
                "RUNNING (auto-start on)"
            } else {
                "STOPPED"
            }
        ),
        format!(
            "Idle Activation:      {}",
            if app.idle_enabled { "ON" } else { "OFF" }
        ),
        format!("Idle Timeout:         {} min", app.idle_timeout_mins),
        format!("Render Scale:         {:.0}%", app.render_scale * 100.0),
        format!(
            "FPS Overlay:          {}",
            if app.show_fps_overlay { "ON" } else { "OFF" }
        ),
    ];

    let mut settings_items = Vec::new();
    for (idx, text) in settings_list.iter_mut().enumerate() {
        let mut style = Style::default();
        if app.active_pane == ActivePane::Settings && app.selected_setting_idx == idx {
            style = style.fg(Color::Yellow).add_modifier(Modifier::BOLD);
            *text = format!("> {text}");
        } else {
            *text = format!("  {text}");
        }
        settings_items.push(ListItem::new(text.clone()).style(style));
    }

    let settings_block = Block::default()
        .title(" Settings ")
        .borders(Borders::ALL)
        .border_style(
            Style::default().fg(if app.active_pane == ActivePane::Settings {
                Color::Yellow
            } else {
                Color::DarkGray
            }),
        );
    let settings_widget = List::new(settings_items).block(settings_block);
    f.render_widget(settings_widget, columns[0]);

    let mut saver_items = vec![ListItem::new(format!(
        "{} Random",
        if app.active_saver == "Random" {
            "*"
        } else {
            " "
        }
    ))];

    for s in &app.screensavers {
        let prefix = if app.active_saver == *s { "*" } else { " " };
        saver_items.push(ListItem::new(format!("{prefix} {}", display_saver_name(s))));
    }

    let mut state = ListState::default();
    state.select(Some(app.selected_saver_idx));

    let savers_block = Block::default()
        .title(" Screensavers ")
        .borders(Borders::ALL)
        .border_style(
            Style::default().fg(if app.active_pane == ActivePane::Screensavers {
                Color::Yellow
            } else {
                Color::DarkGray
            }),
        );
    let savers_widget = List::new(saver_items)
        .block(savers_block)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    f.render_stateful_widget(savers_widget, columns[1], &mut state);

    let help_text = match app.active_pane {
        ActivePane::Settings => {
            " [Tab] Pane | [Space/Enter] Toggle | [←/→] Timeout/Scale | [q] Quit  ·  Daemon on = enable --now"
        }
        ActivePane::Screensavers => {
            " [Tab] Pane | [↑/↓] Navigate | [Enter] Set Active | [p] Preview | [q] Quit"
        }
    };
    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let help_paragraph = Paragraph::new(help_text).block(help_block);
    f.render_widget(help_paragraph, chunks[2]);
}

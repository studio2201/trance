// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 IdleScreen

mod app;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use anyhow::Result;
use app::{ActivePane, App};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use ui::render_ui;

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error running TUI: {err}");
    }
    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(250);

    loop {
        terminal.draw(|f| render_ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)?
            && let Event::Key(key) = event::read()?
        {
            if key.modifiers.contains(KeyModifiers::CONTROL)
                && (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('d'))
            {
                return Ok(());
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Tab => {
                    app.active_pane = match app.active_pane {
                        ActivePane::Settings => ActivePane::Screensavers,
                        ActivePane::Screensavers => ActivePane::Settings,
                    };
                }
                KeyCode::Up => match app.active_pane {
                    ActivePane::Settings => {
                        if app.selected_setting_idx > 0 {
                            app.selected_setting_idx -= 1;
                        }
                    }
                    ActivePane::Screensavers => {
                        if app.selected_saver_idx > 0 {
                            app.selected_saver_idx -= 1;
                        }
                    }
                },
                KeyCode::Down => match app.active_pane {
                    ActivePane::Settings => {
                        if app.selected_setting_idx < 4 {
                            app.selected_setting_idx += 1;
                        }
                    }
                    ActivePane::Screensavers => {
                        if app.selected_saver_idx <= app.screensavers.len() {
                            app.selected_saver_idx += 1;
                        }
                    }
                },
                KeyCode::Left => {
                    if app.active_pane == ActivePane::Settings {
                        match app.selected_setting_idx {
                            2 => app.adjust_timeout(-1),
                            3 => app.adjust_scale(-0.05),
                            _ => {}
                        }
                    }
                }
                KeyCode::Right => {
                    if app.active_pane == ActivePane::Settings {
                        match app.selected_setting_idx {
                            2 => app.adjust_timeout(1),
                            3 => app.adjust_scale(0.05),
                            _ => {}
                        }
                    }
                }
                KeyCode::Char(' ') | KeyCode::Enter => match app.active_pane {
                    ActivePane::Settings => match app.selected_setting_idx {
                        0 => app.toggle_daemon(),
                        1 => app.toggle_idle(),
                        2 => app.adjust_timeout(5),
                        3 => app.adjust_scale(0.1),
                        4 => app.toggle_fps(),
                        _ => {}
                    },
                    ActivePane::Screensavers => {
                        if key.code == KeyCode::Enter {
                            app.select_saver();
                        }
                    }
                },
                KeyCode::Char('p') => {
                    if app.active_pane == ActivePane::Screensavers {
                        app.preview_saver();
                    }
                }
                KeyCode::Char('r') => {
                    app.refresh_state();
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.refresh_state();
            last_tick = Instant::now();
        }
    }
}

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::{Duration, Instant};
pub mod mouse;
pub mod particles;
pub mod state;
pub use state::AppState;

pub fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Terminal initialization
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Panic hook to restore terminal if app panics
    std::panic::set_hook(Box::new(|panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture,
            crossterm::cursor::Show
        );
        eprintln!("application panicked: {:?}", panic_info);
    }));

    let mut state = AppState::new();
    let mut last_tick = Instant::now();
    let mut last_status_decay = Instant::now();

    while !state.should_quit {
        state.check_daemon_running();

        // 2. Render TUI
        terminal.draw(|f| {
            crate::ui::draw_ui(f, &mut state);
        })?;

        // 3. Event Polling
        let dynamic_tick_rate = if !state.particles.is_empty() {
            Duration::from_millis(50)
        } else {
            Duration::from_millis(250)
        };
        let timeout = dynamic_tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_default();
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == event::KeyEventKind::Press {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => {
                                state.should_quit = true;
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                let size = terminal.size().unwrap_or_default();
                                let width = size.width as f64;
                                let height = size.height as f64;
                                particles::toggle_trance_mode(&mut state, width, height);
                            }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.selected_idx > 0 {
                                    state.selected_idx -= 1;
                                } else {
                                    state.selected_idx = state.savers.len() - 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.selected_idx < state.savers.len() - 1 {
                                    state.selected_idx += 1;
                                } else {
                                    state.selected_idx = 0;
                                }
                            }
                            KeyCode::Char('p') | KeyCode::Char('P') => {
                                let name = &state.savers[state.selected_idx];
                                state.status_message =
                                    format!("previewing screensaver: {}...", name);
                                state.status_ttl_sec = 5;
                                // Temporarily leave raw mode / alternate screen to launch screensaver preview
                                let _ = disable_raw_mode();
                                let _ = execute!(
                                    terminal.backend_mut(),
                                    LeaveAlternateScreen,
                                    DisableMouseCapture
                                );

                                let _res = crate::start_screensaver(name);

                                // Re-enter raw mode / alternate screen
                                let _ = enable_raw_mode();
                                let _ = execute!(
                                    terminal.backend_mut(),
                                    EnterAlternateScreen,
                                    EnableMouseCapture
                                );
                                let _ = terminal.clear();
                            }
                            KeyCode::Char('i') | KeyCode::Char('I') => {
                                state.idle_enabled = !state.idle_enabled;
                                let _ = state.save_config();
                                let m = if state.idle_enabled {
                                    "enabled"
                                } else {
                                    "disabled"
                                };
                                state.status_message = format!("idle activation {m}");
                                state.status_ttl_sec = 5;
                            }
                            KeyCode::Char('m') | KeyCode::Char('M') => {
                                state.idle_timeout_mins =
                                    state.idle_timeout_mins.saturating_add(1).min(60);
                                let _ = state.save_config();
                                state.status_message = format!(
                                    "timeout increased to {} mins",
                                    state.idle_timeout_mins
                                );
                                state.status_ttl_sec = 5;
                            }
                            KeyCode::Char('l') | KeyCode::Char('L') => {
                                state.idle_timeout_mins =
                                    state.idle_timeout_mins.saturating_sub(1).max(1);
                                let _ = state.save_config();
                                state.status_message = format!(
                                    "timeout decreased to {} mins",
                                    state.idle_timeout_mins
                                );
                                state.status_ttl_sec = 5;
                            }
                            KeyCode::Char('d') | KeyCode::Char('D') => {
                                if state.daemon_running {
                                    if let Some(p) = get_daemon_pid_path()
                                        .and_then(|p| std::fs::read_to_string(p).ok())
                                    {
                                        if let Ok(pid) = p.trim().parse::<i32>() {
                                            unsafe {
                                                libc::kill(pid, libc::SIGTERM);
                                            }
                                        }
                                    }
                                    state.status_message =
                                        "requested daemon process to stop.".to_string();
                                } else {
                                    let current_exe =
                                        std::env::current_exe().unwrap_or_else(|_| "trance".into());
                                    let _ = std::process::Command::new(current_exe)
                                        .arg("daemon")
                                        .stdout(std::process::Stdio::null())
                                        .stderr(std::process::Stdio::null())
                                        .spawn();
                                    state.status_message =
                                        "background daemon process spawned.".to_string();
                                }
                                state.status_ttl_sec = 5;
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                let name = state.savers[state.selected_idx].clone();
                                if state.active_saver.as_ref() == Some(&name) {
                                    state.active_saver = None;
                                    state.status_message =
                                        "cleared active screensaver (now random)".to_string();
                                } else {
                                    state.active_saver = Some(name.clone());
                                    state.status_message = format!("selected {} for use", name);
                                }
                                let _ = state.save_config();
                                state.status_ttl_sec = 5;
                            }
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                state.theme_idx = (state.theme_idx + 1) % 5;
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                                    .unwrap_or_default();
                                state.quote_idx = now.as_nanos() as usize;
                                let is_dark =
                                    trance_runner::toolkit::sys_info::query_dark_mode();
                                state.dark_mode = is_dark;
                                state.accent_color =
                                    AppState::get_accent_by_index(state.theme_idx, is_dark);
                                let name =
                                    ["navy", "violet", "teal", "fuchsia", "rust"][state.theme_idx];
                                let _ = state.save_config();
                                state.status_message = format!("theme changed to {}", name);
                                state.status_ttl_sec = 5;
                            }
                            _ => {}
                        }
                    }
                }
                Event::Mouse(mouse_event) => {
                    let size = terminal.size().unwrap_or_default();
                    mouse::handle_mouse(&mut state, mouse_event, size.width, size.height);
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= dynamic_tick_rate {
            last_tick = Instant::now();

            let size = terminal.size().unwrap_or_default();
            let width = size.width as f64;
            let height = size.height as f64;
            particles::update_particles(&mut state, width, height);
        }

        if last_status_decay.elapsed() >= Duration::from_millis(250) {
            last_status_decay = Instant::now();
            if state.status_ttl_sec > 0 {
                state.status_ttl_sec = state.status_ttl_sec.saturating_sub(1);
                if state.status_ttl_sec == 0 {
                    state.status_message = "".to_string();
                }
            }
        }
    }

    // 4. Terminal shutdown restoration
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        crossterm::cursor::Show
    )?;
    terminal.show_cursor()?;

    Ok(())
}

fn get_daemon_pid_path() -> Option<std::path::PathBuf> {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !runtime_dir.is_empty() {
            return Some(std::path::PathBuf::from(runtime_dir).join("trance-daemon.pid"));
        }
    }
    Some(std::env::temp_dir().join("trance-daemon.pid"))
}

//! Cross-platform screensaver runtime host.
//! Vendored from `runner::trance_runner`.

use crate::core::TerminalCell;
use crate::core::screensaver::Screensaver;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[path = "args.rs"]
mod args;
#[path = "platform_helpers.rs"]
mod platform_helpers;
#[path = "renderer.rs"]
mod renderer;
#[path = "terminal_guard.rs"]
mod terminal_guard;

pub use args::{Mode, parse_args, print_usage};

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_signal(_sig: libc::c_int) {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

/// Run the screensaver with the given effect.
pub fn run_main<S: Screensaver + 'static>(mut saver: S, name: &str) {
    let mode = parse_args();
    match mode {
        Mode::Run => {
            let code = run_fullscreen(&mut saver);
            std::process::exit(code as i32);
        }
        Mode::Configure => {
            eprintln!("({name}) configuration dialog: not yet implemented.");
            std::process::exit(0);
        }
        Mode::Preview => {
            #[cfg(target_os = "windows")]
            {
                let code = run_preview_stub(&mut saver);
                std::process::exit(code as i32);
            }
            #[cfg(not(target_os = "windows"))]
            {
                let code = run_fullscreen(&mut saver);
                std::process::exit(code as i32);
            }
        }
        Mode::ShowUsage => {
            print_usage(name);
            std::process::exit(0);
        }
    }
}

#[cfg(target_os = "windows")]
fn run_preview_stub(_saver: &mut dyn Screensaver) -> isize {
    eprintln!("Windows preview mode is not supported in console mode.");
    0
}

/// Loads a screensaver plugin dynamic library and runs it fullscreen.
pub fn run_plugin_fullscreen(plugin_path: &str) -> Result<isize, Box<dyn std::error::Error>> {
    use trance_api::ScreensaverInstance;

    unsafe {
        let lib = libloading::Library::new(plugin_path)?;
        let create_fn: libloading::Symbol<unsafe extern "C" fn() -> *mut ScreensaverInstance> =
            lib.get(b"create_screensaver")?;
        let destroy_fn: libloading::Symbol<unsafe extern "C" fn(*mut ScreensaverInstance)> =
            lib.get(b"destroy_screensaver")?;

        let raw_ptr = create_fn();
        if raw_ptr.is_null() {
            return Err("failed to create screensaver instance (null pointer)".into());
        }

        struct PluginGuard {
            ptr: *mut ScreensaverInstance,
            destroy: unsafe extern "C" fn(*mut ScreensaverInstance),
            _lib: libloading::Library,
        }

        impl Drop for PluginGuard {
            fn drop(&mut self) {
                unsafe {
                    (self.destroy)(self.ptr);
                }
            }
        }

        let guard = PluginGuard {
            ptr: raw_ptr,
            destroy: *destroy_fn,
            _lib: lib,
        };

        let exit_code = run_fullscreen(&mut *(*guard.ptr).inner);
        Ok(exit_code)
    }
}

// ---------------------------------------------------------------------------
// Common Fullscreen Animation Loop
// ---------------------------------------------------------------------------

fn run_fullscreen(saver: &mut dyn Screensaver) -> isize {
    // Note: Classic xscreensaver embedding support (XSCREENSAVER_WINDOW + xterm -into)
    // has been removed. The user runs via trance-daemon fullscreen xterm or ubermetroid
    // previews, which do not require X11 embedding. Raw terminal + ANSI works on
    // Wayland (via xterm under XWayland or native terminals).

    #[cfg(not(target_os = "windows"))]
    unsafe {
        libc::signal(
            libc::SIGINT,
            handle_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGTERM,
            handle_signal as *const () as libc::sighandler_t,
        );
    }

    let _raw_mode = match terminal_guard::RawTerminalGuard::enable() {
        Some(g) => g,
        None => {
            eprintln!("screensaver: could not enter raw mode; aborting.");
            return 1;
        }
    };
    let (mut cols, mut rows) = platform_helpers::get_terminal_size();
    saver.init(cols, rows);

    let mut r = renderer::Renderer::new(cols, rows);
    let mut grid = vec![TerminalCell::default(); cols * rows];
    let mut last_frame = std::time::Instant::now();

    // Query actual monitor refresh rate
    let target_fps = platform_helpers::get_monitor_refresh_rate().max(10);
    let mut frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);

    // Decoupled physics accumulator settings
    let mut physics_hz = 120.0;
    let mut physics_duration = Duration::from_secs_f32(1.0 / physics_hz);
    let mut physics_accumulator = Duration::ZERO;

    let mut frame_count = 0;
    let mut frame_time_sum = 0.0;
    let mut calibrated = false;

    let mut initial_mouse_pos = None;
    let start_time = std::time::Instant::now();

    loop {
        if SHUTDOWN.load(Ordering::Relaxed) {
            break;
        }

        let is_startup = start_time.elapsed() < Duration::from_millis(500);

        if platform_helpers::check_keypress() {
            if is_startup {
                #[cfg(not(target_os = "windows"))]
                unsafe {
                    libc::tcflush(libc::STDIN_FILENO, libc::TCIFLUSH);
                }
            } else {
                break;
            }
        }

        // Prevent instant exit on startup due to initial mouse shake or clicks
        if !is_startup && platform_helpers::check_mouse_activity(&mut initial_mouse_pos) {
            break;
        }

        // Handle terminal resize dynamically
        let (new_cols, new_rows) = platform_helpers::get_terminal_size();
        if new_cols != cols || new_rows != rows {
            cols = new_cols;
            rows = new_rows;
            grid = vec![TerminalCell::default(); cols * rows];
            saver.init(cols, rows);
            r = renderer::Renderer::new(cols, rows);
        }

        let now = std::time::Instant::now();
        let dt = now.duration_since(last_frame);
        last_frame = now;

        saver.update_frame_time(dt);

        if !calibrated {
            frame_count += 1;
            frame_time_sum += dt.as_secs_f32();
            if frame_count == 20 {
                let avg_frame_time = frame_time_sum / 20.0;
                if avg_frame_time > 0.001 {
                    let measured_fps = 1.0 / avg_frame_time;
                    let mut snapped_fps = measured_fps;
                    for &std_rate in &[30.0, 60.0, 75.0, 90.0, 120.0, 144.0, 240.0] {
                        if (measured_fps - std_rate).abs() < 4.0 {
                            snapped_fps = std_rate;
                            break;
                        }
                    }
                    frame_duration = Duration::from_secs_f32(1.0 / snapped_fps);

                    let k = ((120.0 / snapped_fps).ceil() as u32).max(1);
                    physics_hz = snapped_fps * k as f32;
                    physics_duration = Duration::from_secs_f32(1.0 / physics_hz);
                }
                calibrated = true;
            }
        }

        // Accumulate physics time step
        physics_accumulator += dt;
        if physics_accumulator > Duration::from_millis(100) {
            physics_accumulator = Duration::from_millis(100); // Prevent spiral of death
        }

        // Run updates at target aligned physics tick rate
        while physics_accumulator >= physics_duration {
            saver.update(physics_duration, cols, rows);
            physics_accumulator -= physics_duration;
        }

        saver.draw(&mut grid, cols, rows);
        r.render_grid(&grid, cols, rows, saver.has_scanlines());

        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    #[cfg(not(target_os = "windows"))]
    unsafe {
        libc::tcflush(libc::STDIN_FILENO, libc::TCIFLUSH);
    }

    0
}

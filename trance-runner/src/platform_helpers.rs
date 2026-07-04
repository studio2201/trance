//! Linux terminal/screen helpers used by the screensaver runner.
//! (Windows support has been removed; this is now Linux-only for the supported
//! distros: Debian family, Red Hat family, Gentoo, Arch.)

// ---------------------------------------------------------------------------
// Monitor refresh rate
// ---------------------------------------------------------------------------

pub fn get_monitor_refresh_rate() -> u32 {
    120 // Reasonable default for terminal-based screensavers on Linux
}

// ---------------------------------------------------------------------------
// Terminal size
// ---------------------------------------------------------------------------

pub fn get_terminal_size() -> (usize, usize) {
    if let Ok((cols, rows)) = crossterm::terminal::size() {
        (cols as usize, rows as usize)
    } else {
        (80, 24)
    }
}

// ---------------------------------------------------------------------------
// Mouse activity
// ---------------------------------------------------------------------------

pub fn check_mouse_activity(_initial_pos: &mut Option<(i32, i32)>) -> bool {
    false // Mouse activity detection not needed for these fullscreen terminal savers
}

// ---------------------------------------------------------------------------
// Keypress detection
// ---------------------------------------------------------------------------

pub fn check_keypress() -> bool {
    use crossterm::event::{self, Event};
    if let Ok(true) = event::poll(std::time::Duration::from_secs(0))
        && let Ok(Event::Key(_)) = event::read()
    {
        return true;
    }
    false
}

// ---------------------------------------------------------------------------
// Misc
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub fn command_exists(cmd: &str) -> bool {
    // Avoid shell metachar injection. Try "which" first (common on Linux),
    // then fallback to attempting to invoke the command.
    if let Ok(status) = std::process::Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        && status.success()
    {
        return true;
    }
    // Fallback: if the command can at least be started (even if it exits non-zero),
    // consider it present. (Used for "xterm" in fullscreen launch paths.)
    std::process::Command::new(cmd)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

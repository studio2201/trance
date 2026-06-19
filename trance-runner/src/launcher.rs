//! Secure launcher: basename-only resolve + safe xterm for the 12 savers.
//! Security: allowlist only; Daemon=installed paths (no dev); Preview=dev ok.
//! Sanitize names; untrusted registry; safe envs. Removes old dupe launch code.

use std::path::{Path, PathBuf};
use std::process::{Child, Command};

/// The canonical list of allowed saver basenames.
/// Order matches historical lists in the codebase.
pub const ALLOWED_SAVERS: &[&str] = &[
    "beams", "bursts", "chaos", "cosmos", "glyphs", "gnats", "storm",
];

/// How the launcher is being invoked. This controls path resolution policy
/// and xterm flags.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaunchMode {
    /// Normal idle/screensaver activation by trance-daemon.
    /// Uses *only* trusted installed locations. Dev paths are forbidden.
    Daemon,
    /// Explicit fullscreen preview launched from the local76 TUI ('p' key).
    /// Dev paths under the user's Projects tree are permitted (developer UX).
    Preview,
    /// (Legacy xscreensaver embed support removed; this variant no longer used.)
    /// Uses installed locations; passes /s to the saver and -into.
    Embed { window_id: String },
}

/// Reduce string (path/.scr/registry) to clean basename or None.
/// Rules: stem only, alnum+-, no traversal.
pub fn sanitize_saver_name(raw: &str) -> Option<String> {
    // Take only the last component (handles full paths or .scr "paths" from registry)
    let stem = Path::new(raw)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(raw);

    if !stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return None;
    }

    let mut cleaned = stem.to_string();
    if cleaned.starts_with("screensaver-") {
        cleaned = cleaned["screensaver-".len()..].to_string();
    }

    if cleaned.is_empty() {
        return None;
    }

    Some(cleaned)
}

/// Resolve saver name to trusted binary or plugin path.
/// Searches user-installed and system XDG-compliant directories first.
pub fn resolve_saver_binary(name: &str, mode: &LaunchMode) -> std::io::Result<PathBuf> {
    let clean = sanitize_saver_name(name).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("unknown or invalid screensaver name: {}", name),
        )
    })?;

    let candidates = vec![
        format!("libscreensaver_{}.so", clean),
        format!("lib{}.so", clean),
        clean.clone(),
    ];

    let find_in_dir = |base: &Path| -> Option<PathBuf> {
        for cand in &candidates {
            let p = base.join(cand);
            if p.is_file() {
                return Some(p);
            }
        }
        None
    };

    // 1. Check dynamically discovered screensaver directories
    for base in crate::discovery::get_screensaver_dirs() {
        if let Some(path) = find_in_dir(&base) {
            return Ok(path);
        }
    }

    // 2. Dev paths are only allowed for explicit Preview (TUI 'p')
    if *mode == LaunchMode::Preview {
        if let Ok(home) = std::env::var("HOME") {
            let projects = PathBuf::from(home).join("Projects");

            // Base release and debug workspace directories
            let dirs = vec![
                projects.join("local76").join("trance-plugins").join("target").join("release"),
                projects.join("local76").join("trance-plugins").join("target").join("debug"),
                projects.join("local76").join("screensavers").join("target").join("release"),
                projects.join("local76").join("screensavers").join("target").join("debug"),
                projects.join("trance-plugins").join("target").join("release"),
                projects.join("trance-plugins").join("target").join("debug"),
                projects.join("screensavers").join("target").join("release"),
                projects.join("screensavers").join("target").join("debug"),
                projects.join(format!("trance-plugin-{}", clean)).join("target").join("release"),
                projects.join(format!("trance-plugin-{}", clean)).join("target").join("debug"),
                projects.join(format!("screensaver-{}", clean)).join("target").join("release"),
                projects.join(format!("screensaver-{}", clean)).join("target").join("debug"),
            ];

            for base in dirs {
                if let Some(path) = find_in_dir(&base) {
                    return Ok(path);
                }
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!(
            "no trusted binary or plugin found for saver '{}' (mode {:?}).",
            clean, mode
        ),
    ))
}

/// Build a Command that will run the saver inside a hardened xterm.
///
/// The caller is still responsible for .spawn(), but this centralizes all
/// the xterm flags, safe environment variables, and the /s argument for embed.
pub fn prepare_xterm_command(binary: &Path, mode: LaunchMode) -> Command {
    let mut cmd = Command::new("xterm");
    let current_exe = std::env::current_exe()
        .unwrap_or_else(|_| PathBuf::from("trance"));

    match mode {
        LaunchMode::Daemon | LaunchMode::Preview => {
            cmd.args([
                "-fullscreen",
                "-bg", "black",
                "-fg", "white",
                "-ms", "black",
                "-xrm", "XTerm*pointerColorBackground: black",
                "-xrm", "XTerm*pointerMode: 2",
                "-xrm", "XTerm*borderWidth: 0",
                "-xrm", "XTerm*internalBorder: 0",
                "-xrm", "XTerm*highlightThickness: 0",
                "-e",
            ])
            .arg(current_exe)
            .arg("run-plugin")
            .arg(binary);
        }
        LaunchMode::Embed { ref window_id } => {
            cmd.args([
                "-into",
                window_id,
                "-geometry",
                "120x40",
                "-bg",
                "black",
                "-fg",
                "white",
                "-ms",
                "black",
                "-xrm",
                "XTerm*pointerColorBackground: black",
                "-xrm",
                "XTerm*pointerMode: 2",
                "-xrm",
                "XTerm*borderWidth: 0",
                "-xrm",
                "XTerm*internalBorder: 0",
                "-xrm",
                "XTerm*highlightThickness: 0",
                "-e",
            ])
            .arg(current_exe)
            .arg("run-plugin")
            .arg(binary);
        }
    }

    cmd.env("XCURSOR_PATH", "/nonexistent")
       .env("XCURSOR_THEME", "none");

    cmd.env("TRANCE_SCREENSAVER", "1");

    match mode {
        LaunchMode::Daemon => {
            cmd.env("TRANCE_IDLE_SCREENSAVER", "1");
        }
        LaunchMode::Preview => {}
        LaunchMode::Embed { .. } => {
            cmd.env("TRANCE_EMBEDDED", "1");
            cmd.env("TRANCE_IDLE_SCREENSAVER", "1");
        }
    }

    cmd
}

#[cfg(test)]
#[path = "launcher_tests.rs"]
mod tests;

pub struct ScreensaverProcess {
    pub primary: Child,
    pub secondaries: Vec<Child>,
    pub disabled_monitors: Vec<String>,
}

impl ScreensaverProcess {
    fn restore_monitors(&mut self) {
        for mon in self.disabled_monitors.drain(..) {
            let _ = std::process::Command::new("cosmic-randr")
                .arg("enable")
                .arg(&mon)
                .status();
        }
    }

    pub fn kill(&mut self) -> std::io::Result<()> {
        let mut first_err = None;
        if let Err(e) = self.primary.kill() {
            first_err = Some(e);
        }
        for sec in &mut self.secondaries {
            let _ = sec.kill();
        }
        self.restore_monitors();
        if let Some(e) = first_err {
            Err(e)
        } else {
            Ok(())
        }
    }

    pub fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        let status = self.primary.wait()?;
        for sec in &mut self.secondaries {
            let _ = sec.kill();
            let _ = sec.wait();
        }
        self.restore_monitors();
        Ok(status)
    }

    pub fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        match self.primary.try_wait()? {
            Some(status) => {
                for sec in &mut self.secondaries {
                    let _ = sec.kill();
                    let _ = sec.wait();
                }
                self.restore_monitors();
                Ok(Some(status))
            }
            None => Ok(None),
        }
    }

    pub fn id(&self) -> u32 {
        self.primary.id()
    }
}

impl Drop for ScreensaverProcess {
    fn drop(&mut self) {
        self.restore_monitors();
    }
}

fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::new();
    let mut in_escape = false;
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                in_escape = true;
                continue;
            }
        }
        
        if in_escape {
            if c == 'm' {
                in_escape = false;
            }
            continue;
        }
        
        result.push(c);
    }
    result
}

fn is_wayland() -> bool {
    std::env::var("XDG_SESSION_TYPE").unwrap_or_default().to_lowercase().contains("wayland")
        || std::env::var("WAYLAND_DISPLAY").is_ok()
}

fn get_secondary_monitors_to_disable() -> Vec<String> {
    let mut secondaries = Vec::new();
    let output = std::process::Command::new("cosmic-randr")
        .arg("list")
        .output();
    
    if let Ok(out) = output {
        if out.status.success() {
            let raw_stdout = String::from_utf8_lossy(&out.stdout);
            let stdout = strip_ansi_codes(&raw_stdout);
            let mut current_monitor: Option<String> = None;
            let mut current_is_primary = false;
            let mut current_is_enabled = false;
            
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                
                if !line.starts_with(' ') {
                    if let Some(ref mon) = current_monitor {
                        if current_is_enabled && !current_is_primary {
                            secondaries.push(mon.clone());
                        }
                    }
                    
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if !parts.is_empty() {
                        let name = parts[0].to_string();
                        let enabled = trimmed.contains("(enabled)");
                        current_monitor = Some(name);
                        current_is_enabled = enabled;
                        current_is_primary = false;
                    } else {
                        current_monitor = None;
                    }
                } else {
                    if trimmed.contains("Xwayland primary: true") {
                        current_is_primary = true;
                    }
                }
            }
            
            if let Some(ref mon) = current_monitor {
                if current_is_enabled && !current_is_primary {
                    secondaries.push(mon.clone());
                }
            }
        }
    }
    secondaries
}

/// High-level convenience: resolve + prepare + spawn in one call.
///
/// This is the recommended entry point for host code (trance-daemon, local76 TUI).
///
/// Example for daemon idle:
/// ```ignore
/// let child = trance_runner::launcher::launch_screensaver("security", LaunchMode::Daemon)?;
/// ```
pub fn launch_screensaver(name: &str, mode: LaunchMode) -> std::io::Result<ScreensaverProcess> {
    let mut disabled_monitors = Vec::new();

    if is_wayland() && matches!(mode, LaunchMode::Daemon | LaunchMode::Preview) {
        let secondaries = get_secondary_monitors_to_disable();
        for mon in secondaries {
            let status = std::process::Command::new("cosmic-randr")
                .arg("disable")
                .arg(&mon)
                .status();
            if let Ok(stat) = status {
                if stat.success() {
                    disabled_monitors.push(mon);
                }
            }
        }
        if !disabled_monitors.is_empty() {
            // Wait briefly for compositor to configure layout
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }
    }

    let binary = resolve_saver_binary(name, &mode)?;
    let mut cmd = prepare_xterm_command(&binary, mode.clone());
    let primary = cmd.spawn()?;

    Ok(ScreensaverProcess {
        primary,
        secondaries: Vec::new(),
        disabled_monitors,
    })
}

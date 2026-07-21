// SPDX-License-Identifier: MIT

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use trance_dbus::TranceClient;

#[derive(Debug)]
pub struct CheckResult {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

pub fn chk(name: &'static str, passed: bool, detail: impl Into<String>) -> CheckResult {
    CheckResult {
        name,
        passed,
        detail: detail.into(),
    }
}

pub fn check_wayland() -> CheckResult {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(val) if !val.is_empty() => {
            println!(" [✔] Environment: Wayland compositor detected ($WAYLAND_DISPLAY={val}).");
            chk("Environment", true, format!("WAYLAND_DISPLAY={val}"))
        }
        _ => {
            println!(" [✗] Environment: WAYLAND_DISPLAY is not set!");
            println!("     -> Trance requires a Wayland session (e.g., Pop!_OS COSMIC).");
            chk("Environment", false, "WAYLAND_DISPLAY missing")
        }
    }
}

pub fn check_dbus() -> CheckResult {
    if let Ok(client) = TranceClient::connect() {
        match client.get_status() {
            Ok(status) => {
                println!(" [✔] D-Bus Service: Connected to org.crateria.Trance via D-Bus.");
                println!(
                    "     -> Status: idle_enabled={}, timeout={}m, active_saver='{}'",
                    status.idle_enabled, status.idle_timeout_mins, status.active_saver
                );
                chk(
                    "D-Bus Service",
                    true,
                    format!("idle_enabled={}", status.idle_enabled),
                )
            }
            Err(e) => {
                println!(" [✗] D-Bus Service: Connected to D-Bus, but GetStatus() failed: {e}");
                chk("D-Bus Service", false, format!("GetStatus error: {e}"))
            }
        }
    } else {
        println!(" [✗] D-Bus Service: Could not connect to org.crateria.Trance via D-Bus.");
        println!(
            "     -> Fix: Ensure trance-daemon is running (systemctl --user start trance-daemon)."
        );
        chk("D-Bus Service", false, "cannot connect")
    }
}

pub fn check_systemd_service() -> CheckResult {
    let output = Command::new("systemctl")
        .args(["--user", "is-active", "trance-daemon.service"])
        .output();

    match output {
        Ok(out) => {
            let status = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if status == "active" {
                println!(" [✔] Systemd Service: trance-daemon.service is active.");
                chk("Systemd Service", true, "active")
            } else {
                println!(" [✗] Systemd Service: trance-daemon.service is '{status}'.");
                println!("     -> Fix: Run: systemctl --user start trance-daemon");
                chk("Systemd Service", false, status)
            }
        }
        Err(e) => {
            println!(" [✗] Systemd Service: Could not check systemctl: {e}");
            chk("Systemd Service", false, format!("systemctl error: {e}"))
        }
    }
}

pub fn check_running_pid() -> CheckResult {
    let pid_path = pid_file_path();
    let dbus_ok = TranceClient::connect().is_ok();

    if pid_path.exists() {
        if let Ok(content) = fs::read_to_string(&pid_path) {
            let pid_str = content.trim();
            if let Ok(pid) = pid_str.parse::<i32>() {
                if unsafe { libc::kill(pid, 0) } == 0 {
                    println!(" [✔] Process Status: Daemon process is running (PID {pid}).");
                    return chk("Process Status", true, format!("PID {pid} running"));
                } else {
                    println!(
                        " [✗] Process Status: PID file exists ({pid}), but process is NOT running (stale PID)."
                    );
                    println!(
                        "     -> Fix: Remove stale PID file or restart daemon: systemctl --user restart trance-daemon"
                    );
                    return chk("Process Status", false, format!("stale PID {pid}"));
                }
            }
        }
        println!(
            " [!] Process Status: PID file exists at '{}', but content is unreadable.",
            pid_path.display()
        );
        chk("Process Status", true, "pid file unreadable")
    } else if dbus_ok {
        println!(" [!] Process Status: Connected to daemon via D-Bus, but PID file is missing.");
        chk("Process Status", true, "missing pid but d-bus ok")
    } else {
        println!(" [✗] Process Status: Daemon PID file does not exist.");
        chk("Process Status", false, "missing pid file")
    }
}

pub fn check_config_parses() -> CheckResult {
    match get_config_path() {
        Some(path) if path.exists() => match fs::read_to_string(&path) {
            Ok(content) => {
                println!(" [✔] Configuration: File found at '{}'.", path.display());
                let n = content.lines().count();
                println!("     -> Health check: Configuration file read successfully ({n} lines).");
                chk("Configuration", true, format!("{n} lines"))
            }
            Err(e) => {
                println!(
                    " [✗] Configuration: Found at '{}' but unreadable: {e}",
                    path.display()
                );
                chk("Configuration", false, format!("unreadable: {e}"))
            }
        },
        Some(path) => {
            println!(" [!] Configuration: File not found. Default settings will be used.");
            println!(
                "     -> Note: Config file path is expected at '{}'.",
                path.display()
            );
            chk("Configuration", true, "default settings")
        }
        None => {
            println!(" [✗] Configuration: Could not resolve home directory path for settings.");
            chk("Configuration", false, "cannot resolve home")
        }
    }
}

pub fn check_fonts() -> CheckResult {
    if font_check_via_fc_list() {
        println!(" [✔] System Fonts: Monospace font is installed.");
        chk("System Fonts", true, "monospace font found")
    } else {
        println!(" [✗] System Fonts: Monospace font not found on system!");
        println!("     -> Fix: Please install fonts-dejavu-core or a system monospace font.");
        chk("System Fonts", false, "monospace font missing")
    }
}

pub fn check_package_install() -> CheckResult {
    if let Ok(o) = Command::new("rpm")
        .args([
            "-q",
            "trance",
            "--qf",
            "%{NAME}-%{VERSION}-%{RELEASE}.%{ARCH}",
        ])
        .output()
        && o.status.success()
    {
        let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
        println!(" [✔] Package (RPM): {ver}");
        println!("     -> Upgrade with: sudo dnf update");
        return chk("Package", true, ver);
    }
    if let Ok(o) = Command::new("dpkg-query")
        .args(["-W", "-f=${Package} ${Version}", "trance"])
        .output()
        && o.status.success()
    {
        let ver = String::from_utf8_lossy(&o.stdout).trim().to_string();
        println!(" [✔] Package (DEB): {ver}");
        println!("     -> Upgrade with: sudo apt update && sudo apt upgrade");
        return chk("Package", true, ver);
    }
    println!(" [!] Package: trance not found via RPM or dpkg.");
    println!("     -> Install from the crateria apt/dnf repository.");
    chk("Package", true, "not a system package")
}

fn pid_file_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("trance-daemon.pid")
    } else {
        std::env::temp_dir().join("trance-daemon.pid")
    }
}

fn get_config_path() -> Option<PathBuf> {
    if let Some(xdg_config) = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
    {
        return Some(PathBuf::from(xdg_config).join("trance").join("config.yaml"));
    }
    let home = std::env::var("HOME").ok()?;
    Some(
        PathBuf::from(home)
            .join(".config")
            .join("trance")
            .join("config.yaml"),
    )
}

fn font_check_via_fc_list() -> bool {
    let output = Command::new("fc-list").args([":mono"]).output();
    match output {
        Ok(out) => out.status.success() && !out.stdout.is_empty(),
        Err(_) => {
            let common_dirs = ["/usr/share/fonts", "/usr/local/share/fonts"];
            common_dirs.iter().any(|dir| PathBuf::from(dir).exists())
        }
    }
}

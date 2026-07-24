// SPDX-License-Identifier: MIT

//! D-Bus, systemd, and process checks for doctor.

use super::doctor_checks::{CheckResult, chk};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use trance_dbus::TranceClient;

pub fn check_dbus() -> CheckResult {
    if let Ok(client) = TranceClient::connect() {
        match client.get_status() {
            Ok(status) => chk(
                "D-Bus Service",
                true,
                format!(
                    "connected idle_enabled={} timeout={}m saver='{}'",
                    status.idle_enabled, status.idle_timeout_mins, status.active_saver
                ),
            ),
            Err(e) => chk("D-Bus Service", false, format!("GetStatus error: {e}")),
        }
    } else {
        chk(
            "D-Bus Service",
            false,
            "cannot connect to io.github.ubermetroid.trance; start trance-daemon",
        )
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
                chk("Systemd Service", true, "active")
            } else {
                chk(
                    "Systemd Service",
                    false,
                    format!("status '{status}'; run systemctl --user start trance-daemon"),
                )
            }
        }
        Err(e) => chk("Systemd Service", false, format!("systemctl error: {e}")),
    }
}

pub fn check_running_pid() -> CheckResult {
    let pid_path = pid_file_path();
    let dbus_ok = TranceClient::connect().is_ok();

    if pid_path.exists() {
        if let Ok(content) = fs::read_to_string(&pid_path) {
            let pid_str = content.trim();
            if let Ok(pid) = pid_str.parse::<i32>() {
                // SAFETY: kill(pid, 0) only checks process existence; no signal delivered.
                if unsafe { libc::kill(pid, 0) } == 0 {
                    return chk("Process Status", true, format!("PID {pid} running"));
                }
                return chk("Process Status", false, format!("stale PID {pid}"));
            }
        }
        chk("Process Status", true, "pid file unreadable")
    } else if dbus_ok {
        chk("Process Status", true, "missing pid but d-bus ok")
    } else {
        chk("Process Status", false, "missing pid file")
    }
}

fn pid_file_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join("trance-daemon.pid")
    } else {
        std::env::temp_dir().join("trance-daemon.pid")
    }
}

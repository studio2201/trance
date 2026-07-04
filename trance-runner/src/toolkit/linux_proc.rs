//! Linux-specific power supply and theme helper queries.

use crate::toolkit::platform::PowerStatus;

pub fn query_dark_mode_linux() -> bool {
    // 1. Try reading standard GTK-3.0 / GTK-4.0 settings files directly
    for path in &[
        ".config/gtk-4.0/settings.ini",
        ".config/gtk-3.0/settings.ini",
    ] {
        if let Some(home) = std::env::var_os("HOME") {
            let full_path = std::path::Path::new(&home).join(path);
            if let Ok(content) = std::fs::read_to_string(full_path) {
                let lower = content.to_lowercase();
                for line in lower.lines() {
                    if line.contains("gtk-application-prefer-dark-theme") && line.contains("true") {
                        return true;
                    }
                    if line.contains("gtk-theme-name") && line.contains("dark") {
                        return true;
                    }
                }
            }
        }
    }

    // 2. Fallback to gsettings if ini files are missing or inconclusive
    if let Ok(output) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if s.contains("prefer-dark") {
            return true;
        }
    }
    if let Ok(output) = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
        .output()
    {
        let s = String::from_utf8_lossy(&output.stdout).to_lowercase();
        if s.contains("dark") {
            return true;
        }
    }
    true
}

pub fn query_power_status_linux() -> Option<PowerStatus> {
    let mut ac_online = true;
    let mut has_ac = false;
    let mut battery_percent: Option<u8> = None;
    if let Ok(entries) = std::fs::read_dir("/sys/class/power_supply") {
        for entry in entries.take(64).flatten() {
            let path = entry.path();
            if let Ok(ty_str) = std::fs::read_to_string(path.join("type")) {
                match ty_str.trim() {
                    "Mains" => {
                        if let Ok(online_str) = std::fs::read_to_string(path.join("online")) {
                            let online = online_str.trim() == "1";
                            if !has_ac {
                                ac_online = online;
                                has_ac = true;
                            } else {
                                ac_online = ac_online || online;
                            }
                        }
                    }
                    "Battery" => {
                        if let Ok(cap_str) = std::fs::read_to_string(path.join("capacity"))
                            && let Ok(pct) = cap_str.trim().parse::<u8>()
                        {
                            battery_percent = Some(pct);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    battery_percent.map(|pct| PowerStatus {
        ac_online: if has_ac { ac_online } else { true },
        battery_percent: pct,
    })
}

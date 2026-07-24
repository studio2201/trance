// SPDX-License-Identifier: MIT

//! Environment and protocol soft-checks for doctor.

use super::doctor_checks::{CheckResult, chk};

pub fn check_wayland() -> CheckResult {
    match std::env::var("WAYLAND_DISPLAY") {
        Ok(val) if !val.is_empty() => chk("Environment", true, format!("WAYLAND_DISPLAY={val}")),
        _ => chk(
            "Environment",
            false,
            "WAYLAND_DISPLAY missing; IdleScreen needs a Wayland session",
        ),
    }
}

/// Soft protocol/DE hints (pass with guidance when DE is unusual).
pub fn check_protocol_hints() -> CheckResult {
    let de = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
    if !wayland {
        return chk(
            "Protocols",
            false,
            "WAYLAND_DISPLAY unset; need ext-idle-notify-v1 and zwlr_layer_shell_v1",
        );
    }
    let known = [
        "cosmic", "hyprland", "sway", "niri", "river", "wayfire", "kde", "plasma",
    ];
    let lower = de.to_ascii_lowercase();
    let friendly = known.iter().any(|k| lower.contains(k));
    let de_label = if de.is_empty() {
        "unknown"
    } else {
        de.as_str()
    };
    if friendly || de.is_empty() {
        chk(
            "Protocols",
            true,
            format!("WAYLAND_DISPLAY set; DE='{de_label}' (need idle-notify + layer-shell)"),
        )
    } else {
        chk(
            "Protocols",
            true,
            format!(
                "WAYLAND_DISPLAY set; DE='{de_label}' may lack layer-shell/idle-notify — see docs/BOUNDARIES.md"
            ),
        )
    }
}

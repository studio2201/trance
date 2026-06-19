// SPDX-License-Identifier: MIT

use std::process::Command;

pub struct IdleStatus {
    pub is_idle: bool,
    pub idle_since_micros: u64,
}

/// Query systemd-logind for idle details.
/// First tries the self-session endpoint; falls back to the system manager endpoint.
pub fn query_logind_idle() -> Option<IdleStatus> {
    // 1. Try session/self (Session interface)
    if let (Some(is_idle), Some(idle_since)) = (
        get_property::<bool>("session/self", "org.freedesktop.login1.Session", "IdleHint"),
        get_property::<u64>(
            "session/self",
            "org.freedesktop.login1.Session",
            "IdleSinceHint",
        ),
    ) {
        return Some(IdleStatus {
            is_idle,
            idle_since_micros: idle_since,
        });
    }

    // 2. Fallback to manager level (Manager interface)
    if let (Some(is_idle), Some(idle_since)) = (
        get_property::<bool>("", "org.freedesktop.login1.Manager", "IdleHint"),
        get_property::<u64>("", "org.freedesktop.login1.Manager", "IdleSinceHint"),
    ) {
        return Some(IdleStatus {
            is_idle,
            idle_since_micros: idle_since,
        });
    }

    None
}

fn get_property_raw(sub_path: &str, interface: &str, property: &str) -> Option<BoolOrU64> {
    let path = if sub_path.is_empty() {
        "/org/freedesktop/login1".to_string()
    } else {
        format!("/org/freedesktop/login1/{}", sub_path)
    };

    let output = Command::new("busctl")
        .args([
            "get-property",
            "org.freedesktop.login1",
            &path,
            interface,
            property,
        ])
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.split_whitespace().collect();
        if parts.len() >= 2 {
            match parts[0] {
                "b" => return Some(BoolOrU64::Bool(parts[1] == "true")),
                "t" => {
                    if let Ok(val) = parts[1].parse::<u64>() {
                        return Some(BoolOrU64::U64(val));
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[derive(Debug, Clone, Copy)]
enum BoolOrU64 {
    Bool(bool),
    U64(u64),
}

impl BoolOrU64 {
    fn bool(self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(b),
            _ => None,
        }
    }
    fn u64(self) -> Option<u64> {
        match self {
            Self::U64(u) => Some(u),
            _ => None,
        }
    }
}

// Implement trait to allow destructuring Option<BoolOrU64> directly
trait ExtractProperty<T> {
    fn extract(self) -> Option<T>;
}

impl ExtractProperty<bool> for Option<BoolOrU64> {
    fn extract(self) -> Option<bool> {
        self.and_then(|x| x.bool())
    }
}

impl ExtractProperty<u64> for Option<BoolOrU64> {
    fn extract(self) -> Option<u64> {
        self.and_then(|x| x.u64())
    }
}

fn get_property<T>(sub_path: &str, interface: &str, property: &str) -> Option<T>
where
    Option<BoolOrU64>: ExtractProperty<T>,
{
    let raw: Option<BoolOrU64> = get_property_raw(sub_path, interface, property);
    raw.extract()
}

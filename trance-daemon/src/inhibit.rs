// SPDX-License-Identifier: MIT

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use zbus::names::UniqueName;

#[derive(Debug, Clone)]
pub struct Inhibitor {
    pub cookie: u32,
    #[allow(dead_code)]
    pub application_name: String,
    #[allow(dead_code)]
    pub reason: String,
    pub client: UniqueName<'static>,
}

#[derive(Debug)]
pub struct InhibitorState {
    inhibitors: Mutex<Vec<Inhibitor>>,
    last_cookie: AtomicU32,
    logind_cache: Mutex<(bool, std::time::Instant)>,
}

impl InhibitorState {
    pub fn new() -> Self {
        Self {
            inhibitors: Mutex::new(Vec::new()),
            last_cookie: AtomicU32::new(0),
            logind_cache: Mutex::new((
                false,
                std::time::Instant::now()
                    .checked_sub(std::time::Duration::from_secs(5))
                    .unwrap_or_else(std::time::Instant::now),
            )),
        }
    }

    pub fn is_inhibited(&self) -> bool {
        if !self.inhibitors.lock().unwrap().is_empty() {
            return true;
        }

        let mut cache = self.logind_cache.lock().unwrap();
        if cache.1.elapsed() >= std::time::Duration::from_secs(2) {
            cache.0 = check_logind_inhibited();
            cache.1 = std::time::Instant::now();
        }
        cache.0
    }

    pub fn add(
        &self,
        application_name: String,
        reason: String,
        client: UniqueName<'static>,
    ) -> u32 {
        let cookie = self.last_cookie.fetch_add(1, Ordering::Relaxed) + 1;
        let mut inhibitors = self.inhibitors.lock().unwrap();
        inhibitors.push(Inhibitor {
            cookie,
            application_name,
            reason,
            client,
        });
        cookie
    }

    /// Remove an inhibitor only when `cookie` belongs to `client`.
    pub fn remove_for_client(&self, cookie: u32, client: &UniqueName<'_>) -> bool {
        let mut inhibitors = self.inhibitors.lock().unwrap();
        if let Some(index) = inhibitors
            .iter()
            .position(|entry| entry.cookie == cookie && entry.client == *client)
        {
            inhibitors.remove(index);
            true
        } else {
            false
        }
    }

    pub fn remove_client(&self, client: &UniqueName<'_>) {
        let mut inhibitors = self.inhibitors.lock().unwrap();
        inhibitors.retain(|entry| entry.client != *client);
    }
}

#[cfg(target_os = "linux")]
type LogindInhibitorInfo = (String, String, String, String, u32, u32);

#[cfg(target_os = "linux")]
fn check_logind_inhibited() -> bool {
    let Ok(conn) = zbus::blocking::Connection::system() else {
        return false;
    };
    let Ok(reply) = conn.call_method(
        Some("org.freedesktop.login1"),
        "/org/freedesktop/login1",
        Some("org.freedesktop.login1.Manager"),
        "ListInhibitors",
        &(),
    ) else {
        return false;
    };
    let Ok(inhibitors): Result<Vec<LogindInhibitorInfo>, _> = reply.body().deserialize() else {
        return false;
    };
    for (what, _, _, _, _, _) in inhibitors {
        if what.split(':').any(|w| w == "idle") {
            return true;
        }
    }
    false
}

#[cfg(not(target_os = "linux"))]
fn check_logind_inhibited() -> bool {
    false
}

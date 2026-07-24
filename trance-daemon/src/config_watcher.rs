// SPDX-License-Identifier: MIT

//! Hot-reload `~/.config/trance/config.yaml` without a Tokio runtime.
//!
//! The daemon main path is not Tokio-driven; only the D-Bus thread owns a
//! runtime. Keep the notify watcher on a plain OS thread so startup cannot
//! panic with "there is no reactor running".

use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::config::DaemonConfig;
use crate::controller::DaemonController;

pub fn start_config_watcher(controller: Arc<DaemonController>) {
    let Some(path) = DaemonConfig::get_config_path() else {
        return;
    };
    let Some(parent_dir) = path.parent() else {
        return;
    };

    if !parent_dir.exists() {
        let _ = std::fs::create_dir_all(parent_dir);
    }

    let controller_clone = controller.clone();
    let target_path = path.clone();

    let mut watcher = match notify::recommended_watcher(move |res: Result<Event, _>| {
        if let Ok(event) = res
            && matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_))
            && event.paths.iter().any(|p| p == &target_path)
        {
            tracing::info!("Config file modified on disk; hot-reloading settings...");
            let fresh = DaemonConfig::load();
            let _ = controller_clone.mutate_config(|cfg| {
                *cfg = fresh;
            });
        }
    }) {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!("Failed to initialize config file watcher: {e}");
            return;
        }
    };

    if let Err(e) = watcher.watch(parent_dir, RecursiveMode::NonRecursive) {
        tracing::warn!("Failed to watch config directory {:?}: {e}", parent_dir);
        return;
    }

    // Retain the watcher for process lifetime without requiring a Tokio runtime.
    thread::Builder::new()
        .name("trance-config-watch".into())
        .spawn(move || {
            let _watcher = watcher;
            loop {
                thread::sleep(Duration::from_hours(1));
            }
        })
        .ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_config_watcher_does_not_panic_without_tokio() {
        // No Tokio Handle in this thread — must not abort.
        let controller = Arc::new(DaemonController::new(DaemonConfig::default()));
        start_config_watcher(controller);
    }
}

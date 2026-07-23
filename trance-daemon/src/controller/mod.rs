// SPDX-License-Identifier: MIT

//! Daemon control plane: configuration mutations, live status, and command queue.

mod commands;
mod status;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use trance_dbus::DaemonStatus;
use trance_runner::launcher::{LaunchMode, resolve_saver_binary};

use crate::config::DaemonConfig;
use crate::inhibit::InhibitorState;

#[derive(Debug, Clone)]
pub enum DaemonCommand {
    Enable,
    Disable,
    SetTimeout(u32),
    SetSaver(Option<String>),
    SetShowFpsOverlay(bool),
    SetRenderScale(Option<f32>),
    Preview(String),
    StopPresentation,
}

pub struct DaemonController {
    pub config: Arc<Mutex<DaemonConfig>>,
    pub status: Arc<Mutex<DaemonStatus>>,
    pub command_tx: mpsc::Sender<DaemonCommand>,
    pub command_rx: Mutex<mpsc::Receiver<DaemonCommand>>,
    pub inhibitors: Arc<InhibitorState>,
    pub session_locked: Arc<AtomicBool>,
    pub shutdown: Arc<AtomicBool>,
    pub status_dirty: Arc<AtomicBool>,
    pub status_emit_tx: Mutex<Option<mpsc::Sender<DaemonStatus>>>,
    dbus_connection: Mutex<Option<zbus::Connection>>,
}

impl DaemonController {
    pub fn new(initial_config: DaemonConfig) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let status = DaemonStatus {
            running: true,
            idle_enabled: initial_config.idle_enabled,
            idle_timeout_mins: initial_config.idle_timeout_mins,
            active_saver: initial_config.active_saver.clone().unwrap_or_default(),
            gpu_enabled: false,
            show_fps_overlay: initial_config.show_fps_overlay,
            render_scale: initial_config
                .render_scale
                .map(|s| s.to_string())
                .unwrap_or_default(),
            ..DaemonStatus::default()
        };

        Self {
            config: Arc::new(Mutex::new(initial_config)),
            status: Arc::new(Mutex::new(status)),
            command_tx,
            command_rx: Mutex::new(command_rx),
            inhibitors: Arc::new(InhibitorState::new()),
            session_locked: Arc::new(AtomicBool::new(false)),
            shutdown: Arc::new(AtomicBool::new(false)),
            status_dirty: Arc::new(AtomicBool::new(true)),
            status_emit_tx: Mutex::new(None),
            dbus_connection: Mutex::new(None),
        }
    }

    pub fn set_dbus_connection(&self, connection: zbus::Connection) {
        *self
            .dbus_connection
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(connection);
    }

    pub fn dbus_connection(&self) -> Result<zbus::Connection, String> {
        self.dbus_connection
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or_else(|| "D-Bus connection unavailable".into())
    }

    pub fn publish_status_if_dirty(&self) {
        if !self.take_dirty() {
            return;
        }
        let status = self
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if let Some(sender) = self
            .status_emit_tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
        {
            let _ = sender.send(status);
        }
    }

    pub fn drain_commands(&self) -> Vec<DaemonCommand> {
        let mut commands = Vec::new();
        let receiver = self.command_rx.lock().unwrap_or_else(|e| e.into_inner());
        while let Ok(command) = receiver.try_recv() {
            commands.push(command);
        }
        commands
    }

    pub fn mark_dirty(&self) {
        self.status_dirty.store(true, Ordering::Relaxed);
    }

    pub fn take_dirty(&self) -> bool {
        self.status_dirty.swap(false, Ordering::Relaxed)
    }
}

pub fn installed_savers() -> Vec<String> {
    trance_runner::discovery::detect_screensavers()
        .into_iter()
        .filter(|name| {
            trance_runner::launcher::is_allowed_saver(name)
                && resolve_saver_binary(name, &LaunchMode::Daemon).is_ok()
        })
        .collect()
}

pub const MAIN_LOOP_INTERVAL: Duration = Duration::from_millis(250);

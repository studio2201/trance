// SPDX-License-Identifier: MIT

//! OOP plugin process liveness and crash recovery.

use super::ipc_session::IpcPluginSession;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use trance_ipc::IpcCommand;

impl IpcPluginSession {
    /// True if the OOP plugin child is still running.
    pub fn is_plugin_alive(&mut self) -> bool {
        let Some(child) = self.child.as_mut() else {
            return false;
        };
        match child.try_wait() {
            Ok(None) => true,
            Ok(Some(status)) => {
                if !self.expected_stop.load(Ordering::Relaxed) {
                    tracing::error!(?status, "plugin child exited unexpectedly");
                }
                false
            }
            Err(e) => {
                tracing::error!(%e, "plugin child status query failed");
                false
            }
        }
    }

    /// Tear down and re-spawn the OOP plugin process (crash isolation).
    pub fn recover(&mut self, cols: usize, rows: usize) -> Result<(), String> {
        tracing::warn!(saver = %self.saver_name, "recovering OOP plugin session");
        self.expected_stop.store(true, Ordering::Relaxed);
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.socket = None;
        if let Some(path) = self.socket_path.take() {
            let _ = fs::remove_file(path);
        }
        self.shm = None;
        self.expected_stop = Arc::new(AtomicBool::new(false));
        self.init(cols, rows)
    }
}

impl Drop for IpcPluginSession {
    fn drop(&mut self) {
        self.expected_stop.store(true, Ordering::Relaxed);
        if let Some(ref mut socket) = self.socket {
            let _ = IpcCommand::Stop.write_to(&mut *socket);
        }
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(ref socket_path) = self.socket_path {
            let _ = fs::remove_file(socket_path);
        }
    }
}

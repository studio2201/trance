// SPDX-License-Identifier: MIT

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

use crate::appearance::OverlayAppearance;
use crate::output::{OutputLayout, OutputRegistry};
use crate::overlay::{PresenterCommand, spawn_event_thread};

/// Presents fullscreen Wayland overlays on top of the desktop.
pub struct OverlayPresenter {
    command_tx: Sender<PresenterCommand>,
    visible: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    outputs: OutputRegistry,
    is_alive: Arc<AtomicBool>,
    supports_scaling: Arc<AtomicBool>,
}

impl OverlayPresenter {
    /// Connect to the compositor and prepare the overlay session.
    pub fn new() -> Option<Self> {
        if !Self::is_available() {
            return None;
        }

        let (ready_tx, ready_rx) = mpsc::channel();
        let (command_tx, command_rx) = mpsc::channel();
        let visible = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));
        let outputs = OutputRegistry::new();
        let is_alive = Arc::new(AtomicBool::new(true));
        let supports_scaling = Arc::new(AtomicBool::new(false));

        spawn_event_thread(
            ready_tx,
            command_rx,
            visible.clone(),
            shutdown.clone(),
            outputs.clone(),
            is_alive.clone(),
            supports_scaling.clone(),
        );

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Some(Self {
                command_tx,
                visible,
                shutdown,
                outputs,
                is_alive,
                supports_scaling,
            }),
            _ => None,
        }
    }

    pub fn is_available() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
    }

    pub fn is_visible(&self) -> bool {
        self.visible.load(Ordering::SeqCst)
    }

    /// Returns `true` if the Wayland presentation thread is still running.
    pub fn is_alive(&self) -> bool {
        self.is_alive.load(Ordering::SeqCst)
    }

    /// Returns `true` if the compositor supports wp_viewporter hardware scaling.
    pub fn supports_scaling(&self) -> bool {
        self.supports_scaling.load(Ordering::SeqCst)
    }

    pub fn output_layouts(&self) -> Vec<OutputLayout> {
        self.outputs.layouts()
    }

    pub fn show(&self, appearance: OverlayAppearance) {
        let _ = self
            .command_tx
            .send(PresenterCommand::ShowSolid(appearance));
    }

    pub fn show_screensaver(&self) {
        let _ = self.command_tx.send(PresenterCommand::ShowScreensaver);
    }

    pub fn submit_frame(&self, output_id: u32, width: u32, height: u32, pixels: Vec<u8>) {
        let _ = self.command_tx.send(PresenterCommand::UpdateFrame {
            output_id,
            width,
            height,
            pixels,
        });
    }

    pub fn hide(&self) {
        let _ = self.command_tx.send(PresenterCommand::Hide);
    }
}

impl Drop for OverlayPresenter {
    fn drop(&mut self) {
        let _ = self.command_tx.send(PresenterCommand::Hide);
        self.shutdown.store(true, Ordering::Relaxed);
    }
}

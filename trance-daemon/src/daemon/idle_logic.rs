// SPDX-License-Identifier: MIT

//! Idle-driven screensaver presentation state machine.

use std::sync::Arc;

use wayland_present::OverlayPresenter;

use super::idle_decision::{IdlePolicyInput, PresentationDecision, decide_presentation};
use super::presentation::{
    ActivePresentation, current_time_micros, pick_saver_name, start_presentation, stop_presentation,
};
use crate::config::DaemonConfig;

pub fn update_presentation_state(
    overlay_presenter: &Arc<OverlayPresenter>,
    presentation: &mut ActivePresentation,
    preview_name: &mut Option<String>,
    current_saver: &mut String,
    config: &DaemonConfig,
    system_idle: bool,
    session_locked: bool,
    inhibited: bool,
) {
    let input = IdlePolicyInput {
        is_active: presentation.is_active(),
        surface_visible: overlay_presenter.is_visible(),
        current_saver: current_saver.as_str(),
        preview_name: preview_name.as_deref(),
        idle_enabled: config.idle_enabled,
        system_idle,
        session_locked,
        inhibited,
    };
    let idle_name = pick_saver_name(config, current_time_micros());
    match decide_presentation(input, &idle_name) {
        PresentationDecision::Hold => {}
        PresentationDecision::Stop { clear_preview } => {
            if presentation.is_active() {
                stop_presentation(Some(overlay_presenter), presentation);
                current_saver.clear();
                if !system_idle && preview_name.is_none() {
                    tracing::info!("system activity detected. presentation stopped.");
                }
            }
            if clear_preview {
                *preview_name = None;
            }
        }
        PresentationDecision::Start { name, reason } => {
            if presentation.is_active() && current_saver.as_str() != name.as_str() {
                stop_presentation(Some(overlay_presenter), presentation);
                current_saver.clear();
            }
            if !presentation.is_active() {
                start_presentation(
                    overlay_presenter,
                    presentation,
                    current_saver,
                    name,
                    reason,
                    config,
                );
            }
        }
    }
}

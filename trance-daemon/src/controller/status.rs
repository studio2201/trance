// SPDX-License-Identifier: MIT

use std::sync::atomic::Ordering;

use super::DaemonController;

impl DaemonController {
    #[tracing::instrument(skip_all, fields(system_idle, presentation_active, preview_active, current_saver = %current_saver))]
    pub fn update_live_state(
        &self,
        system_idle: bool,
        presentation_active: bool,
        preview_active: bool,
        current_saver: &str,
    ) {
        // Snapshot config once per tick (avoids double lock+clone in dirty/copy).
        let config = self.config.lock().unwrap().clone();
        let mut status = self.status.lock().unwrap();
        let changed = Self::compute_dirty_fields(
            &mut status,
            &config,
            system_idle,
            presentation_active,
            preview_active,
            current_saver,
            self.session_locked.load(Ordering::Relaxed),
            self.inhibitors.is_inhibited(),
        );
        Self::copy_live_fields(
            &mut status,
            &config,
            system_idle,
            presentation_active,
            preview_active,
            current_saver,
            self.session_locked.load(Ordering::Relaxed),
            self.inhibitors.is_inhibited(),
        );
        if changed {
            self.status_dirty.store(true, Ordering::Relaxed);
        }
    }

    #[tracing::instrument(skip_all, fields(tick_counter))]
    pub fn reload_config_if_due(&self, tick_counter: u32) -> Option<u32> {
        if !tick_counter.is_multiple_of(10) {
            return None;
        }
        let reloaded = crate::config::DaemonConfig::load();
        let mut config = self.config.lock().unwrap();
        let previous_timeout = config.idle_timeout_mins;
        if *config != reloaded {
            *config = reloaded;
            self.mark_dirty();
        }
        if config.idle_timeout_mins != previous_timeout {
            Some(config.idle_timeout_mins)
        } else {
            None
        }
    }

    #[allow(clippy::fn_params_excessive_bools)]
    fn compute_dirty_fields(
        status: &mut trance_dbus::DaemonStatus,
        config: &crate::config::DaemonConfig,
        system_idle: bool,
        presentation_active: bool,
        preview_active: bool,
        current_saver: &str,
        session_locked: bool,
        inhibited: bool,
    ) -> bool {
        let active_saver = config.active_saver.as_deref().unwrap_or("");
        let render_scale = config
            .render_scale
            .map(|s| s.to_string())
            .unwrap_or_default();

        status.system_idle != system_idle
            || status.presentation_active != presentation_active
            || status.preview_active != preview_active
            || status.session_locked != session_locked
            || status.inhibited != inhibited
            || status.idle_enabled != config.idle_enabled
            || status.idle_timeout_mins != config.idle_timeout_mins
            || status.active_saver != active_saver
            || {
                #[allow(deprecated)]
                let gpu_diff = status.gpu_enabled != config.gpu_enabled;
                gpu_diff
            }
            || status.show_fps_overlay != config.show_fps_overlay
            || status.render_scale != render_scale
            || status.current_saver != current_saver
    }

    #[allow(clippy::fn_params_excessive_bools)]
    fn copy_live_fields(
        status: &mut trance_dbus::DaemonStatus,
        config: &crate::config::DaemonConfig,
        system_idle: bool,
        presentation_active: bool,
        preview_active: bool,
        current_saver: &str,
        session_locked: bool,
        inhibited: bool,
    ) {
        status.running = true;
        status.system_idle = system_idle;
        status.presentation_active = presentation_active;
        status.preview_active = preview_active;
        status.session_locked = session_locked;
        status.inhibited = inhibited;
        status.idle_enabled = config.idle_enabled;
        status.idle_timeout_mins = config.idle_timeout_mins;
        status.active_saver = config.active_saver.clone().unwrap_or_default();
        #[allow(deprecated)]
        {
            status.gpu_enabled = config.gpu_enabled;
        }
        status.show_fps_overlay = config.show_fps_overlay;
        status.render_scale = config
            .render_scale
            .map(|s| s.to_string())
            .unwrap_or_default();
        status.current_saver = current_saver.to_string();
    }
}

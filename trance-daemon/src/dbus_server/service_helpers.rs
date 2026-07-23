// SPDX-License-Identifier: MIT

use crate::controller::DaemonController;
use std::sync::Arc;
use trance_dbus::DaemonStatus;

pub async fn authorize_control(
    controller: &Arc<DaemonController>,
    header: &zbus::message::Header<'_>,
) -> zbus::fdo::Result<()> {
    super::auth::require_control_peer(
        &controller
            .dbus_connection()
            .map_err(zbus::fdo::Error::Failed)?,
        header,
    )
    .await
}

#[tracing::instrument(skip(controller), level = "trace")]
pub fn live_status(controller: &Arc<DaemonController>) -> DaemonStatus {
    let mut status = controller
        .status
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    status.session_locked = controller
        .session_locked
        .load(std::sync::atomic::Ordering::Relaxed);
    status.inhibited = controller.inhibitors.is_inhibited();
    status
}

#[tracing::instrument(skip(controller))]
pub fn sync_config_status(controller: &Arc<DaemonController>) {
    let config = controller
        .config
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    {
        let mut status = controller.status.lock().unwrap_or_else(|e| e.into_inner());
        status.idle_enabled = config.idle_enabled;
        status.idle_timeout_mins = config.idle_timeout_mins;
        status.active_saver = config.active_saver.clone().unwrap_or_default();
        #[allow(deprecated)]
        {
            status.gpu_enabled = false;
        }
        status.show_fps_overlay = config.show_fps_overlay;
        status.render_scale = config
            .render_scale
            .map(|s| s.to_string())
            .unwrap_or_default();
    }
    controller.mark_dirty();
    controller.publish_status_if_dirty();
}

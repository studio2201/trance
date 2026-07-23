// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::sync::Arc;

use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

use super::service_helpers::{
    apply_config_command, authorize_control, live_status, sync_config_status,
};
use crate::controller::{DaemonCommand, DaemonController};

pub struct TranceService {
    pub controller: Arc<DaemonController>,
}

#[zbus::interface(name = "io.github.ubermetroid.trance")]
#[allow(deprecated)]
impl TranceService {
    async fn get_status(&self) -> zbus::fdo::Result<HashMap<String, OwnedValue>> {
        Ok(live_status(&self.controller).to_map())
    }

    async fn enable(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        authorize_control(&self.controller, &header).await?;
        apply_config_command(&self.controller, DaemonCommand::Enable, "Enable")
    }

    async fn disable(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        authorize_control(&self.controller, &header).await?;
        if let Err(error) = self.controller.apply_command(DaemonCommand::Disable) {
            tracing::error!(target: "trance_daemon::dbus", "Disable failed: {error:?}");
            return Err(zbus::fdo::Error::Failed(error.to_string()));
        }
        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::StopPresentation);
        sync_config_status(&self.controller);
        Ok(())
    }

    async fn set_timeout(
        &self,
        minutes: u32,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        authorize_control(&self.controller, &header).await?;
        apply_config_command(
            &self.controller,
            DaemonCommand::SetTimeout(minutes),
            "SetTimeout",
        )?;
        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::SetTimeout(minutes));
        Ok(())
    }

    async fn set_saver(
        &self,
        name: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        authorize_control(&self.controller, &header).await?;
        let saver = (!name.is_empty()).then(|| name.to_string());
        apply_config_command(&self.controller, DaemonCommand::SetSaver(saver), "SetSaver")
    }

    async fn list_savers(&self) -> zbus::fdo::Result<Vec<String>> {
        Ok(crate::controller::installed_savers())
    }

    async fn preview(
        &self,
        name: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        authorize_control(&self.controller, &header).await?;
        trance_runner::launcher::sanitize_saver_name(name).ok_or_else(|| {
            zbus::fdo::Error::Failed(format!("unknown or invalid screensaver name: {name}"))
        })?;
        trance_runner::launcher::resolve_saver_binary(
            name,
            &trance_runner::launcher::LaunchMode::Preview,
        )
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;

        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::Preview(name.to_string()));
        self.controller.mark_dirty();
        Ok(())
    }

    async fn stop_preview(
        &self,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        authorize_control(&self.controller, &header).await?;
        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::StopPresentation);
        self.controller.mark_dirty();
        Ok(())
    }

    async fn inhibit(
        &self,
        application: &str,
        reason: &str,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<u32> {
        let sender = header.sender().ok_or_else(|| {
            zbus::fdo::Error::Failed("inhibit request missing D-Bus sender".into())
        })?;
        let cookie = self
            .controller
            .inhibitors
            .add(
                application.to_string(),
                reason.to_string(),
                sender.to_owned(),
            )
            .map_err(|error| zbus::fdo::Error::LimitsExceeded(error.to_string()))?;
        let _ = self
            .controller
            .command_tx
            .send(DaemonCommand::StopPresentation);
        self.controller.mark_dirty();
        Ok(cookie)
    }

    async fn un_inhibit(
        &self,
        cookie: u32,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        let sender = header.sender().ok_or_else(|| {
            zbus::fdo::Error::Failed("un_inhibit request missing D-Bus sender".into())
        })?;
        if !self.controller.inhibitors.remove_for_client(cookie, sender) {
            return Err(zbus::fdo::Error::Failed(format!(
                "unknown inhibit cookie for caller: {cookie}"
            )));
        }
        self.controller.mark_dirty();
        Ok(())
    }

    async fn list_inhibitors(
        &self,
        #[zbus(header)] _header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<Vec<(u32, String, String)>> {
        Ok(self.controller.inhibitors.list())
    }

    /// DEPRECATED (2026) — no-op method retained for D-Bus client compatibility.
    #[deprecated(note = "GPU upscaler removed; this method is a no-op")]
    #[allow(deprecated)]
    async fn set_gpu_enabled(
        &self,
        _enabled: bool,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        authorize_control(&self.controller, &header).await?;
        tracing::warn!(target: "trance_daemon::deprecation", "set_gpu_enabled called; GPU upscaler removed in 2026");
        Ok(())
    }

    async fn set_show_fps_overlay(
        &self,
        enabled: bool,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        authorize_control(&self.controller, &header).await?;
        apply_config_command(
            &self.controller,
            DaemonCommand::SetShowFpsOverlay(enabled),
            "SetShowFpsOverlay",
        )
    }

    async fn set_render_scale(
        &self,
        scale: f64,
        #[zbus(header)] header: zbus::message::Header<'_>,
    ) -> zbus::fdo::Result<()> {
        authorize_control(&self.controller, &header).await?;
        apply_config_command(
            &self.controller,
            DaemonCommand::SetRenderScale(Some(scale as f32)),
            "SetRenderScale",
        )
    }

    #[zbus(signal)]
    pub(super) async fn status_changed(
        signal_emitter: &SignalEmitter<'_>,
        status: HashMap<String, OwnedValue>,
    ) -> zbus::Result<()>;
}

// SPDX-License-Identifier: MIT

use trance_runner::launcher::{LaunchMode, resolve_saver_binary, sanitize_saver_name};

use super::{DaemonCommand, DaemonController};

impl DaemonController {
    pub fn apply_command(&self, command: DaemonCommand) -> Result<(), String> {
        match command {
            DaemonCommand::Enable => {
                let mut config = self.config.lock().unwrap();
                config.idle_enabled = true;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::Disable => {
                let mut config = self.config.lock().unwrap();
                config.idle_enabled = false;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::SetTimeout(minutes) => {
                if minutes == 0 || minutes > 240 {
                    return Err("timeout must be between 1 and 240 minutes".into());
                }
                let mut config = self.config.lock().unwrap();
                config.idle_timeout_mins = minutes;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::SetSaver(name) => {
                if let Some(ref saver) = name {
                    sanitize_saver_name(saver)
                        .ok_or_else(|| format!("unknown or invalid screensaver name: {saver}"))?;
                    resolve_saver_binary(saver, &LaunchMode::Daemon)
                        .map_err(|error| error.to_string())?;
                }
                let mut config = self.config.lock().unwrap();
                config.active_saver = name;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }

            DaemonCommand::SetShowFpsOverlay(enabled) => {
                let mut config = self.config.lock().unwrap();
                config.show_fps_overlay = enabled;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }

            DaemonCommand::SetRenderScale(scale) => {
                let stored = match scale {
                    None => None,
                    Some(value) if value <= 0.0 => None,
                    Some(value) => {
                        if !value.is_finite() || !(0.25..=1.0).contains(&value) {
                            return Err("render_scale must be between 0.25 and 1.0".into());
                        }
                        Some(value)
                    }
                };
                let mut config = self.config.lock().unwrap();
                config.render_scale = stored;
                config.save().map_err(|error| error.to_string())?;
                self.mark_dirty();
                Ok(())
            }
            DaemonCommand::Preview(_) | DaemonCommand::StopPresentation => Ok(()),
        }
    }
}

// SPDX-License-Identifier: MIT

use trance_dbus::TranceClient;

pub fn handle_config(client: &TranceClient, args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage: trance config get <key> | set <key> <val> | list".into());
    }

    match args[0].as_str() {
        "list" => {
            let status = client.get_status().map_err(|e| e.to_string())?;
            println!("idle_enabled:      {}", status.idle_enabled);
            println!("idle_timeout_mins: {}", status.idle_timeout_mins);
            println!(
                "active_saver:      {}",
                if status.active_saver.is_empty() {
                    "random"
                } else {
                    &status.active_saver
                }
            );
            println!("gpu_enabled:       {}", status.gpu_enabled);
            println!("show_fps_overlay:  {}", status.show_fps_overlay);
            println!(
                "render_scale:      {}",
                if status.render_scale.is_empty() {
                    "default"
                } else {
                    &status.render_scale
                }
            );
            Ok(())
        }
        "get" => {
            if args.len() < 2 {
                return Err("usage: trance config get <key>".into());
            }
            let key = args[1].as_str();
            let status = client.get_status().map_err(|e| e.to_string())?;
            match key {
                "idle_enabled" | "enabled" => println!("{}", status.idle_enabled),
                "idle_timeout_mins" | "timeout" => println!("{}", status.idle_timeout_mins),
                "active_saver" | "saver" => println!(
                    "{}",
                    if status.active_saver.is_empty() {
                        "random"
                    } else {
                        &status.active_saver
                    }
                ),
                "gpu_enabled" | "gpu" => println!("{}", status.gpu_enabled),
                "show_fps_overlay" | "fps" => println!("{}", status.show_fps_overlay),
                "render_scale" | "scale" => println!(
                    "{}",
                    if status.render_scale.is_empty() {
                        "default"
                    } else {
                        &status.render_scale
                    }
                ),
                _ => return Err(format!("unknown configuration key: {key}")),
            }
            Ok(())
        }
        "set" => {
            if args.len() < 3 {
                return Err("usage: trance config set <key> <value>".into());
            }
            let key = args[1].as_str();
            let val = args[2].as_str();
            match key {
                "idle_enabled" | "enabled" => {
                    let b = val
                        .parse::<bool>()
                        .map_err(|_| "value must be true or false")?;
                    if b { client.enable() } else { client.disable() }
                        .map_err(|e| e.to_string())?;
                }
                "idle_timeout_mins" | "timeout" => {
                    let n = val
                        .parse::<u32>()
                        .map_err(|_| "value must be an integer (1–240)")?;
                    if !(1..=240).contains(&n) {
                        return Err("timeout must be between 1 and 240 minutes".into());
                    }
                    client.set_timeout(n).map_err(|e| e.to_string())?;
                }
                "active_saver" | "saver" => {
                    let name = if val == "random" || val == "none" {
                        ""
                    } else {
                        val
                    };
                    client.set_saver(name).map_err(|e| e.to_string())?;
                }
                "gpu_enabled" | "gpu" => {
                    let b = val
                        .parse::<bool>()
                        .map_err(|_| "value must be true or false")?;
                    client.set_gpu_enabled(b).map_err(|e| e.to_string())?;
                }
                "show_fps_overlay" | "fps" => {
                    let b = val
                        .parse::<bool>()
                        .map_err(|_| "value must be true or false")?;
                    client.set_show_fps_overlay(b).map_err(|e| e.to_string())?;
                }
                "render_scale" | "scale" => {
                    let scale = if val == "default" {
                        0.0f32
                    } else {
                        val.parse::<f32>()
                            .map_err(|_| "value must be between 0.25 and 1.0, or 'default'")?
                    };
                    if scale != 0.0 && !(0.25..=1.0).contains(&scale) {
                        return Err("scale must be between 0.25 and 1.0, or 'default'".into());
                    }
                    client.set_render_scale(scale).map_err(|e| e.to_string())?;
                }
                _ => return Err(format!("unknown configuration key: {key}")),
            }
            println!("Set config key '{key}' to '{val}' successfully.");
            Ok(())
        }
        _ => Err("unknown config action; use get, set, or list".into()),
    }
}

// SPDX-License-Identifier: MIT

use std::sync::Arc;

use futures_lite::StreamExt;
use zbus::fdo::DBusProxy;
use zbus::names::BusName;

use crate::controller::DaemonController;
use crate::inhibit::InhibitorState;

pub async fn watch_inhibitor_clients(
    connection: zbus::Connection,
    inhibitors: Arc<InhibitorState>,
    controller: Arc<DaemonController>,
) {
    let dbus = match DBusProxy::new(&connection).await {
        Ok(proxy) => proxy,
        Err(error) => {
            tracing::error!("failed to watch inhibitor clients: {error}");
            return;
        }
    };

    let mut stream = match dbus.receive_name_owner_changed().await {
        Ok(stream) => stream,
        Err(error) => {
            tracing::error!("failed to subscribe to NameOwnerChanged: {error}");
            return;
        }
    };

    while let Some(event) = stream.next().await {
        let args = match event.args() {
            Ok(args) => args,
            Err(_) => continue,
        };
        if args.new_owner.is_some() {
            continue;
        }
        let BusName::Unique(name) = &args.name else {
            continue;
        };
        inhibitors.remove_client(name);
        controller.mark_dirty();
    }
}

pub async fn watch_external_dbus_inhibits(
    inhibitors: Arc<InhibitorState>,
    controller: Arc<DaemonController>,
) {
    use tokio::io::AsyncBufReadExt;

    loop {
        tracing::info!("Starting external D-Bus inhibitor monitor...");
        let mut child = match tokio::process::Command::new("dbus-monitor")
            .args([
                "type='method_call',interface='org.freedesktop.ScreenSaver'",
                "type='method_return'",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                tracing::error!("Failed to spawn dbus-monitor: {error}. Retrying in 5 seconds...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        let stdout = child.stdout.take().unwrap();
        let mut reader = tokio::io::BufReader::new(stdout).lines();

        let mut pending_inhibits: std::collections::HashMap<u32, (String, String, String)> =
            std::collections::HashMap::new();
        let mut current_member: Option<String> = None;
        let mut current_sender: Option<String> = None;
        let mut current_serial: Option<u32> = None;
        let mut current_reply_serial: Option<u32> = None;
        let mut app_name: Option<String> = None;

        while let Ok(Some(line)) = reader.next_line().await {
            let line = line.trim();
            if line.starts_with("method call ") {
                current_member = None;
                current_sender = None;
                current_serial = None;
                current_reply_serial = None;
                app_name = None;

                if line.contains("interface=org.freedesktop.ScreenSaver")
                    && let Some(sender_part) = extract_field(line, "sender=")
                    && let Some(serial_part) = extract_field(line, "serial=")
                    && let Ok(serial) = serial_part.parse::<u32>()
                {
                    current_sender = Some(sender_part);
                    current_serial = Some(serial);
                    if line.contains("member=Inhibit") {
                        current_member = Some("Inhibit".to_string());
                    } else if line.contains("member=UnInhibit") {
                        current_member = Some("UnInhibit".to_string());
                    }
                }
            } else if line.starts_with("method return ") {
                current_member = None;
                current_sender = None;
                current_serial = None;
                current_reply_serial = None;
                app_name = None;

                if let Some(reply_serial_part) = extract_field(line, "reply_serial=")
                    && let Ok(reply_serial) = reply_serial_part.parse::<u32>()
                {
                    current_reply_serial = Some(reply_serial);
                }
            } else if let Some(stripped) = line.strip_prefix("string ") {
                if !stripped.is_empty() {
                    let val = stripped.trim_matches('"').to_string();
                    if let Some(member) = &current_member
                        && member == "Inhibit"
                    {
                        if app_name.is_none() {
                            app_name = Some(val);
                        } else {
                            if let (Some(sender), Some(serial), Some(app)) =
                                (&current_sender, current_serial, app_name.take())
                            {
                                pending_inhibits.insert(serial, (sender.clone(), app, val));
                            }
                            current_member = None;
                        }
                    }
                }
            } else if let Some(stripped) = line.strip_prefix("uint32 ")
                && let Ok(cookie) = stripped.trim().parse::<u32>()
            {
                if let Some(member) = &current_member {
                    if member == "UnInhibit" {
                        if let Some(sender) = &current_sender
                            && let Ok(client_name) =
                                zbus::names::UniqueName::try_from(sender.as_str())
                        {
                            tracing::info!(
                                "Removing external inhibitor for client {} (cookie {})",
                                client_name,
                                cookie
                            );
                            inhibitors.remove_for_client(cookie, &client_name);
                            controller.mark_dirty();
                        }
                        current_member = None;
                    }
                } else if let Some(reply_serial) = current_reply_serial {
                    if let Some((sender, app, reason)) = pending_inhibits.remove(&reply_serial)
                        && let Ok(client_name) = zbus::names::UniqueName::try_from(sender.as_str())
                    {
                        inhibitors.add_with_cookie(app, reason, client_name.to_owned(), cookie);
                        controller.mark_dirty();
                    }
                    current_reply_serial = None;
                }
            }
        }

        tracing::warn!("dbus-monitor exited. Restarting in 5 seconds...");
        let _ = child.kill().await;
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

fn extract_field(line: &str, prefix: &str) -> Option<String> {
    if let Some(pos) = line.find(prefix) {
        let start = pos + prefix.len();
        let rest = &line[start..];
        let end = rest.find(|c: char| c.is_whitespace() || c == ';' || c == ',');
        let val = match end {
            Some(e) => &rest[..e],
            None => rest,
        };
        Some(val.trim_matches('"').to_string())
    } else {
        None
    }
}

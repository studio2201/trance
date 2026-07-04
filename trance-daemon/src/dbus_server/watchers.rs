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

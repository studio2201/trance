// SPDX-License-Identifier: MIT

use cosmic::iced::window::Id;

/// Messages emitted by the application and its widgets.
///
/// The update loop routes daemon-affecting toggles through D-Bus when
/// `daemon_client::is_running()` and persists [`Local76Config`] otherwise.
#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    PopupClosed(Id),
    SubscriptionChannel,
    UpdateConfig(crate::config::Config),
    ToggleIdleEnabled(bool),
    ActiveSaverSelected(String),
    ToggleDaemon(bool),
    ToggleFpsOverlay(bool),
    DecreaseTimeout,
    IncreaseTimeout,
    OpenPowerSettings,
    MiddleClick,
}

// Popup lifecycle messages are handled before settings mutations in update().

// SPDX-License-Identifier: MIT

//! COSMIC panel applet for trance screensaver configuration.

mod message;
mod state;
mod update;
mod view;

use cosmic::iced::window::Id;
use cosmic::prelude::*;

pub use message::Message;

pub struct AppModel {
    pub(crate) core: cosmic::Core,
    pub(crate) popup: Option<Id>,
    pub(crate) config: crate::config::Config,
    pub(crate) local_config: crate::config::Local76Config,
    pub(crate) screensavers: Vec<String>,
    pub(crate) daemon_running: bool,
    pub(crate) gpu_enabled: bool,
    pub(crate) show_fps_overlay: bool,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            core: cosmic::Core::default(),
            popup: None,
            config: crate::config::Config::default(),
            local_config: crate::config::Local76Config::default(),
            screensavers: Vec::new(),
            daemon_running: false,
            gpu_enabled: true,
            show_fps_overlay: false,
        }
    }
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "com.system76.CosmicApplet.Trance";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        Self::init_app(core)
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        self.view_panel()
    }

    fn view_window(&self, id: Id) -> Element<'_, Self::Message> {
        self.view_popup(id)
    }

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        self.subscription_batch()
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        self.handle_update(message)
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

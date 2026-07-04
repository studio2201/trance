// SPDX-License-Identifier: MIT

use cosmic::iced::window::Id;
use cosmic::widget;

use super::{AppModel, Message};

impl AppModel {
    pub(crate) fn view_panel(&self) -> cosmic::Element<'_, Message> {
        let btn = self
            .core
            .applet
            .icon_button("display-symbolic")
            .on_press(Message::TogglePopup);
        cosmic::iced::widget::mouse_area(btn)
            .on_middle_press(Message::MiddleClick)
            .into()
    }

    pub(crate) fn view_popup(&self, _id: Id) -> cosmic::Element<'_, Message> {
        let options = {
            let mut opts = vec!["Random".to_string()];
            for s in &self.screensavers {
                opts.push(s.clone());
            }
            opts
        };
        let selected = Some(
            self.local_config
                .active_saver
                .clone()
                .unwrap_or_else(|| "Random".to_string()),
        );

        let mut grid = cosmic::iced::widget::Column::new()
            .spacing(6)
            .width(cosmic::iced::Length::Fill);
        let mut row = cosmic::iced::widget::Row::new()
            .spacing(6)
            .width(cosmic::iced::Length::Fill);
        let len = options.len();
        for (i, s) in options.into_iter().enumerate() {
            let is_selected = selected.as_ref() == Some(&s);
            let btn = if is_selected {
                widget::button::suggested(s.clone())
            } else {
                widget::button::standard(s.clone())
            };
            let btn = btn
                .width(cosmic::iced::Length::Fill)
                .on_press(Message::ActiveSaverSelected(s));
            row = row.push(btn);
            if i % 2 == 1 {
                grid = grid.push(row);
                row = cosmic::iced::widget::Row::new()
                    .spacing(6)
                    .width(cosmic::iced::Length::Fill);
            }
        }
        if len % 2 != 0 {
            grid = grid.push(row);
        }

        let header = widget::text("Trance Screensaver").size(16);

        let decrease_btn = widget::button::standard("-").on_press(Message::DecreaseTimeout);
        let increase_btn = widget::button::standard("+").on_press(Message::IncreaseTimeout);
        let timeout_val = widget::text(format!("{} mins", self.local_config.idle_timeout_mins));

        let timeout_adjuster = cosmic::iced::widget::Row::new()
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .push(decrease_btn)
            .push(timeout_val)
            .push(increase_btn);

        let actions = widget::button::standard("Power Settings")
            .width(cosmic::iced::Length::Fill)
            .on_press(Message::OpenPowerSettings);

        let content_list = widget::list_column()
            .add(header)
            .add(widget::settings::item(
                "Background Daemon",
                widget::toggler(self.daemon_running).on_toggle(Message::ToggleDaemon),
            ))
            .add(widget::settings::item(
                "Idle Activation",
                widget::toggler(self.local_config.idle_enabled)
                    .on_toggle(Message::ToggleIdleEnabled),
            ))
            .add(widget::settings::item("Idle Timeout", timeout_adjuster))
            .add(widget::settings::item(
                "FPS Overlay",
                widget::toggler(self.show_fps_overlay).on_toggle(Message::ToggleFpsOverlay),
            ))
            .add(cosmic::iced::widget::container(grid).width(cosmic::iced::Length::Fill))
            .add(actions);

        self.core.applet.popup_container(content_list).into()
    }
}

// SPDX-License-Identifier: GPL-3.0-only

use cosmic::{
    app::{Core, Task},
    iced::Subscription,
    widget::{autosize, button, Id},
    Element,
};
use std::sync::LazyLock;

use cosmic::cosmic_config::CosmicConfigEntry;

use crate::config::{self, ClockCenterConfig, TimeAppletConfig};

static AUTOSIZE_MAIN_ID: LazyLock<Id> = LazyLock::new(|| Id::new("autosize-main"));

pub struct App {
    core: Core,
    time_config: TimeAppletConfig,
    now: jiff::Zoned,
    #[allow(dead_code)]
    config: ClockCenterConfig,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Placeholder until real messages land.
    Noop,
    Tick,
    TimeConfigChanged(TimeAppletConfig),
}

impl cosmic::Application for App {
    type Message = Message;
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    const APP_ID: &'static str = config::APP_ID;

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        (
            Self {
                core,
                time_config: cosmic::cosmic_config::Config::new(config::TIME_CONFIG_ID, TimeAppletConfig::VERSION)
                    .ok()
                    .and_then(|c| TimeAppletConfig::get_entry(&c).ok())
                    .unwrap_or_default(),
                now: jiff::Zoned::now(),
                config: ClockCenterConfig::default(),
            },
            Task::none(),
        )
    }

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::Noop => Task::none(),
            Message::Tick => {
                self.now = jiff::Zoned::now();
                Task::none()
            }
            Message::TimeConfigChanged(c) => {
                self.time_config = c;
                self.now = jiff::Zoned::now();
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let period = if self.time_config.show_seconds
            || self.time_config.format_strftime.contains("%S")
            || self.time_config.format_strftime.contains("%T")
        {
            std::time::Duration::from_secs(1)
        } else {
            std::time::Duration::from_secs(15)
        };
        Subscription::batch([
            cosmic::iced::time::every(period).map(|_| Message::Tick),
            self.core
                .watch_config::<TimeAppletConfig>(config::TIME_CONFIG_ID)
                .map(|u| {
                    for err in u.errors {
                        tracing::error!(?err, "Error watching time config");
                    }
                    Message::TimeConfigChanged(u.config)
                }),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let padding = self.core.applet.suggested_padding(true).0;
        let btn = button::custom(self.core.applet.text(crate::clock::format_label(&self.now, &self.time_config)))
            .padding([0, padding])
            .on_press(Message::Noop)
            .class(cosmic::theme::Button::AppletIcon);
        autosize::autosize(btn, AUTOSIZE_MAIN_ID.clone()).into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

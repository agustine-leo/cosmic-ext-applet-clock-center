// SPDX-License-Identifier: GPL-3.0-only

use cosmic::{
    app::{Core, Task},
    iced::Subscription,
    widget::{autosize, button, Id},
    Element,
};
use std::sync::LazyLock;

use crate::config::{self, ClockCenterConfig, TimeAppletConfig};

static AUTOSIZE_MAIN_ID: LazyLock<Id> = LazyLock::new(|| Id::new("autosize-main"));

pub struct App {
    core: Core,
    #[allow(dead_code)]
    time_config: TimeAppletConfig,
    #[allow(dead_code)]
    config: ClockCenterConfig,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Placeholder until real messages land.
    Noop,
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
                time_config: TimeAppletConfig::default(),
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
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let padding = self.core.applet.suggested_padding(true).0;
        let btn = button::custom(self.core.applet.text("clock-center"))
            .padding([0, padding])
            .on_press(Message::Noop)
            .class(cosmic::theme::Button::AppletIcon);
        autosize::autosize(btn, AUTOSIZE_MAIN_ID.clone()).into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

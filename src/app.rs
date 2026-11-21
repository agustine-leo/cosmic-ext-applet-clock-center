// SPDX-License-Identifier: GPL-3.0-only

use cosmic::{
    app::{Core, Task},
    iced::{
        platform_specific::shell::wayland::commands::popup::destroy_popup,
        window, Rectangle, Subscription,
    },
    widget::{autosize, button, container, rectangle_tracker::*, Id},
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
    popup: Option<window::Id>,
    rectangle_tracker: Option<RectangleTracker<u32>>,
    rectangle: Rectangle,
    date_selected: jiff::civil::Date,
    #[allow(dead_code)]
    config: ClockCenterConfig,
}

#[derive(Debug, Clone)]
pub enum Message {
    TogglePopup,
    CloseRequested(window::Id),
    Rectangle(RectangleUpdate<u32>),
    PrevMonth,
    NextMonth,
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
                popup: None,
                rectangle_tracker: None,
                rectangle: Rectangle::default(),
                date_selected: jiff::Zoned::now().date(),
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
            Message::TogglePopup => {
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
                return cosmic::surface::surface_task(cosmic::surface::action::app_popup(
                    |_| Default::default(),
                    |app: &mut Self| {
                        app.date_selected = app.now.date();
                        let new_id = window::Id::unique();
                        app.popup = Some(new_id);
                        let mut settings = app.core.applet.get_popup_settings(
                            app.core.main_window_id().unwrap(),
                            new_id,
                            None,
                            None,
                            None,
                        );
                        let Rectangle { x, y, width, height } = app.rectangle;
                        settings.positioner.anchor_rect = Rectangle::<i32> {
                            x: x.max(1.) as i32,
                            y: y.max(1.) as i32,
                            width: width.max(1.) as i32,
                            height: height.max(1.) as i32,
                        };
                        settings.positioner.size = None;
                        settings
                    },
                    None,
                ));
            }
            Message::CloseRequested(id) => {
                if Some(id) == self.popup {
                    self.popup = None;
                }
                Task::none()
            }
            Message::Rectangle(u) => {
                match u {
                    RectangleUpdate::Rectangle(r) => self.rectangle = r.1,
                    RectangleUpdate::Init(t) => self.rectangle_tracker = Some(t),
                }
                Task::none()
            }
            Message::PrevMonth => {
                self.date_selected = shift_month(self.date_selected, -1);
                Task::none()
            }
            Message::NextMonth => {
                self.date_selected = shift_month(self.date_selected, 1);
                Task::none()
            }
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
        let seconds = self.time_config.show_seconds
            || self.time_config.format_strftime.contains("%S")
            || self.time_config.format_strftime.contains("%T");
        let tick = if seconds {
            cosmic::iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick)
        } else {
            // Wake once per minute, aligned to the minute boundary.
            Subscription::run(|| {
                cosmic::iced::stream::channel(1, |mut output| async move {
                    loop {
                        let now = jiff::Zoned::now();
                        let wait_ms = 60_000 - (now.second() as u64 * 1000 + now.millisecond() as u64);
                        tokio::time::sleep(std::time::Duration::from_millis(wait_ms + 20)).await;
                        if futures_util::SinkExt::send(&mut output, Message::Tick).await.is_err() {
                            break;
                        }
                    }
                    std::future::pending().await
                })
            })
        };
        Subscription::batch([
            tick,
            rectangle_tracker_subscription(0).map(|e| Message::Rectangle(e.1)),
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
            .on_press_down(Message::TogglePopup)
            .class(cosmic::theme::Button::AppletIcon);
        autosize::autosize(
            if let Some(tracker) = self.rectangle_tracker.as_ref() {
                Element::from(tracker.container(0, btn).ignore_bounds(true))
            } else {
                btn.into()
            },
            AUTOSIZE_MAIN_ID.clone(),
        )
        .into()
    }

    fn view_window(&self, _id: window::Id) -> Element<'_, Message> {
        let cal = crate::calendar::view(self.date_selected, self.now.date(), self.time_config.first_day_of_week);
        self.core.applet.popup_container(container(cal).padding(8).width(cosmic::iced::Length::Fixed(324.0))).into()
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::CloseRequested(id))
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

/// Move to the same day in an adjacent month, clamped to the month's length.
fn shift_month(d: jiff::civil::Date, months: i32) -> jiff::civil::Date {
    d.checked_add(jiff::Span::new().months(months)).unwrap_or(d)
}

// SPDX-License-Identifier: GPL-3.0-only

use cosmic::{
    app::{Core, Task},
    iced::{
        platform_specific::shell::wayland::commands::popup::destroy_popup,
        window, Rectangle, Subscription,
    },
    widget::{autosize, button, column, container, rectangle_tracker::*, row, space, Id},
    Element,
};
use cosmic_notifications_util::{ActionId, Notification};
use tokio::sync::mpsc;
use std::{collections::HashSet, sync::LazyLock};

use cosmic::cosmic_config::CosmicConfigEntry;

use crate::config::{self, ClockCenterConfig, TimeAppletConfig};
use crate::media;
use crate::notifications::{self, channel, ChannelStatus, NotificationStore};

static AUTOSIZE_MAIN_ID: LazyLock<Id> = LazyLock::new(|| Id::new("autosize-main"));

pub struct App {
    core: Core,
    time_config: TimeAppletConfig,
    now: jiff::Zoned,
    popup: Option<window::Id>,
    rectangle_tracker: Option<RectangleTracker<u32>>,
    rectangle: Rectangle,
    date_selected: jiff::civil::Date,
    notifications: NotificationStore,
    notifications_status: Option<ChannelStatus>,
    notifications_tx: Option<mpsc::Sender<channel::Input>>,
    /// Notification ids whose card shows the full body.
    expanded_cards: HashSet<u32>,
    /// App names whose group shows all cards, not only the newest.
    expanded_groups: HashSet<String>,
    do_not_disturb: bool,
    players: Vec<media::PlayerState>,
    /// Loaded album art for the shown player, keyed by its `file://` URL.
    media_art: Option<(String, cosmic::widget::icon::Handle)>,
    config: ClockCenterConfig,
    timer: crate::timer::TimerState,
    /// Latest `Instant` seen by a timer tick (drives the countdown view).
    timer_now: std::time::Instant,
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
    Notification(Notification),
    ChannelStatus(ChannelStatus),
    NotificationsInput(mpsc::Sender<channel::Input>),
    NotificationClosed(u32),
    DismissNotification(u32),
    ActivateNotification(u32),
    ClearNotifications,
    ClearNotificationGroup(String),
    ToggleNotificationExpanded(u32),
    ToggleNotificationGroup(String),
    SetDnd(bool),
    DndChanged(bool),
    Players(Vec<media::PlayerState>),
    MediaCommand(String, media::MediaCommand),
    StartTimer(u32),
    CancelTimer,
    TimerTick,
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
                notifications: NotificationStore::default(),
                notifications_status: None,
                notifications_tx: None,
                do_not_disturb: notifications::dnd::read(),
                expanded_cards: HashSet::new(),
                expanded_groups: HashSet::new(),
                players: Vec::new(),
                media_art: None,
                config: ClockCenterConfig::default(),
                timer: Default::default(),
                timer_now: std::time::Instant::now(),
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
                        app.notifications.mark_all_read();
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
            Message::StartTimer(m) => {
                self.timer_now = std::time::Instant::now();
                self.timer.start(m, self.timer_now);
                Task::none()
            }
            Message::CancelTimer => {
                self.timer.cancel();
                Task::none()
            }
            Message::TimerTick => {
                self.timer_now = std::time::Instant::now();
                match self.timer.tick(self.timer_now) {
                    Some(m) => Task::perform(crate::timer::send_done_notification(m), |r| {
                        if let Err(err) = r {
                            tracing::error!(?err, "Failed to send timer notification");
                        }
                        cosmic::Action::None
                    }),
                    None => Task::none(),
                }
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
            Message::Notification(n) => {
                self.notifications.push(n);
                if self.popup.is_some() {
                    self.notifications.mark_all_read();
                }
                Task::none()
            }
            Message::ChannelStatus(status) => {
                self.notifications_status = Some(status);
                Task::none()
            }
            Message::NotificationsInput(tx) => {
                self.notifications_tx = Some(tx);
                Task::none()
            }
            Message::NotificationClosed(id) => {
                self.notifications.dismiss(id);
                self.prune_expansion();
                Task::none()
            }
            Message::DismissNotification(id) => {
                self.notifications.dismiss(id);
                self.prune_expansion();
                cosmic::iced::Task::future(channel::close_notifications(vec![id])).discard()
            }
            Message::ClearNotifications => {
                let ids = self.notifications.ids();
                self.notifications.clear();
                self.prune_expansion();
                cosmic::iced::Task::future(channel::close_notifications(ids)).discard()
            }
            Message::ClearNotificationGroup(app) => {
                let ids = self.notifications.ids_for_app(&app);
                for id in &ids {
                    self.notifications.dismiss(*id);
                }
                self.prune_expansion();
                cosmic::iced::Task::future(channel::close_notifications(ids)).discard()
            }
            Message::ToggleNotificationExpanded(id) => {
                if !self.expanded_cards.remove(&id) {
                    self.expanded_cards.insert(id);
                }
                Task::none()
            }
            Message::ToggleNotificationGroup(app) => {
                if !self.expanded_groups.remove(&app) {
                    self.expanded_groups.insert(app);
                }
                Task::none()
            }
            Message::SetDnd(b) => {
                self.do_not_disturb = b;
                notifications::dnd::write(b);
                Task::none()
            }
            Message::DndChanged(b) => {
                self.do_not_disturb = b;
                Task::none()
            }
            Message::Players(players) => {
                self.players = players;
                let url = media::pick_player(&self.players).and_then(|p| p.art_url.clone());
                if self.media_art.as_ref().map(|(u, _)| u) != url.as_ref() {
                    self.media_art = url.and_then(|u| {
                        let path = media::art_path(&u)?;
                        Some((u, cosmic::widget::icon::from_path(path)))
                    });
                }
                Task::none()
            }
            Message::MediaCommand(bus, cmd) => {
                cosmic::iced::Task::future(media::send_command(bus, cmd)).discard()
            }
            Message::ActivateNotification(id) => {
                let Some(n) = self.notifications.get(id) else {
                    return Task::none();
                };
                let action = if n.actions.iter().any(|a| matches!(a.0, ActionId::Default)) {
                    Some(ActionId::Default.to_string())
                } else {
                    n.actions.first().map(|a| a.0.to_string())
                };
                match (action, self.notifications_tx.clone()) {
                    (Some(action), Some(tx)) => cosmic::iced::Task::future(async move {
                        if let Err(err) = tx.send(channel::Input::Activate(id, action)).await {
                            tracing::error!("Failed to send activation for {id}: {err}");
                        }
                    })
                    .discard(),
                    // Nothing to activate: clicking just dismisses it.
                    (None, _) => cosmic::task::message(Message::DismissNotification(id)),
                    (Some(_), None) => {
                        tracing::warn!("Activation for {id} ignored: notifications channel not ready");
                        Task::none()
                    }
                }
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
        let timer_tick = if self.timer.current.is_some() {
            cosmic::iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::TimerTick)
        } else {
            Subscription::none()
        };
        Subscription::batch([
            tick,
            timer_tick,
            rectangle_tracker_subscription(0).map(|e| Message::Rectangle(e.1)),
            self.core
                .watch_config::<TimeAppletConfig>(config::TIME_CONFIG_ID)
                .map(|u| {
                    for err in u.errors {
                        tracing::error!(?err, "Error watching time config");
                    }
                    Message::TimeConfigChanged(u.config)
                }),
            self.core
                .watch_config::<cosmic_notifications_config::NotificationsConfig>(cosmic_notifications_config::ID)
                .map(|u| {
                    for err in u.errors {
                        tracing::error!(?err, "Error watching notifications config");
                    }
                    Message::DndChanged(u.config.do_not_disturb)
                }),
            notifications::channel::subscription(),
            notifications::channel::closed_subscription(),
            media::subscription(),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let padding = self.core.applet.suggested_padding(true).0;
        let label = self.core.applet.text(crate::clock::format_label(&self.now, &self.time_config));
        let content: Element<'_, Message> = if self.notifications.has_unread() {
            row![label, unread_dot()].spacing(4).align_y(cosmic::iced::Alignment::Center).into()
        } else {
            label.into()
        };
        let btn = button::custom(content)
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
        let notifs = notifications::view(
            &self.notifications,
            self.notifications_status.as_ref(),
            notifications::Expansion { cards: &self.expanded_cards, groups: &self.expanded_groups },
            self.do_not_disturb,
        );
        let mut content = column![].spacing(8);
        if media::pick_player(&self.players).is_some() {
            content = content.push(media::view(&self.players, self.media_art.as_ref().map(|(_, h)| h)));
        }
        let content = content.push(notifs).push(cal).push(crate::timer::view(&self.config.timer_presets, &self.timer, self.timer_now));
        self.core.applet.popup_container(container(content).padding(8).width(cosmic::iced::Length::Fixed(324.0))).into()
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::CloseRequested(id))
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

impl App {
    /// Forget expansion state for notifications/groups that no longer exist.
    fn prune_expansion(&mut self) {
        let store = &self.notifications;
        self.expanded_cards.retain(|id| store.contains(*id));
        let apps: HashSet<String> = store.groups_newest_first().into_iter().map(|(a, _)| a).collect();
        self.expanded_groups.retain(|a| apps.contains(a));
    }
}

/// Move to the same day in an adjacent month, clamped to the month's length.
fn shift_month(d: jiff::civil::Date, months: i32) -> jiff::civil::Date {
    d.checked_add(jiff::Span::new().months(months)).unwrap_or(d)
}

/// Small accent-coloured dot shown after the time while there are unread notifications.
fn unread_dot() -> Element<'static, Message> {
    container(space::horizontal().width(6).height(6))
        .class(cosmic::theme::Container::custom(|theme| container::Style {
            background: Some(cosmic::iced::Color::from(theme.cosmic().accent_color()).into()),
            border: cosmic::iced::Border { radius: 3.0.into(), ..Default::default() },
            ..Default::default()
        }))
        .into()
}

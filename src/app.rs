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

use crate::config::{self, TimeAppletConfig};
use crate::media;
use crate::notifications::{self, channel, ChannelStatus, NotificationStore};

/// Popup column widths (~60/40) and minimum content height.
const LEFT_WIDTH: f32 = 416.0;
const RIGHT_WIDTH: f32 = 288.0;
const MIN_HEIGHT: f32 = 760.0;
/// Popup padding (theme `space_s`) and gap between columns (theme `space_xs`).
/// Fixed values, so the popup width matches the content exactly.
const POPUP_PADDING: f32 = 16.0;
const COLUMN_SPACING: f32 = 12.0;
const POPUP_WIDTH: f32 = popup_width(LEFT_WIDTH, RIGHT_WIDTH, POPUP_PADDING, COLUMN_SPACING);

/// Left column + 1px divider + right column, with a gap on each side of the
/// divider and padding around everything.
pub const fn popup_width(left: f32, right: f32, padding: f32, gap: f32) -> f32 {
    left + 1.0 + right + 2.0 * gap + 2.0 * padding
}
const POPUP_MAX_HEIGHT: f32 = 1000.0;

/// libcosmic caps applet popups at 360px wide; this popup needs more.
fn popup_limits() -> cosmic::iced::Limits {
    cosmic::iced::Limits::NONE
        .min_width(POPUP_WIDTH)
        .max_width(POPUP_WIDTH)
        .min_height(MIN_HEIGHT + 2.0 * POPUP_PADDING)
        .max_height(POPUP_MAX_HEIGHT)
}

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
    /// Do Not Disturb forced on while the popup is open (banners go to the list).
    popup_dnd: notifications::dnd::PopupDnd,
    players: Vec<media::PlayerState>,
    /// Loaded album art for the shown player, keyed by its `file://` URL.
    media_art: Option<(String, cosmic::widget::icon::Handle)>,
    /// Animation frame of the playing indicator.
    indicator_frame: u32,
    /// World clocks and weather place.
    cc_config: config::ClockCenterConfig,
    weather: Option<crate::extras::Weather>,
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
    /// A specific action button on a card (action id as sent by the app).
    InvokeAction(u32, String),
    ClearNotifications,
    ClearNotificationGroup(String),
    ToggleNotificationExpanded(u32),
    ToggleNotificationGroup(String),
    SetDnd(bool),
    DndChanged(bool),
    Players(Vec<media::PlayerState>),
    MediaCommand(String, media::MediaCommand),
    IndicatorFrame,
    ClockCenterConfigChanged(config::ClockCenterConfig),
    RefreshWeather,
    Weather(Option<crate::extras::Weather>),
}

impl App {
    /// Ask the daemon to run `action` for notification `id`, then close the
    /// popup (GNOME and the stock applet close it after activating).
    fn send_activation(&mut self, id: u32, action: String) -> Task<Message> {
        let Some(tx) = self.notifications_tx.clone() else {
            tracing::warn!("Activation for {id} ignored: notifications channel not ready");
            return Task::none();
        };
        let activate = cosmic::iced::Task::future(async move {
            if let Err(err) = tx.send(channel::Input::Activate(id, action)).await {
                tracing::error!("Failed to send activation for {id}: {err}");
            }
        })
        .discard();
        match self.popup.take() {
            Some(p) => {
                self.popup_closed();
                Task::batch([activate, destroy_popup(p)])
            }
            None => activate,
        }
    }

    fn popup_opened(&mut self) {
        if let Some(v) = self.popup_dnd.on_open(self.do_not_disturb) {
            if let Some(m) = notifications::dnd::marker_path() {
                notifications::dnd::set_marker(&m, true);
            }
            notifications::dnd::write(v);
        }
    }

    fn popup_closed(&mut self) {
        // Each opening starts collapsed: groups show only their newest card
        // and no card is expanded.
        self.expanded_groups.clear();
        self.expanded_cards.clear();
        // Unread rows keep their dot while the popup is open; seen on close.
        self.notifications.mark_all_read();
        if let Some(v) = self.popup_dnd.on_close() {
            notifications::dnd::write(v);
            if let Some(m) = notifications::dnd::marker_path() {
                notifications::dnd::set_marker(&m, false);
            }
        }
    }
}

/// Do Not Disturb at startup, undoing a value left forced by a crash while the popup was open.
fn startup_dnd() -> bool {
    if notifications::dnd::marker_path().is_some_and(|m| notifications::dnd::take_marker(&m)) {
        tracing::warn!("Do Not Disturb was left on by an open popup; switching it back off");
        notifications::dnd::write(false);
        return false;
    }
    notifications::dnd::read()
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
                do_not_disturb: startup_dnd(),
                popup_dnd: Default::default(),
                expanded_cards: HashSet::new(),
                expanded_groups: HashSet::new(),
                players: Vec::new(),
                media_art: None,
                indicator_frame: 0,
                cc_config: cosmic::cosmic_config::Config::new(config::CLOCK_CENTER_CONFIG_ID, config::ClockCenterConfig::VERSION)
                    .ok()
                    .and_then(|c| config::ClockCenterConfig::get_entry(&c).ok())
                    .unwrap_or_default(),
                weather: None,
            },
            Task::done(cosmic::Action::App(Message::RefreshWeather)),
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
                    self.popup_closed();
                    return destroy_popup(p);
                }
                return cosmic::surface::surface_task(cosmic::surface::action::app_popup(
                    |_| Default::default(),
                    |app: &mut Self| {
                        app.date_selected = app.now.date();
                        app.popup_opened();
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
                        settings.positioner.size_limits = popup_limits();
                        settings
                    },
                    None,
                ));
            }
            Message::CloseRequested(id) => {
                if Some(id) == self.popup {
                    self.popup = None;
                    self.popup_closed();
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
            Message::IndicatorFrame => {
                self.indicator_frame = self.indicator_frame.wrapping_add(1);
                Task::none()
            }
            Message::Tick => {
                self.now = jiff::Zoned::now();
                Task::none()
            }
            Message::ClockCenterConfigChanged(c) => {
                let place_changed = c.weather != self.cc_config.weather;
                self.cc_config = c;
                if place_changed {
                    self.weather = None;
                    return Task::done(cosmic::Action::App(Message::RefreshWeather));
                }
                Task::none()
            }
            Message::RefreshWeather => match self.cc_config.weather.clone() {
                Some(place) => Task::perform(crate::extras::fetch(place), |w| cosmic::Action::App(Message::Weather(w))),
                None => Task::none(),
            },
            Message::Weather(w) => {
                // Keep the last good reading through a failed refetch.
                if w.is_some() {
                    self.weather = w;
                }
                Task::none()
            }
            Message::TimeConfigChanged(c) => {
                self.time_config = c;
                self.now = jiff::Zoned::now();
                Task::none()
            }
            Message::Notification(n) => {
                self.notifications.push(n);
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
                self.popup_dnd.on_user_set();
                if let Some(m) = notifications::dnd::marker_path() {
                    notifications::dnd::set_marker(&m, false);
                }
                notifications::dnd::write(b);
                Task::none()
            }
            Message::DndChanged(b) => {
                // Our own forced `true` while the popup is open is not the user's setting.
                if self.popup_dnd.forced() && b {
                    return Task::none();
                }
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
                match action {
                    Some(action) => self.send_activation(id, action),
                    // Nothing to activate: clicking just dismisses it.
                    None => cosmic::task::message(Message::DismissNotification(id)),
                }
            }
            Message::InvokeAction(id, action) => self.send_activation(id, action),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let seconds = self.time_config.show_seconds
            || self.time_config.format_strftime.contains("%S")
            || self.time_config.format_strftime.contains("%T");
        let tick = if seconds {
            cosmic::iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick)
        } else {
            // Tick when the wall-clock minute changes. Sleeps are capped at 5s so the
            // label catches up within seconds after suspend/resume.
            Subscription::run(|| {
                cosmic::iced::stream::channel(1, |mut output| async move {
                    let mut last = crate::clock::minute_key(&jiff::Zoned::now());
                    loop {
                        tokio::time::sleep(crate::clock::tick_delay(&jiff::Zoned::now())).await;
                        let key = crate::clock::minute_key(&jiff::Zoned::now());
                        if key == last {
                            continue;
                        }
                        last = key;
                        if futures_util::SinkExt::send(&mut output, Message::Tick).await.is_err() {
                            break;
                        }
                    }
                    std::future::pending().await
                })
            })
        };
        // Only animate while something is playing: no ticking otherwise.
        let indicator_tick = if media::any_playing(&self.players) {
            cosmic::iced::time::every(std::time::Duration::from_millis(166)).map(|_| Message::IndicatorFrame)
        } else {
            Subscription::none()
        };
        Subscription::batch([
            tick,
            indicator_tick,
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
            self.core
                .watch_config::<config::ClockCenterConfig>(config::CLOCK_CENTER_CONFIG_ID)
                .map(|u| {
                    for err in u.errors {
                        tracing::error!(?err, "Error watching clock center config");
                    }
                    Message::ClockCenterConfigChanged(u.config)
                }),
            if self.cc_config.weather.is_some() {
                cosmic::iced::time::every(crate::extras::WEATHER_EVERY).map(|_| Message::RefreshWeather)
            } else {
                Subscription::none()
            },
            notifications::channel::subscription(),
            notifications::channel::closed_subscription(),
            media::subscription(),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let padding = self.core.applet.suggested_padding(true).0;
        let label = self.core.applet.text(crate::clock::format_label(&self.now, &self.time_config));
        let mut content = row![].spacing(4).align_y(cosmic::iced::Alignment::Center);
        if media::any_playing(&self.players) {
            content = content.push(crate::indicator::view(self.indicator_frame));
        }
        content = content.push(label);
        if self.notifications.has_unread() {
            content = content.push(unread_dot());
        }
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
        use cosmic::iced::Length;

        let content = popup_content(PopupData {
            players: &self.players,
            art: self.media_art.as_ref().map(|(_, h)| h),
            store: &self.notifications,
            status: self.notifications_status.as_ref(),
            expansion: notifications::Expansion { cards: &self.expanded_cards, groups: &self.expanded_groups },
            do_not_disturb: self.do_not_disturb,
            shown: self.date_selected,
            today: self.now.date(),
            first_day_of_week: self.time_config.first_day_of_week,
            clocks: &self.cc_config.world_clocks,
            place: self.cc_config.weather.as_ref(),
            weather: self.weather.as_ref(),
            now: &self.now,
        });
        self.core
            .applet
            .popup_container(
                container(content)
                    .padding(POPUP_PADDING)
                    .height(Length::Fixed(MIN_HEIGHT + 2.0 * POPUP_PADDING))
            )
            .limits(popup_limits())
            .into()
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::CloseRequested(id))
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

/// Everything the popup draws, borrowed from the app (or from sample data
/// in the `CLOCK_CENTER_PREVIEW` window).
pub struct PopupData<'a> {
    pub players: &'a [media::PlayerState],
    pub art: Option<&'a cosmic::widget::icon::Handle>,
    pub store: &'a NotificationStore,
    pub status: Option<&'a ChannelStatus>,
    pub expansion: notifications::Expansion<'a>,
    pub do_not_disturb: bool,
    pub shown: jiff::civil::Date,
    pub today: jiff::civil::Date,
    pub first_day_of_week: u8,
    pub clocks: &'a [config::WorldClock],
    pub place: Option<&'a config::WeatherPlace>,
    pub weather: Option<&'a crate::extras::Weather>,
    pub now: &'a jiff::Zoned,
}

/// The popup's two columns at their fixed size (without the popup frame).
pub fn popup_content(d: PopupData<'_>) -> Element<'_, Message> {
    use cosmic::iced::Length;

    // Left: media (if any), notifications filling the column, DND/Clear pinned at the bottom.
    let space = cosmic::theme::spacing();
    let _ = space;
    let mut left = column![].spacing(20).width(Length::Fixed(LEFT_WIDTH)).height(Length::Fill);
    if media::pick_player(d.players).is_some() {
        left = left.push(media::view(d.players, d.art));
    }
    left = left.push(notifications::view(d.store, d.status, d.expansion, d.do_not_disturb));

    // Right: calendar, then the credits (world clocks, weather).
    let right = column![
        crate::calendar::view(d.shown, d.today, d.first_day_of_week),
        crate::sleeve::hairline(),
        crate::extras::view(d.clocks, d.place, d.weather, d.now),
    ]
    .spacing(16)
    .width(Length::Fixed(RIGHT_WIDTH));

    row![left, crate::sleeve::vrule(), right].spacing(COLUMN_SPACING).into()
}

/// Popup size (content + padding), for the preview window.
pub const POPUP_SIZE: (f32, f32) = (POPUP_WIDTH, MIN_HEIGHT + 2.0 * POPUP_PADDING);
pub const PREVIEW_PADDING: f32 = POPUP_PADDING;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_surface_enforces_min_height() {
        assert_eq!(popup_limits().min().height, MIN_HEIGHT + 2.0 * POPUP_PADDING);
    }

    #[test]
    fn popup_width_matches_layout() {
        assert_eq!(popup_width(416.0, 288.0, 16.0, 12.0), 416.0 + 1.0 + 288.0 + 2.0 * 12.0 + 2.0 * 16.0);
    }
}

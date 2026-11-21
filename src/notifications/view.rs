// Adapted from pop-os/cosmic-applets cosmic-applet-notifications/src/lib.rs @ 328d142 (GPL-3.0-only)
// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use cosmic::{
    iced::{
        widget::{column, rich_text, row},
        Alignment, Length,
    },
    widget::{button, container, icon, scrollable, text, Column},
    Element,
};
use cosmic_notifications_util::{markup, Notification};

use super::{ChannelStatus, NotificationStore};
use crate::{app::Message, fl};

/// The notification list with a bottom row holding Clear.
pub fn view<'a>(store: &'a NotificationStore, status: Option<&'a ChannelStatus>) -> Element<'a, Message> {
    let list: Element<'a, Message> = if let Some(ChannelStatus::Unavailable(reason)) = status {
        container(text::body(fl!("notifications-unavailable", reason = reason.as_str())))
            .padding([8, 0])
            .width(Length::Fill)
            .into()
    } else if store.is_empty() {
        container(
            column![
                icon::from_name("cosmic-applet-notification-symbolic").size(40).symbolic(true).icon(),
                text::body(fl!("no-notifications")),
            ]
            .spacing(4)
            .align_x(Alignment::Center),
        )
        .padding([8, 0])
        .center_x(Length::Fill)
        .into()
    } else {
        scrollable(
            Column::with_children(store.iter_newest_first().map(item))
                .spacing(4)
                .width(Length::Fill),
        )
        .height(Length::Shrink)
        .into()
    };

    let clear = button::text(fl!("clear-all")).on_press_maybe((!store.is_empty()).then_some(Message::ClearNotifications));
    let bottom = row![cosmic::widget::space::horizontal(), clear].align_y(Alignment::Center);

    column![
        container(list).max_height(360.0),
        bottom,
    ]
    .spacing(4)
    .width(Length::Fill)
    .into()
}

fn item(n: &Notification) -> Element<'_, Message> {
    let app_name = text::caption(n.app_name.as_str()).width(Length::Fill);
    let close = button::custom(icon::from_name("window-close-symbolic").size(16).symbolic(true))
        .on_press(Message::DismissNotification(n.id))
        .class(cosmic::theme::Button::Text);
    let header = if let Some(ic) = n.notification_icon() {
        row![ic.size(16), app_name, text::caption(duration_ago(n)), close]
    } else {
        row![app_name, text::caption(duration_ago(n)), close]
    }
    .spacing(8)
    .align_y(Alignment::Center);

    let mut body = column![text::body(n.summary.lines().next().unwrap_or_default()).width(Length::Fill)];
    if !n.body.is_empty() {
        body = body.push(rich_text(markup::html_to_spans(&n.body)).size(12.0).width(Length::Fill));
    }

    button::custom(column![header, body].width(Length::Fill))
        .on_press(Message::ActivateNotification(n.id))
        .class(cosmic::theme::Button::MenuItem)
        .padding(8)
        .width(Length::Fill)
        .into()
}

fn duration_ago(n: &Notification) -> String {
    let Some(d) = n.duration_since() else { return String::new() };
    let min = d.as_secs() / 60;
    let hrs = min / 60;
    if hrs > 0 {
        fl!("hours-ago", duration = hrs)
    } else if min > 0 {
        fl!("minutes-ago", duration = min)
    } else {
        fl!("just-now")
    }
}

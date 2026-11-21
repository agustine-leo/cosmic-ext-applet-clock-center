// Adapted from pop-os/cosmic-applets cosmic-applet-notifications/src/lib.rs @ 328d142 (GPL-3.0-only)
// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashSet;

use cosmic::{
    iced::{
        advanced::text::{Ellipsize, EllipsizeHeightLimit},
        widget::{column, row},
        Alignment, Length,
    },
    widget::{button, container, icon, scrollable, text, Column},
    Element,
};
use cosmic_notifications_util::Notification;

use super::{first_line_preview, normalize_body, ChannelStatus, NotificationStore};
use crate::{app::Message, fl};

/// Which cards and groups the user has expanded.
pub struct Expansion<'a> {
    pub cards: &'a HashSet<u32>,
    pub groups: &'a HashSet<String>,
}

/// Notifications grouped per app, with a bottom row holding Clear.
pub fn view<'a>(
    store: &'a NotificationStore,
    status: Option<&'a ChannelStatus>,
    expansion: Expansion<'a>,
    do_not_disturb: bool,
) -> Element<'a, Message> {
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
        let groups = store
            .groups_newest_first()
            .into_iter()
            .map(|(app, ns)| group(app, ns, &expansion));
        scrollable(Column::with_children(groups).spacing(12).width(Length::Fill))
            .height(Length::Shrink)
            .into()
    };

    let clear = button::text(fl!("clear-all")).on_press_maybe((!store.is_empty()).then_some(Message::ClearNotifications));
    let bottom = bottom_row(do_not_disturb, clear.into());

    column![container(list).max_height(360.0), bottom]
        .spacing(4)
        .width(Length::Fill)
        .into()
}

/// Do Not Disturb toggler (left) and the Clear button (right).
pub fn bottom_row<'a>(do_not_disturb: bool, clear: Element<'a, Message>) -> Element<'a, Message> {
    row![
        cosmic::widget::toggler(do_not_disturb)
            .label(fl!("do-not-disturb"))
            .on_toggle(Message::SetDnd)
            .width(Length::Fill),
        clear
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn one_line() -> Ellipsize {
    Ellipsize::End(EllipsizeHeightLimit::Lines(1))
}

fn small_icon_button(name: &'static str, msg: Message) -> Element<'static, Message> {
    button::custom(icon::from_name(name).size(16).symbolic(true))
        .on_press(msg)
        .padding(2)
        .class(cosmic::theme::Button::Text)
        .into()
}

/// One app's notifications: a slim header (app name + clear-group) above
/// its card stack; only the newest card shows until "+N more" is pressed.
fn group<'a>(app: String, ns: Vec<&'a Notification>, expansion: &Expansion<'_>) -> Element<'a, Message> {
    let app_label = if app.is_empty() { fl!("unknown-app") } else { app.clone() };
    let header = row![
        text::caption_heading(app_label).ellipsize(one_line()).width(Length::Fill),
        small_icon_button("window-close-symbolic", Message::ClearNotificationGroup(app.clone())),
    ]
    .spacing(8)
    .padding([0, 4])
    .align_y(Alignment::Center);

    let group_expanded = expansion.groups.contains(&app);
    let shown = if group_expanded { ns.len() } else { 1 };
    let mut col = Column::new().spacing(8).width(Length::Fill).push(header);
    for n in &ns[..shown] {
        col = col.push(card(n, expansion.cards.contains(&n.id)));
    }
    if ns.len() > 1 {
        let label = if group_expanded {
            fl!("show-less")
        } else {
            {
            let more = ns.len() - 1;
            fl!("show-more", more = more)
        }
        };
        col = col.push(button::text(label).on_press(Message::ToggleNotificationGroup(app)));
    }
    col.into()
}

/// A rounded card: 32px icon, bold summary + time, one body line (or the
/// full body when expanded), then the expand chevron and dismiss button.
fn card(n: &Notification, expanded: bool) -> Element<'_, Message> {
    let body = normalize_body(&n.body);
    let preview = first_line_preview(&n.body);
    let summary_line = n.summary.lines().next().unwrap_or_default().trim();
    // The text column is narrow (~25-30 chars), so offer expansion whenever
    // there is any body or the summary may be cut off.
    let expandable =
        !body.is_empty() || summary_line.chars().count() > 20 || n.summary.trim().contains('\n');

    let title_row = if expanded {
        row![text::heading(n.summary.trim().to_string()).width(Length::Fill), time_text(n)]
    } else {
        row![
            text::heading(summary_line.to_string()).ellipsize(one_line()).width(Length::Fill),
            time_text(n)
        ]
    }
    .spacing(8)
    .align_y(Alignment::Start);
    let mut content = column![title_row].spacing(2).width(Length::Fill);
    if expanded {
        if !body.is_empty() {
            content = content.push(text::body(body).width(Length::Fill));
        }
    } else if !preview.is_empty() {
        content = content.push(text::body(preview).ellipsize(one_line()).width(Length::Fill));
    }

    let app_icon = n
        .notification_icon()
        .unwrap_or_else(|| icon::from_name("cosmic-applet-notification-symbolic").symbolic(true).icon())
        .size(32);

    let mut controls = row![].spacing(2).align_y(Alignment::Center);
    if expandable {
        let chevron = if expanded { "go-up-symbolic" } else { "go-down-symbolic" };
        controls = controls.push(small_icon_button(chevron, Message::ToggleNotificationExpanded(n.id)));
    }
    controls = controls.push(small_icon_button("window-close-symbolic", Message::DismissNotification(n.id)));

    let inner = row![app_icon, content, controls].spacing(12).align_y(Alignment::Center);
    container(
        button::custom(inner)
            .on_press(Message::ActivateNotification(n.id))
            .class(cosmic::theme::Button::Transparent)
            .padding(12)
            .width(Length::Fill),
    )
    .class(cosmic::theme::Container::Card)
    .width(Length::Fill)
    .into()
}

fn time_text(n: &Notification) -> Element<'static, Message> {
    text::caption(duration_ago(n)).class(cosmic::theme::Text::Custom(dim_text)).into()
}

fn dim_text(theme: &cosmic::Theme) -> cosmic::iced::widget::text::Style {
    let mut c: cosmic::iced::Color = theme.cosmic().on_bg_component_color().into();
    c.a *= 0.6;
    cosmic::iced::widget::text::Style { color: Some(c), ..Default::default() }
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

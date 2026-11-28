// Adapted from pop-os/cosmic-applets cosmic-applet-notifications/src/lib.rs @ 328d142 (GPL-3.0-only)
// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashSet;

use cosmic::{
    iced::{
        advanced::text::{Ellipsize, EllipsizeHeightLimit, Wrapping},
        widget::{column, row},
        Alignment, Length,
    },
    widget::{button, container, icon, scrollable, text, Column},
    Element,
};
use cosmic_notifications_util::{Image, Notification};

use super::{action_buttons, first_line_preview, normalize_body, ChannelStatus, NotificationStore};
use crate::sleeve::{self, caption, color, figures, flat, hairline, Tone, CREAM, GREY, ROW};
use crate::{app::Message, fl};

/// Which cards and groups the user has expanded.
pub struct Expansion<'a> {
    pub cards: &'a HashSet<u32>,
    pub groups: &'a HashSet<String>,
}

/// Notifications set as a tracklist: one "side" per app, newest first,
/// filling the available height, with the DND/Clear catalogue line pinned
/// at the bottom.
pub fn view<'a>(
    store: &'a NotificationStore,
    status: Option<&'a ChannelStatus>,
    expansion: Expansion<'a>,
    do_not_disturb: bool,
) -> Element<'a, Message> {
    let list: Element<'a, Message> = if let Some(ChannelStatus::Unavailable(reason)) = status {
        container(text(fl!("notifications-unavailable", reason = reason.as_str())).size(ROW).class(color(GREY)))
            .padding([8, 0])
            .width(Length::Fill)
            .into()
    } else if store.is_empty() {
        container(
            column![
                icon::from_name("cosmic-applet-notification-symbolic").size(32).symbolic(true).icon(),
                text(fl!("no-notifications")).size(ROW).class(color(GREY)),
            ]
            .spacing(8)
            .align_x(Alignment::Center),
        )
        .center(Length::Fill)
        .into()
    } else {
        let sides = store
            .groups_newest_first()
            .into_iter()
            .map(|(app, ns)| side(app, ns, store, &expansion));
        // `spacing` puts the scrollbar beside the rows instead of over them.
        scrollable(Column::with_children(sides).spacing(20).width(Length::Fill))
            .spacing(8)
            .height(Length::Fill)
            .into()
    };

    let clear = button::custom(text(fl!("clear-all")).size(ROW).font(cosmic::font::bold()))
        .padding([4, 8])
        .on_press_maybe((!store.is_empty()).then_some(Message::ClearNotifications))
        .class(flat(CREAM));
    column![container(list).width(Length::Fill).height(Length::Fill), hairline(), bottom_row(do_not_disturb, clear.into())]
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Do Not Disturb toggler (left) and Clear (right): the sleeve's catalogue line.
pub fn bottom_row<'a>(do_not_disturb: bool, clear: Element<'a, Message>) -> Element<'a, Message> {
    row![
        cosmic::widget::toggler(do_not_disturb)
            .label(fl!("do-not-disturb"))
            .on_toggle(Message::SetDnd)
            .spacing(cosmic::theme::spacing().space_xs)
            .width(Length::Shrink),
        cosmic::widget::Space::new().width(Length::Fill),
        clear
    ]
    .spacing(cosmic::theme::spacing().space_xxs)
    .align_y(Alignment::Center)
    .into()
}

fn one_line() -> Ellipsize {
    Ellipsize::End(EllipsizeHeightLimit::Lines(1))
}

fn close_button(msg: Message) -> Element<'static, Message> {
    button::custom(sleeve::close_glyph(14))
        .on_press(msg)
        .padding(4)
        .class(flat(GREY))
        .into()
}

fn chevron_button(expanded: bool, msg: Message) -> Element<'static, Message> {
    let name = if expanded { "go-up-symbolic" } else { "go-down-symbolic" };
    button::custom(icon::from_name(name).size(14).symbolic(true)).on_press(msg).padding(4).class(flat(GREY)).into()
}

/// One app's side: tracked app name and count over a hairline, then its
/// rows; only the newest row shows until "+N more" is pressed.
fn side<'a>(app: String, ns: Vec<&'a Notification>, store: &NotificationStore, expansion: &Expansion<'_>) -> Element<'a, Message> {
    let app_label = if app.is_empty() { fl!("unknown-app") } else { app.clone() };
    let header = row![
        caption::<Message>(app_label).ellipsize(one_line()).width(Length::Fill),
        figures(&ns.len().to_string(), sleeve::CAPTION, true, GREY),
        close_button(Message::ClearNotificationGroup(app.clone())),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let group_expanded = expansion.groups.contains(&app);
    let shown = if group_expanded { ns.len() } else { 1 };
    let mut col = Column::new().spacing(0).width(Length::Fill).push(header).push(hairline());
    for (i, n) in ns[..shown].iter().enumerate() {
        if i > 0 {
            col = col.push(hairline());
        }
        col = col.push(track(n, expansion.cards.contains(&n.id), store.is_unread(n.id)));
    }
    if ns.len() > 1 {
        let label = if group_expanded { fl!("show-less") } else { { let more = ns.len() - 1; fl!("show-more", more = more) } };
        col = col.push(hairline()).push(
            button::custom(text(label).size(12).font(cosmic::font::bold()))
                .padding([6, 0])
                .on_press(Message::ToggleNotificationGroup(app))
                .class(flat(GREY)),
        );
    }
    col.into()
}

/// Text a row shows: summary (one line unless expanded), and the body (first
/// line when collapsed, all of it when expanded) if there is one.
#[derive(Debug, PartialEq, Eq)]
struct CardText {
    summary: String,
    body: Option<String>,
    expandable: bool,
}

/// Characters that fit on one row line (about 300px of 14px text).
const ONE_LINE_CHARS: usize = 40;

fn card_text(n: &Notification, expanded: bool) -> CardText {
    let body = normalize_body(&n.body);
    let summary_line = n.summary.lines().next().unwrap_or_default().trim();
    // The text column is narrow (~35 chars), so offer expansion whenever
    // there is any body or the summary may be cut off.
    // Only offer expansion when it reveals something: a body longer than
    // the one line a collapsed row shows, a summary that may be cut off, or
    // the app's action buttons.
    let expandable = body.contains('\n')
        || body.chars().count() > ONE_LINE_CHARS
        || summary_line.chars().count() > ONE_LINE_CHARS
        || n.summary.trim().contains('\n')
        || !action_buttons(&n.actions).is_empty();
    let (summary, body) = if expanded {
        (n.summary.trim().to_string(), body)
    } else {
        (summary_line.to_string(), first_line_preview(&n.body))
    };
    CardText { summary, body: (!body.is_empty()).then_some(body), expandable }
}

/// Local wall-clock `HH:MM` of `t`; empty if it cannot be resolved.
fn clock_label(t: std::time::SystemTime) -> String {
    jiff::Timestamp::try_from(t)
        .map(|ts| ts.to_zoned(jiff::tz::TimeZone::system()).strftime("%H:%M").to_string())
        .unwrap_or_default()
}

/// The row's picture at 24px: the sender picture the app sent, else the
/// app's icon (an unresolvable name falls back to a generic one, never a
/// blank slot).
fn picture(n: &Notification) -> Element<'static, Message> {
    let fallback = || icon::from_name("cosmic-applet-notification-symbolic").symbolic(true).icon();
    let named = match n.image() {
        Some(Image::Name(name)) => Some(name.as_str()),
        Some(_) => None,
        None => Some(n.app_icon.as_str()).filter(|a| !a.is_empty() && !a.starts_with("file://") && !a.starts_with('/')),
    };
    match named {
        Some(name) if icon::from_name(name).path().is_none() => fallback(),
        _ => n.notification_icon().unwrap_or_else(fallback),
    }
    .size(24)
    .into()
}

/// A tracklist row: picture (sender or app icon), bold summary over a
/// dimmed body line, then the time flush right (destructive colour when
/// critical) with the unread dot, chevron and dismiss. Expanded rows show the full body and the app's
/// actions underneath.
///
/// The picture + text area is the activate target; the controls are
/// its siblings, so pressing them never activates the notification.
fn track(n: &Notification, expanded: bool, unread: bool) -> Element<'_, Message> {
    let t = card_text(n, expanded);
    let critical = n.urgency() == 2;

    let lead = container(picture(n)).width(Length::Fixed(28.0)).padding([1, 0, 0, 0]);

    let summary = text(t.summary).size(ROW).font(cosmic::font::bold()).width(Length::Fill).class(color(CREAM));
    // Expanded text may hold long unbroken runs (paths, URLs): let them
    // break inside the word so expanding always shows all of it.
    let summary = if expanded { summary.wrapping(Wrapping::WordOrGlyph) } else { summary.ellipsize(one_line()) };
    let mut texts = column![summary].spacing(2).width(Length::Fill);
    if let Some(body) = t.body {
        let body = text(body).size(ROW).width(Length::Fill).class(color(GREY));
        texts = texts.push(if expanded { body.wrapping(Wrapping::WordOrGlyph) } else { body.ellipsize(one_line()) });
    }

    let body_area = cosmic::widget::mouse_area(row![lead, texts].spacing(6).align_y(Alignment::Start).width(Length::Fill))
        .on_press(Message::ActivateNotification(n.id))
        .interaction(cosmic::iced::mouse::Interaction::Pointer);

    let mut meta = row![].spacing(6).align_y(Alignment::Center);
    if unread {
        meta = meta.push(sleeve::dot());
    }
    meta = meta.push(figures(&clock_label(n.time), ROW, critical, if critical { Tone::Critical } else { GREY }));
    if t.expandable {
        meta = meta.push(chevron_button(expanded, Message::ToggleNotificationExpanded(n.id)));
    }
    meta = meta.push(close_button(Message::DismissNotification(n.id)));

    let mut content = column![row![body_area, meta].spacing(8).align_y(Alignment::Start)].spacing(6).width(Length::Fill);
    let buttons = action_buttons(&n.actions);
    if expanded && !buttons.is_empty() {
        let mut actions = row![].spacing(4);
        for (action, label) in buttons {
            actions = actions.push(
                button::custom(text(label).size(ROW).font(cosmic::font::bold()).ellipsize(one_line()))
                    .padding([4, 8])
                    .on_press(Message::InvokeAction(n.id, action.to_string()))
                    .class(flat(CREAM)),
            );
        }
        // Pull the first label in line with the summary above it.
        content = content.push(container(actions).padding([0.0, 0.0, 0.0, 34.0 - 8.0]));
    }
    container(content).padding([10, 0]).width(Length::Fill).into()
}

#[cfg(test)]
mod tests {

    #[test]
    fn desktop_entry_has_no_timer_text() {
        let entry = include_str!("../../data/com.system76.CosmicAppletNotifications.desktop");
        assert!(!entry.to_lowercase().contains("timer"), "{entry}");
    }

    use super::*;
    use std::time::SystemTime;

    fn flameshot() -> Notification {
        Notification {
            id: 1,
            app_name: "flameshot".into(),
            app_icon: String::new(),
            summary: "Flameshot Info".into(),
            body: "Screenshot aborted.".into(),
            actions: Vec::new(),
            hints: Vec::new(),
            expire_timeout: -1,
            time: SystemTime::now(),
        }
    }

    #[test]
    fn collapsed_card_shows_summary_and_preview() {
        let t = card_text(&flameshot(), false);
        assert_eq!(t.summary, "Flameshot Info");
        assert_eq!(t.body.as_deref(), Some("Screenshot aborted."));
        // Short body, no actions: nothing more to show, so no chevron.
        assert!(!t.expandable);
    }

    #[test]
    fn expanded_card_shows_full_body() {
        let mut n = flameshot();
        n.body = "line one\nline two".into();
        let t = card_text(&n, true);
        assert_eq!(t.body.as_deref(), Some("line one\nline two"));
        assert_eq!(card_text(&n, false).body.as_deref(), Some("line one"));
    }

    #[test]
    fn long_or_multiline_body_or_actions_are_expandable() {
        let mut n = flameshot();
        n.body = "x".repeat(60);
        assert!(card_text(&n, false).expandable);
        n.body = "one\ntwo".into();
        assert!(card_text(&n, false).expandable);
        n.body = "hello".into();
        n.actions = vec![("reply".parse().unwrap(), "Reply".into())];
        assert!(card_text(&n, false).expandable);
    }

    #[test]
    fn summary_only_card_has_no_body_line() {
        let mut n = flameshot();
        n.body = String::new();
        n.summary = "Short".into();
        let t = card_text(&n, false);
        assert_eq!(t.summary, "Short");
        assert_eq!(t.body, None);
        assert!(!t.expandable);
    }

    #[test]
    fn dismiss_icon_does_not_depend_on_icon_theme() {
        // Themes may blank window-close-symbolic (e.g. to hide titlebar close buttons).
        assert!(std::str::from_utf8(sleeve::CLOSE_SVG).unwrap().contains("<path"));
    }




}

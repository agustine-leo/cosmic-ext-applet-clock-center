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

use super::{action_buttons, first_line_preview, normalize_body, ChannelStatus, NotificationStore};
use crate::{app::Message, fl};

/// Which cards and groups the user has expanded.
pub struct Expansion<'a> {
    pub cards: &'a HashSet<u32>,
    pub groups: &'a HashSet<String>,
}

/// Notifications grouped per app filling the available height, with the
/// DND/Clear row pinned at the bottom.
pub fn view<'a>(
    store: &'a NotificationStore,
    status: Option<&'a ChannelStatus>,
    expansion: Expansion<'a>,
    do_not_disturb: bool,
) -> Element<'a, Message> {
    let space = cosmic::theme::spacing();
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
        .center(Length::Fill)
        .into()
    } else {
        let groups = store
            .groups_newest_first()
            .into_iter()
            .map(|(app, ns)| group(app, ns, &expansion));
        // `spacing` puts the scrollbar beside the cards instead of over them.
        scrollable(Column::with_children(groups).spacing(space.space_xs).width(Length::Fill))
            .spacing(space.space_xxs)
            .height(Length::Fill)
            .into()
    };

    let clear = button::standard(fl!("clear-all")).on_press_maybe((!store.is_empty()).then_some(Message::ClearNotifications));
    let bottom = bottom_row(do_not_disturb, clear.into());

    column![container(list).width(Length::Fill).height(Length::Fill), bottom]
        .spacing(space.space_xs)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Do Not Disturb toggler (left) and the Clear button (right).
pub fn bottom_row<'a>(do_not_disturb: bool, clear: Element<'a, Message>) -> Element<'a, Message> {
    row![
        // Label and switch stay together on the left (GNOME); the gap goes before Clear.
        cosmic::widget::toggler(do_not_disturb)
            .label(fl!("do-not-disturb"))
            .on_toggle(Message::SetDnd)
            .spacing(cosmic::theme::spacing().space_xs)
            .width(Length::Shrink),
        cosmic::widget::Space::new().width(Length::Fill),
        clear
    ]
    .spacing(cosmic::theme::spacing().space_xxs)
    .padding(edge_padding(cosmic::theme::spacing().space_xs))
    .align_y(Alignment::Center)
    .into()
}

/// Right inset for rows outside a card (group header, DND/Clear row) so their
/// right-most control lines up with the controls inside the cards, which sit
/// `card_padding` in from the card edge.
fn edge_padding(card_padding: u16) -> cosmic::iced::Padding {
    cosmic::iced::Padding { right: f32::from(card_padding), ..cosmic::iced::Padding::ZERO }
}

fn one_line() -> Ellipsize {
    Ellipsize::End(EllipsizeHeightLimit::Lines(1))
}

/// Dismiss glyph, embedded so icon themes that blank `window-close-symbolic`
/// (to hide titlebar close buttons) do not hide it.
const CLOSE_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><path fill="#232323" d="M4.3 4.3a1 1 0 0 1 1.4 0L8 6.6l2.3-2.3a1 1 0 1 1 1.4 1.4L9.4 8l2.3 2.3a1 1 0 0 1-1.4 1.4L8 9.4l-2.3 2.3a1 1 0 0 1-1.4-1.4L6.6 8 4.3 5.7a1 1 0 0 1 0-1.4z"/></svg>"##;

fn small_icon_button(name: &'static str, msg: Message) -> Element<'static, Message> {
    button::custom(icon::from_name(name).size(16).symbolic(true))
        .on_press(msg)
        .padding(2)
        .class(cosmic::theme::Button::Text)
        .into()
}

fn close_button(msg: Message) -> Element<'static, Message> {
    button::custom(icon::icon(icon::from_svg_bytes(CLOSE_SVG).symbolic(true)).size(16))
        .on_press(msg)
        .padding(2)
        .class(cosmic::theme::Button::Text)
        .into()
}

/// One app's notifications: a slim header (app name + clear-group) above
/// its card stack; only the newest card shows until "+N more" is pressed.
fn group<'a>(app: String, ns: Vec<&'a Notification>, expansion: &Expansion<'_>) -> Element<'a, Message> {
    let space = cosmic::theme::spacing();
    let app_label = if app.is_empty() { fl!("unknown-app") } else { app.clone() };
    let header = row![
        text::caption_heading(app_label).ellipsize(one_line()).width(Length::Fill),
        close_button(Message::ClearNotificationGroup(app.clone())),
    ]
    .spacing(space.space_xxs)
    .padding(edge_padding(space.space_xs))
    .align_y(Alignment::Center);

    let group_expanded = expansion.groups.contains(&app);
    let shown = if group_expanded { ns.len() } else { 1 };
    let mut col = Column::new().spacing(space.space_xxs).width(Length::Fill).push(header);
    for n in &ns[..shown] {
        col = col.push(card(n, expansion.cards.contains(&n.id)));
    }
    if ns.len() > 1 {
        let label = if group_expanded {
            fl!("show-less")
        } else {
            let more = ns.len() - 1;
            fl!("show-more", more = more)
        };
        // No horizontal padding: the label's left edge lines up with the cards.
        col = col.push(
            button::text(label)
                .padding([space.space_xxxs, 0])
                .on_press(Message::ToggleNotificationGroup(app)),
        );
    }
    col.into()
}

/// Text a card shows: summary (one line unless expanded), and the body (first
/// line when collapsed, all of it when expanded) if there is one.
#[derive(Debug, PartialEq, Eq)]
struct CardText {
    summary: String,
    body: Option<String>,
    expandable: bool,
}

fn card_text(n: &Notification, expanded: bool) -> CardText {
    let body = normalize_body(&n.body);
    let summary_line = n.summary.lines().next().unwrap_or_default().trim();
    // The text column is narrow (~25-30 chars), so offer expansion whenever
    // there is any body or the summary may be cut off.
    let expandable = !body.is_empty() || summary_line.chars().count() > 20 || n.summary.trim().contains('\n');
    let (summary, body) = if expanded {
        (n.summary.trim().to_string(), body)
    } else {
        (summary_line.to_string(), first_line_preview(&n.body))
    };
    CardText { summary, body: (!body.is_empty()).then_some(body), expandable }
}

/// A rounded card: 32px icon, bold summary + time, one body line (or the
/// full body when expanded), then the expand chevron and dismiss button.
///
/// Layout: `container(Card, Fill) > row(Fill) [ mouse_area(row(Fill) [icon, text column(Fill)]), controls ]`.
/// The icon + text area is the activate target; the chevron and dismiss buttons are
/// its siblings, so pressing them never activates the notification. Text is not inside
/// a `Button::Transparent` (whose text colour is fully transparent in libcosmic).
fn card(n: &Notification, expanded: bool) -> Element<'_, Message> {
    let space = cosmic::theme::spacing();
    let t = card_text(n, expanded);

    let summary = if expanded {
        text::heading(t.summary).width(Length::Fill)
    } else {
        text::heading(t.summary).ellipsize(one_line()).width(Length::Fill)
    };
    let title_row = row![summary, time_text(n)].spacing(8).align_y(Alignment::Start).width(Length::Fill);
    let mut content = column![title_row].spacing(2).width(Length::Fill);
    if let Some(body) = t.body {
        content = content.push(if expanded {
            text::body(body).width(Length::Fill)
        } else {
            text::body(body).ellipsize(one_line()).width(Length::Fill)
        });
    }

    let app_icon = n
        .notification_icon()
        .unwrap_or_else(|| icon::from_name("cosmic-applet-notification-symbolic").symbolic(true).icon())
        .size(32);

    let body_area = cosmic::widget::mouse_area(
        row![app_icon, content].spacing(space.space_xs).align_y(Alignment::Center).width(Length::Fill),
    )
    .on_press(Message::ActivateNotification(n.id))
    .interaction(cosmic::iced::mouse::Interaction::Pointer);

    let mut controls = row![].spacing(2).align_y(Alignment::Center);
    if t.expandable {
        let chevron = if expanded { "go-up-symbolic" } else { "go-down-symbolic" };
        controls = controls.push(small_icon_button(chevron, Message::ToggleNotificationExpanded(n.id)));
    }
    controls = controls.push(close_button(Message::DismissNotification(n.id)));

    let top = row![body_area, controls].spacing(space.space_xxs).align_y(Alignment::Center).width(Length::Fill);
    let mut content = column![top].spacing(space.space_xs).width(Length::Fill);
    let buttons = action_buttons(&n.actions);
    if !buttons.is_empty() {
        // GNOME-style action row: equal-width buttons separated by thin dividers.
        let mut row_ = row![].width(Length::Fill).align_y(Alignment::Center);
        for (i, (action, label)) in buttons.into_iter().enumerate() {
            if i > 0 {
                row_ = row_.push(cosmic::widget::divider::vertical::default().height(Length::Fixed(20.0)));
            }
            row_ = row_.push(
                button::custom(container(text::body(label)).center_x(Length::Fill))
                    .width(Length::Fill)
                    .class(cosmic::theme::Button::Text)
                    .on_press(Message::InvokeAction(n.id, action.to_string())),
            );
        }
        content = content.push(cosmic::widget::divider::horizontal::default()).push(row_);
    }
    container(content)
        .padding(space.space_xs)
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

#[cfg(test)]
mod tests {
    #[test]
    fn edge_padding_insets_right_by_card_padding() {
        assert_eq!(super::edge_padding(12), cosmic::iced::Padding { right: 12.0, ..cosmic::iced::Padding::ZERO });
    }

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
        assert!(t.expandable);
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
        assert!(std::str::from_utf8(CLOSE_SVG).unwrap().contains("<path"));
    }
}

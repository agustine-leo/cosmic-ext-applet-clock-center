// SPDX-License-Identifier: GPL-3.0-only
// Adapted from pop-os/cosmic-applets cosmic-applet-time/src/window.rs @ 328d142 (GPL-3.0-only)

use cosmic::{
    iced::Length,
    widget::{button, column, container, row, text},
    Apply, Element,
};
use jiff::civil::{Date, Weekday};

use crate::app::Message;
use crate::sleeve::{accent_cell, caption, color, flat, hairline, Tone, CREAM, GREY};

fn week_start(first_day_of_week: u8) -> Weekday {
    match first_day_of_week {
        0 => Weekday::Monday,
        1 => Weekday::Tuesday,
        2 => Weekday::Wednesday,
        3 => Weekday::Thursday,
        4 => Weekday::Friday,
        5 => Weekday::Saturday,
        _ => Weekday::Sunday,
    }
}

/// Rows of day numbers for a month; `None` is padding.
pub fn month_grid(year: i16, month: i8, first_day_of_week: u8) -> Vec<[Option<i8>; 7]> {
    let Ok(first) = Date::new(year, month, 1) else {
        return Vec::new();
    };
    let offset = first.weekday().since(week_start(first_day_of_week)) as usize;
    let days = first.days_in_month();
    let mut rows = Vec::new();
    let mut row = [None; 7];
    let mut col = offset;
    for d in 1..=days {
        row[col] = Some(d);
        col += 1;
        if col == 7 {
            rows.push(row);
            row = [None; 7];
            col = 0;
        }
    }
    if col > 0 {
        rows.push(row);
    }
    rows
}

/// A day cell. Days are not clickable, so they are plain text (a button
/// without `on_press` renders dimmed as disabled in libcosmic). Today is a
/// accent cell with bold figures.
fn day_cell(day: i8, is_today: bool) -> Element<'static, Message> {
    let label = text(format!("{day}")).size(13);
    let cell = if is_today {
        container(label.font(cosmic::font::bold()).class(color(Tone::OnAccent))).class(accent_cell())
    } else {
        container(label.class(color(CREAM)))
    };
    container(cell.center(Length::Fixed(CELL))).center_x(Length::Fill).height(Length::Fixed(CELL + 2.0)).into()
}

const CELL: f32 = 30.0;

/// `shown` picks the month on screen; the header always shows today:
/// the day figure at large scale, weekday and month under it.
pub fn view<'a>(shown: Date, today: Date, first_day_of_week: u8) -> Element<'a, Message> {
    let start = week_start(first_day_of_week);
    let header = row![
        text(today.strftime("%-d").to_string()).size(64).font(cosmic::font::bold()).class(color(CREAM)),
        column![
            text(today.strftime("%A").to_string()).size(15).font(cosmic::font::bold()).class(color(CREAM)),
            text(today.strftime("%B %Y").to_string()).size(13).class(color(GREY)),
        ]
        .spacing(2),
    ]
    .spacing(12)
    .align_y(cosmic::iced::Alignment::End);

    let arrow = |name: &'static str, msg: Message| {
        button::custom(cosmic::widget::icon::from_name(name).size(14).symbolic(true)).padding(4).on_press(msg).class(flat(GREY))
    };
    let nav = row::with_capacity(3)
        .width(Length::Fill)
        .push(arrow("go-previous-symbolic", Message::PrevMonth))
        .push(
            text(shown.strftime("%B %Y").to_string())
                .size(13)
                .font(cosmic::font::bold())
                .class(color(CREAM))
                .apply(container)
                .center_x(Length::Fill),
        )
        .push(arrow("go-next-symbolic", Message::NextMonth))
        .align_y(cosmic::iced::Alignment::Center);

    let mut weekdays = row::with_capacity(7).width(Length::Fill);
    let mut wd = start;
    for _ in 0..7 {
        let name = Date::new(2024, 1, 1)
            .ok()
            .and_then(|d| d.nth_weekday(1, wd).ok())
            .map(|d| d.strftime("%a").to_string())
            .unwrap_or_default();
        weekdays = weekdays.push(caption::<Message>(name).apply(container).center_x(Length::Fill));
        wd = wd.next();
    }

    let mut grid = column::with_capacity(7).width(Length::Fill).spacing(2).push(weekdays);
    for week in month_grid(shown.year(), shown.month(), first_day_of_week) {
        let mut r = row::with_capacity(7).width(Length::Fill);
        for cell in week {
            r = r.push(match cell {
                Some(d) => day_cell(d, Date::new(shown.year(), shown.month(), d).is_ok_and(|x| x == today)),
                None => Element::from(cosmic::widget::Space::new().width(Length::Fill).height(Length::Fixed(CELL + 2.0))),
            });
        }
        grid = grid.push(r);
    }

    column::with_capacity(5)
        .width(Length::Fill)
        .push(header)
        .push(hairline())
        .push(nav)
        .push(grid)
        .spacing(10)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nov_2026_sunday_start() {
        // Nov 1 2026 is a Sunday
        let g = month_grid(2026, 11, 6);
        assert_eq!(g[0], [Some(1), Some(2), Some(3), Some(4), Some(5), Some(6), Some(7)]);
        assert_eq!(*g.last().unwrap(), [Some(29), Some(30), None, None, None, None, None]);
        assert_eq!(g.iter().flatten().flatten().count(), 30);
    }
    #[test]
    fn nov_2026_monday_start() {
        let g = month_grid(2026, 11, 0);
        assert_eq!(g[0], [None, None, None, None, None, None, Some(1)]);
    }
    #[test]
    fn nov_2026_saturday_start() {
        let g = month_grid(2026, 11, 5);
        assert_eq!(g[0], [None, Some(1), Some(2), Some(3), Some(4), Some(5), Some(6)]);
    }
    #[test]
    fn feb_2028_leap() {
        assert_eq!(month_grid(2028, 2, 6).iter().flatten().flatten().count(), 29);
    }
}

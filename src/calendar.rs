// SPDX-License-Identifier: GPL-3.0-only
// Adapted from pop-os/cosmic-applets cosmic-applet-time/src/window.rs @ 328d142 (GPL-3.0-only)

use cosmic::{
    iced::Length,
    widget::{button, column, container, row, text, Button},
    Apply, Element,
};
use jiff::civil::{Date, Weekday};

use crate::app::Message;

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

fn day_button(day: i8, is_selected: bool, is_today: bool) -> Button<'static, Message> {
    let style = if is_today {
        button::ButtonClass::Suggested
    } else if is_selected {
        button::ButtonClass::Standard
    } else {
        button::ButtonClass::Text
    };
    button::custom(text::body(format!("{day}")).apply(container).center(Length::Fill))
        .class(style)
        .height(Length::Fixed(40.0))
        .width(Length::Fill)
}

pub fn view<'a>(selected: Date, today: Date, first_day_of_week: u8) -> Element<'a, Message> {
    let start = week_start(first_day_of_week);
    let header = column::with_capacity(2)
        .push(text::title3(selected.strftime("%A").to_string()))
        .push(text::body(selected.strftime("%B %-d, %Y").to_string()));

    let nav = row::with_capacity(3)
        .push(button::icon(cosmic::widget::icon::from_name("go-previous-symbolic")).on_press(Message::PrevMonth))
        .push(
            text::heading(selected.strftime("%B %Y").to_string())
                .apply(container)
                .center_x(Length::Fill),
        )
        .push(button::icon(cosmic::widget::icon::from_name("go-next-symbolic")).on_press(Message::NextMonth))
        .align_y(cosmic::iced::Alignment::Center);

    let mut weekdays = row::with_capacity(7).width(Length::Fill);
    let mut wd = start;
    for _ in 0..7 {
        let name = Date::new(2024, 1, 1)
            .ok()
            .and_then(|d| d.nth_weekday(1, wd).ok())
            .map(|d| d.strftime("%a").to_string())
            .unwrap_or_default();
        weekdays = weekdays.push(text::caption(name).apply(container).center_x(Length::Fill));
        wd = wd.next();
    }

    let mut grid = column::with_capacity(6).width(Length::Fill).push(weekdays);
    for week in month_grid(selected.year(), selected.month(), first_day_of_week) {
        let mut r = row::with_capacity(7).width(Length::Fill);
        for cell in week {
            r = r.push(match cell {
                Some(d) => {
                    let is_today = Date::new(selected.year(), selected.month(), d).is_ok_and(|x| x == today);
                    Element::from(day_button(d, d == selected.day(), is_today))
                }
                None => Element::from(cosmic::widget::Space::new().width(Length::Fill).height(Length::Fixed(40.0))),
            });
        }
        grid = grid.push(r);
    }

    column::with_capacity(3)
        .width(Length::Fill)
        .push(header)
        .push(nav)
        .push(grid)
        .spacing(8)
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

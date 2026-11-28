// SPDX-License-Identifier: GPL-3.0-only
//! Liner-note credits under the calendar: world clocks and weather.

use std::time::Duration;

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{column, row, text};
use cosmic::Element;
use serde::Deserialize;

use crate::app::Message;
use crate::config::{WeatherPlace, WorldClock};
use crate::fl;
use crate::sleeve::{caption, color, figures, hairline, CREAM, GREY, ROW};

/// How often weather is refetched.
pub const WEATHER_EVERY: Duration = Duration::from_secs(30 * 60);

/// Current conditions and today's range.
#[derive(Debug, Clone, PartialEq)]
pub struct Weather {
    pub temperature: f32,
    pub code: u8,
    pub high: f32,
    pub low: f32,
}

#[derive(Deserialize)]
struct Resp {
    current: Current,
    daily: Daily,
}
#[derive(Deserialize)]
struct Current {
    temperature_2m: f32,
    weather_code: u8,
}
#[derive(Deserialize)]
struct Daily {
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
}

pub fn parse(json: &str) -> Option<Weather> {
    let r: Resp = serde_json::from_str(json).ok()?;
    Some(Weather {
        temperature: r.current.temperature_2m,
        code: r.current.weather_code,
        high: *r.daily.temperature_2m_max.first()?,
        low: *r.daily.temperature_2m_min.first()?,
    })
}

pub fn url(p: &WeatherPlace) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={:.2}&longitude={:.2}\
         &current=temperature_2m,weather_code&daily=temperature_2m_max,temperature_2m_min\
         &timezone=auto&forecast_days=1",
        p.latitude, p.longitude
    )
}

/// Fetch with the system `curl` (no TLS stack in this applet). Only the
/// place's coordinates are sent.
pub async fn fetch(p: WeatherPlace) -> Option<Weather> {
    let out = tokio::process::Command::new("curl")
        .args(["-fsS", "--max-time", "10", &url(&p)])
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        tracing::warn!("weather fetch failed: {}", String::from_utf8_lossy(&out.stderr).trim());
        return None;
    }
    parse(&String::from_utf8_lossy(&out.stdout))
}

/// WMO weather interpretation code to a short label.
pub fn condition(code: u8) -> String {
    match code {
        0 => fl!("wmo-clear"),
        1 => fl!("wmo-mainly-clear"),
        2 => fl!("wmo-partly-cloudy"),
        3 => fl!("wmo-overcast"),
        45 | 48 => fl!("wmo-fog"),
        51..=57 => fl!("wmo-drizzle"),
        61..=67 => fl!("wmo-rain"),
        71..=77 | 85 | 86 => fl!("wmo-snow"),
        80..=82 => fl!("wmo-showers"),
        95..=99 => fl!("wmo-thunderstorm"),
        _ => String::new(),
    }
}

/// `+1h`, `−7h`, `+5:30h` from an offset difference in seconds; empty when equal.
pub fn offset_label(diff_secs: i32) -> String {
    if diff_secs == 0 {
        return String::new();
    }
    let a = diff_secs.unsigned_abs();
    let (h, m) = (a / 3600, (a % 3600) / 60);
    let hours = if m == 0 { h.to_string() } else { format!("{h}:{m:02}") };
    if diff_secs > 0 { fl!("hours-ahead", hours = hours) } else { fl!("hours-behind", hours = hours) }
}

/// Time in `zone` as `HH:MM` plus its offset from local; `None` for an unknown zone.
pub fn zone_time(zone: &str, now: &jiff::Zoned) -> Option<(String, String)> {
    let tz = jiff::tz::TimeZone::get(zone).ok()?;
    let there = now.with_time_zone(tz);
    let diff = there.offset().seconds() - now.offset().seconds();
    Some((there.strftime("%H:%M").to_string(), offset_label(diff)))
}

fn credit_row<'a>(label: String, value: Element<'a, Message>, note: String) -> Element<'a, Message> {
    row![
        text(label).size(ROW).font(cosmic::font::bold()).class(color(CREAM)).width(Length::Fill),
        text(note).size(12).class(color(GREY)),
        value,
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

/// World clocks and weather, each under a tracked caption; nothing when
/// neither is configured.
pub fn view<'a>(clocks: &'a [WorldClock], place: Option<&'a WeatherPlace>, weather: Option<&'a Weather>, now: &jiff::Zoned) -> Element<'a, Message> {
    let mut col = column::with_capacity(6).spacing(8).width(Length::Fill);
    let rows: Vec<_> = clocks.iter().filter_map(|c| zone_time(&c.zone, now).map(|t| (c, t))).collect();
    if !rows.is_empty() {
        col = col.push(caption::<Message>(fl!("world-clock")));
        for (c, (time, off)) in rows {
            col = col.push(credit_row(c.label.clone(), figures(&time, ROW, true, CREAM), off));
        }
    }
    if let Some(place) = place {
        if col_has_children(clocks, now) {
            col = col.push(hairline());
        }
        col = col.push(caption::<Message>(fl!("weather")));
        col = col.push(match weather {
            Some(w) => column![
                credit_row(place.name.clone(), figures(&format!("{:.0}°", w.temperature), 20, true, CREAM), condition(w.code)),
                text(fl!("weather-high-low", high = format!("{:.0}", w.high), low = format!("{:.0}", w.low)))
                    .size(12)
                    .class(color(GREY)),
            ]
            .spacing(0)
            .into(),
            // Not fetched yet, or every fetch so far failed: an honest dash.
            None => Element::from(credit_row(place.name.clone(), figures("—", 20, true, GREY), String::new())),
        });
    }
    col.into()
}

fn col_has_children(clocks: &[WorldClock], now: &jiff::Zoned) -> bool {
    clocks.iter().any(|c| zone_time(&c.zone, now).is_some())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_open_meteo() {
        let j = r#"{"current":{"temperature_2m":31.4,"weather_code":95},"daily":{"temperature_2m_max":[33.1],"temperature_2m_min":[24.6]}}"#;
        assert_eq!(parse(j), Some(Weather { temperature: 31.4, code: 95, high: 33.1, low: 24.6 }));
        assert_eq!(parse("{}"), None);
    }

    #[test]
    fn url_sends_only_rounded_coordinates() {
        let u = url(&WeatherPlace { name: "Kuala Lumpur".into(), latitude: 3.1390, longitude: 101.6869 });
        assert!(u.contains("latitude=3.14&longitude=101.69"), "{u}");
        assert!(!u.contains("Kuala"));
    }

    /// Fluent wraps arguments in bidi isolation marks; drop them to compare.
    fn plain(s: String) -> String {
        s.replace(['\u{2068}', '\u{2069}'], "")
    }

    #[test]
    fn offsets_format() {
        crate::localize::localize();
        assert_eq!(offset_label(0), "");
        assert_eq!(plain(offset_label(3600)), "+1h");
        assert_eq!(plain(offset_label(-8 * 3600)), "−8h");
        assert_eq!(plain(offset_label(5 * 3600 + 1800)), "+5:30h");
    }

    #[test]
    fn tokyo_is_one_hour_ahead_of_singapore() {
        crate::localize::localize();
        let now = "2026-11-28T14:00:00+08:00[Asia/Singapore]".parse::<jiff::Zoned>().unwrap();
        let (time, off) = zone_time("Asia/Tokyo", &now).unwrap();
        assert_eq!(time, "15:00");
        assert_eq!(plain(off), "+1h");
        assert!(zone_time("Nowhere/Nope", &now).is_none());
    }

    #[test]
    fn thunderstorm_code_has_label() {
        crate::localize::localize();
        assert!(!condition(95).is_empty());
        assert!(condition(200).is_empty());
    }
}

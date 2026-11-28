// SPDX-License-Identifier: GPL-3.0-only
//! Liner-note credits under the calendar: world clocks and weather.

use std::time::Duration;

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, row, scrollable, text};
use cosmic::Element;
use serde::Deserialize;

use crate::app::Message;
use crate::config::{WeatherPlace, WorldClock, MAX_WEATHER_PLACES};
use crate::fl;
use crate::sleeve::{caption, color, figures, flat, hairline, CREAM, GREY, ROW};

/// How often weather is refetched.
pub const WEATHER_EVERY: Duration = Duration::from_secs(30 * 60);

/// Current conditions and today's range.
#[derive(Debug, Clone, PartialEq)]
pub struct Weather {
    pub temperature: f32,
    pub code: u8,
    pub high: f32,
    pub low: f32,
    pub is_day: bool,
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
    #[serde(default = "day")]
    is_day: u8,
}
fn day() -> u8 {
    1
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
        is_day: r.current.is_day != 0,
    })
}

pub fn url(p: &WeatherPlace) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={:.2}&longitude={:.2}\
         &current=temperature_2m,weather_code,is_day&daily=temperature_2m_max,temperature_2m_min\
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

/// A city from Open-Meteo's geocoding search.
#[derive(Debug, Clone, PartialEq)]
pub struct City {
    pub name: String,
    /// "Region, Country", for telling same-named cities apart.
    pub detail: String,
    pub latitude: f64,
    pub longitude: f64,
    pub timezone: Option<String>,
}

#[derive(Deserialize)]
struct GeoResp {
    #[serde(default)]
    results: Vec<GeoItem>,
}
#[derive(Deserialize)]
struct GeoItem {
    name: String,
    latitude: f64,
    longitude: f64,
    timezone: Option<String>,
    admin1: Option<String>,
    country: Option<String>,
}

pub fn parse_cities(json: &str) -> Option<Vec<City>> {
    let r: GeoResp = serde_json::from_str(json).ok()?;
    Some(
        r.results
            .into_iter()
            .map(|g| City {
                detail: [g.admin1, g.country].into_iter().flatten().filter(|x| *x != g.name).collect::<Vec<_>>().join(", "),
                name: g.name,
                latitude: g.latitude,
                longitude: g.longitude,
                timezone: g.timezone,
            })
            .collect(),
    )
}

impl City {
    pub fn place(&self) -> WeatherPlace {
        WeatherPlace { name: self.name.clone(), latitude: self.latitude, longitude: self.longitude }
    }
}

/// Percent-encode a query string component.
pub fn encode(q: &str) -> String {
    q.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn geocode_url(query: &str) -> String {
    format!("https://geocoding-api.open-meteo.com/v1/search?name={}&count=4&language=en&format=json", encode(query.trim()))
}

/// Search cities by name (the typed text is sent to Open-Meteo). `None` on
/// a network or parse failure, `Some(vec![])` when nothing matched.
pub async fn geocode(query: String) -> Option<Vec<City>> {
    let out = tokio::process::Command::new("curl")
        .args(["-fsS", "--max-time", "10", &geocode_url(&query)])
        .output()
        .await
        .ok()?;
    if !out.status.success() {
        tracing::warn!("city search failed: {}", String::from_utf8_lossy(&out.stderr).trim());
        return None;
    }
    parse_cities(&String::from_utf8_lossy(&out.stdout))
}

/// Where the settings search stands.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Search {
    #[default]
    Idle,
    Searching,
    Found(Vec<City>),
    Failed,
}

/// In-popup settings for world clocks and weather.
#[derive(Debug, Clone, Default)]
pub struct Edit {
    /// Which list the open search adds to; `None` while no search is open.
    pub target: Option<Target>,
    pub query: String,
    pub search: Search,
    /// Bumped on every keystroke; a search or reply tagged with an older
    /// value is stale and dropped.
    pub generation: u64,
}

/// What a picked search result becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Clock,
    Weather,
}

/// Id of the settings search field, focused when a search opens.
pub static SEARCH_ID: std::sync::LazyLock<cosmic::widget::Id> = std::sync::LazyLock::new(|| cosmic::widget::Id::new("city-search"));

/// Pause after the last keystroke before searching.
pub const SEARCH_DEBOUNCE: Duration = Duration::from_millis(300);
/// Shortest query worth sending.
pub const MIN_QUERY: usize = 2;

pub fn searchable(q: &str) -> bool {
    q.trim().chars().count() >= MIN_QUERY
}

/// Symbolic icon for a WMO code (freedesktop names; COSMIC/Adwaita ship them).
pub fn condition_icon(code: u8, is_day: bool) -> &'static str {
    match (code, is_day) {
        (0 | 1, true) => "weather-clear-symbolic",
        (0 | 1, false) => "weather-clear-night-symbolic",
        (2, true) => "weather-few-clouds-symbolic",
        (2, false) => "weather-few-clouds-night-symbolic",
        (3, _) => "weather-overcast-symbolic",
        (45 | 48, _) => "weather-fog-symbolic",
        (51..=57 | 80..=82, _) => "weather-showers-scattered-symbolic",
        (61..=67, _) => "weather-showers-symbolic",
        (71..=77 | 85 | 86, _) => "weather-snow-symbolic",
        (95..=99, _) => "weather-storm-symbolic",
        _ => "weather-overcast-symbolic",
    }
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

fn small_button<'a>(label: String, msg: Option<Message>, bold: bool) -> Element<'a, Message> {
    let mut t = text(label).size(12);
    if bold {
        t = t.font(cosmic::font::bold());
    }
    button::custom(t).padding([3, 6]).on_press_maybe(msg).class(flat(if bold { CREAM } else { GREY })).into()
}

fn remove_button<'a>(msg: Message) -> Element<'a, Message> {
    button::custom(crate::sleeve::close_glyph(12))
        .padding(4)
        .on_press(msg)
        .class(flat(GREY))
        .into()
}

fn search_block<'a>(edit: &'a Edit, target: Target, clocks: &'a [WorldClock], places: &'a [WeatherPlace]) -> Element<'a, Message> {
    let placeholder = match target {
        Target::Clock => fl!("search-clock-city"),
        Target::Weather => fl!("search-weather-city"),
    };
    let mut col = column::with_capacity(6).spacing(4).width(Length::Fill).push(
        row![
            cosmic::widget::text_input::search_input(placeholder, edit.query.as_str())
                .id(SEARCH_ID.clone())
                .on_input(Message::CitySearchInput)
                .on_submit(|_| Message::CitySearch(None))
                .width(Length::Fill),
            small_button(fl!("cancel"), Some(Message::CloseCitySearch), false),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    );
    let note = |s: String| Element::from(text(s).size(12).class(color(GREY)));
    match &edit.search {
        Search::Idle => {}
        Search::Searching => col = col.push(note(fl!("searching"))),
        Search::Failed => col = col.push(note(fl!("search-failed"))),
        Search::Found(cities) if cities.is_empty() => col = col.push(note(fl!("no-cities"))),
        Search::Found(cities) => {
            for (i, c) in cities.iter().enumerate() {
                // A clock for a zone already shown would repeat that time.
                let twin = match target {
                    Target::Clock => c.timezone.as_ref().and_then(|z| clocks.iter().find(|k| &k.zone == z)),
                    Target::Weather => None,
                };
                let added = target == Target::Weather && places.iter().any(|p| p.key() == c.place().key());
                let usable = match target {
                    Target::Weather => !added,
                    Target::Clock => c.timezone.is_some() && twin.is_none(),
                };
                let second = match (twin, added) {
                    (Some(k), _) => fl!("same-time-as", city = k.label.as_str()),
                    (None, true) => fl!("already-added"),
                    (None, false) => c.detail.clone(),
                };
                col = col.push(
                    button::custom(
                        column![
                            text(c.name.as_str()).size(ROW).font(cosmic::font::bold()),
                            text(second).size(12).class(color(GREY)),
                        ]
                        .spacing(1)
                        .width(Length::Fill),
                    )
                    .padding([5, 8])
                    .width(Length::Fill)
                    .on_press_maybe(usable.then_some(Message::PickCity(i)))
                    .class(flat(CREAM)),
                );
            }
        }
    }
    col.into()
}

/// The credits in edit mode. Each list has its own action: world clocks
/// show their live time, weather cities their temperature, each with a
/// remove button, then "Add city", which opens a search right under that
/// list; a result is one click.
pub fn edit_view<'a>(
    clocks: &'a [WorldClock],
    places: &'a [WeatherPlace],
    readings: &'a std::collections::HashMap<String, Weather>,
    edit: &'a Edit,
    now: &jiff::Zoned,
) -> Element<'a, Message> {
    let mut col = column::with_capacity(12).spacing(6).width(Length::Fill);

    col = col.push(caption::<Message>(fl!("world-clock")));
    for (i, c) in clocks.iter().enumerate() {
        let time = zone_time(&c.zone, now).map(|(t, _)| t).unwrap_or_default();
        col = col.push(
            row![
                text(c.label.as_str()).size(ROW).font(cosmic::font::bold()).class(color(CREAM)).width(Length::Fill),
                figures(&time, ROW, false, GREY),
                remove_button(Message::RemoveClock(i)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    col = col.push(if edit.target == Some(Target::Clock) {
        search_block(edit, Target::Clock, clocks, places)
    } else {
        small_button(fl!("add-city"), Some(Message::OpenCitySearch(Target::Clock)), true)
    });

    col = col.push(hairline());
    col = col.push(caption::<Message>(fl!("weather")));
    for (i, p) in places.iter().enumerate() {
        let temp = readings.get(&p.key()).map(|w| format!("{:.0}°", w.temperature)).unwrap_or_default();
        col = col.push(
            row![
                text(p.name.as_str()).size(ROW).font(cosmic::font::bold()).class(color(CREAM)).width(Length::Fill),
                figures(&temp, ROW, false, GREY),
                remove_button(Message::RemoveWeather(i)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
    col = col.push(if edit.target == Some(Target::Weather) {
        search_block(edit, Target::Weather, clocks, places)
    } else if places.len() >= MAX_WEATHER_PLACES {
        Element::from(text(fl!("max-cities", max = MAX_WEATHER_PLACES)).size(12).class(color(GREY)))
    } else {
        small_button(fl!("add-city"), Some(Message::OpenCitySearch(Target::Weather)), true)
    });

    column![
        scrollable(col).height(Length::Fill),
        row![cosmic::widget::Space::new().width(Length::Fill), small_button(fl!("done"), Some(Message::EditExtras(false)), true)],
    ]
    .spacing(8)
    .height(Length::Fill)
    .into()
}

/// World clocks and weather, each under a tracked caption, with an Edit
/// button; an "Add" button alone when neither is configured.
pub fn view<'a>(
    clocks: &'a [WorldClock],
    places: &'a [WeatherPlace],
    readings: &'a std::collections::HashMap<String, Weather>,
    now: &jiff::Zoned,
) -> Element<'a, Message> {
    if clocks.is_empty() && places.is_empty() {
        return small_button(fl!("add-extras"), Some(Message::EditExtras(true)), false);
    }
    let mut col = column::with_capacity(12).spacing(8).width(Length::Fill);
    let rows: Vec<_> = clocks.iter().filter_map(|c| zone_time(&c.zone, now).map(|t| (c, t))).collect();
    if !rows.is_empty() {
        col = col.push(caption::<Message>(fl!("world-clock")));
        for (c, (time, off)) in rows {
            col = col.push(credit_row(c.label.clone(), figures(&time, ROW, true, CREAM), off));
        }
    }
    if !places.is_empty() {
        if col_has_children(clocks, now) {
            col = col.push(hairline());
        }
        col = col.push(caption::<Message>(fl!("weather")));
        for p in places {
            col = col.push(weather_row(p, readings.get(&p.key())));
        }
    }
    col = col.push(row![cosmic::widget::Space::new().width(Length::Fill), small_button(fl!("edit-extras"), Some(Message::EditExtras(true)), false)]);
    col.into()
}

/// One city: name over "condition · H · L" on the left, icon and
/// temperature on the right; a dash until a reading exists.
fn weather_row<'a>(p: &'a WeatherPlace, w: Option<&'a Weather>) -> Element<'a, Message> {
    let mut left = column![text(p.name.as_str()).size(ROW).font(cosmic::font::bold()).class(color(CREAM))].spacing(1).width(Length::Fill);
    let right: Element<'a, Message> = match w {
        Some(w) => {
            left = left.push(
                text(fl!(
                    "weather-summary",
                    condition = condition(w.code),
                    high = format!("{:.0}", w.high),
                    low = format!("{:.0}", w.low)
                ))
                .size(12)
                .class(color(GREY)),
            );
            row![
                cosmic::widget::icon::from_name(condition_icon(w.code, w.is_day)).size(20).symbolic(true).icon().class(
                    cosmic::theme::Svg::Custom(std::rc::Rc::new(|t| cosmic::iced::widget::svg::Style {
                        color: Some(crate::sleeve::rgb(t, CREAM))
                    }))
                ),
                figures(&format!("{:.0}°", w.temperature), 20, true, CREAM),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        }
        // Not fetched yet, or every fetch so far failed: an honest dash.
        None => figures("—", 20, true, GREY),
    };
    row![left, right].spacing(10).align_y(Alignment::Center).into()
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
        assert_eq!(parse(j), Some(Weather { temperature: 31.4, code: 95, high: 33.1, low: 24.6, is_day: true }));
        let n = r#"{"current":{"temperature_2m":25.0,"weather_code":0,"is_day":0},"daily":{"temperature_2m_max":[30.0],"temperature_2m_min":[24.0]}}"#;
        assert!(!parse(n).unwrap().is_day);
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
    fn parses_geocoding_and_skips_repeated_names() {
        let j = r#"{"results":[{"name":"Tokyo","latitude":35.69,"longitude":139.69,"timezone":"Asia/Tokyo","admin1":"Tokyo","country":"Japan"}]}"#;
        let c = parse_cities(j).unwrap();
        assert_eq!(c[0].detail, "Japan");
        assert_eq!(c[0].timezone.as_deref(), Some("Asia/Tokyo"));
        assert_eq!(parse_cities(r#"{"generationtime_ms":0.1}"#), Some(vec![]));
        assert_eq!(parse_cities("nope"), None);
    }

    #[test]
    fn every_code_has_an_icon_with_night_variants() {
        assert_eq!(condition_icon(0, false), "weather-clear-night-symbolic");
        assert_eq!(condition_icon(95, true), "weather-storm-symbolic");
        assert!((0..=99u8).all(|c| condition_icon(c, true).ends_with("-symbolic")));
    }

    #[test]
    fn short_queries_are_not_searched() {
        assert!(!searchable(""));
        assert!(!searchable(" k "));
        assert!(searchable("Ko"));
    }

    #[test]
    fn encodes_query() {
        assert_eq!(encode("Kuala Lumpur"), "Kuala%20Lumpur");
        assert_eq!(encode("São Paulo"), "S%C3%A3o%20Paulo");
        assert!(geocode_url(" Tokyo ").contains("name=Tokyo&"));
    }

    #[test]
    fn thunderstorm_code_has_label() {
        crate::localize::localize();
        assert!(!condition(95).is_empty());
        assert!(condition(200).is_empty());
    }
}

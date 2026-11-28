// SPDX-License-Identifier: GPL-3.0-only

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};

pub const APP_ID: &str = "com.system76.CosmicAppletNotifications";
pub const TIME_CONFIG_ID: &str = "com.system76.CosmicAppletTime";

/// Mirror of the stock time applet's config (`com.system76.CosmicAppletTime` v1).
// Adapted from pop-os/cosmic-applets cosmic-applets-config/src/time.rs @ 328d142 (GPL-3.0-only)
#[derive(Debug, Clone, CosmicConfigEntry, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[version = 1]
pub struct TimeAppletConfig {
    pub military_time: bool,
    pub show_seconds: bool,
    /// 0 = Monday ... 5 = Saturday, anything else = Sunday.
    pub first_day_of_week: u8,
    pub show_date_in_top_panel: bool,
    pub show_weekday: bool,
    #[serde(default, skip_serializing_if = "str::is_empty")]
    pub format_strftime: String,
}

impl Default for TimeAppletConfig {
    fn default() -> Self {
        Self {
            military_time: false,
            show_seconds: false,
            first_day_of_week: 6,
            show_date_in_top_panel: true,
            show_weekday: false,
            format_strftime: String::new(),
        }
    }
}

pub const CLOCK_CENTER_CONFIG_ID: &str = "io.github.agustine_leo.CosmicExtAppletClockCenter";

/// A world clock row: display label and IANA zone (`Asia/Tokyo`).
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct WorldClock {
    pub label: String,
    pub zone: String,
}

/// Where to fetch weather for (sent to Open-Meteo as latitude/longitude).
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct WeatherPlace {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl WeatherPlace {
    /// Identity for readings and duplicate checks (name + rounded coordinates).
    pub fn key(&self) -> String {
        format!("{}@{:.2},{:.2}", self.name, self.latitude, self.longitude)
    }
}

/// Most weather cities shown under the calendar.
pub const MAX_WEATHER_PLACES: usize = 4;

/// Clock Center's own settings (`io.github.agustine_leo.CosmicExtAppletClockCenter` v1).
/// All default to empty: nothing extra is shown and nothing is fetched.
#[derive(Debug, Clone, Default, CosmicConfigEntry, PartialEq, serde::Deserialize, serde::Serialize)]
#[version = 1]
pub struct ClockCenterConfig {
    pub world_clocks: Vec<WorldClock>,
    pub weather_places: Vec<WeatherPlace>,
    /// Single weather place from before `weather_places`; read only for
    /// migration and cleared on the next weather edit.
    pub weather: Option<WeatherPlace>,
}

impl ClockCenterConfig {
    /// Weather cities to show: the list, or the legacy single place.
    pub fn places(&self) -> Vec<WeatherPlace> {
        if self.weather_places.is_empty() {
            self.weather.clone().into_iter().collect()
        } else {
            self.weather_places.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn kl() -> WeatherPlace {
        WeatherPlace { name: "Kuala Lumpur".into(), latitude: 3.14, longitude: 101.69 }
    }
    #[test]
    fn legacy_single_place_is_migrated() {
        let c = ClockCenterConfig { weather: Some(kl()), ..Default::default() };
        assert_eq!(c.places(), vec![kl()]);
        let c = ClockCenterConfig { weather: Some(kl()), weather_places: vec![], ..Default::default() };
        assert_eq!(c.places().len(), 1);
    }
    #[test]
    fn list_wins_over_legacy() {
        let tokyo = WeatherPlace { name: "Tokyo".into(), latitude: 35.69, longitude: 139.69 };
        let c = ClockCenterConfig { weather: Some(kl()), weather_places: vec![tokyo.clone()], ..Default::default() };
        assert_eq!(c.places(), vec![tokyo]);
    }
}

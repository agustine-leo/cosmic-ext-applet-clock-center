// SPDX-License-Identifier: GPL-3.0-only

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};

pub const APP_ID: &str = "com.system76.CosmicAppletNotifications";
pub const OWN_CONFIG_ID: &str = "io.github.agustine_leo.CosmicExtAppletClockCenter";
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

/// This applet's own settings.
#[derive(Debug, Clone, CosmicConfigEntry, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[version = 1]
pub struct ClockCenterConfig {
    pub timer_presets: Vec<u32>,
}

impl Default for ClockCenterConfig {
    fn default() -> Self {
        Self {
            timer_presets: vec![1, 5, 10, 25],
        }
    }
}

impl ClockCenterConfig {
    /// Drop zero and duplicate presets; fall back to the defaults if nothing is left.
    pub fn sanitized(mut self) -> Self {
        let mut seen = std::collections::HashSet::new();
        self.timer_presets.retain(|&m| m > 0 && seen.insert(m));
        if self.timer_presets.is_empty() {
            self.timer_presets = Self::default().timer_presets;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitized_drops_zero_and_duplicates() {
        let c = ClockCenterConfig { timer_presets: vec![0, 5, 5, 10] }.sanitized();
        assert_eq!(c.timer_presets, vec![5, 10]);
    }
    #[test]
    fn sanitized_empty_uses_default() {
        let c = ClockCenterConfig { timer_presets: vec![0] }.sanitized();
        assert_eq!(c.timer_presets, vec![1, 5, 10, 25]);
    }
}

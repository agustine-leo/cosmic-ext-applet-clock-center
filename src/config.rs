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

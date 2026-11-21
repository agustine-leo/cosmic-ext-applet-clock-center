// SPDX-License-Identifier: GPL-3.0-only

use crate::config::TimeAppletConfig;

/// Text for the panel button, following the stock time applet's settings.
pub fn format_label(now: &jiff::Zoned, cfg: &TimeAppletConfig) -> String {
    if !cfg.format_strftime.is_empty() {
        if let Ok(s) = jiff::fmt::strtime::format(&cfg.format_strftime, now) {
            return s;
        }
    }
    let time = match (cfg.military_time, cfg.show_seconds) {
        (true, true) => "%H:%M:%S",
        (true, false) => "%H:%M",
        (false, true) => "%-I:%M:%S %p",
        (false, false) => "%-I:%M %p",
    };
    let fmt = if cfg.show_date_in_top_panel {
        let date = if cfg.show_weekday { "%a %b %-d" } else { "%b %-d" };
        format!("{date}  {time}")
    } else {
        time.to_string()
    };
    jiff::fmt::strtime::format(&fmt, now).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t() -> jiff::Zoned {
        "2026-11-21T14:05:09[Asia/Singapore]".parse().unwrap()
    }

    #[test]
    fn twelve_hour_with_date() {
        assert_eq!(format_label(&t(), &TimeAppletConfig::default()), "Nov 21  2:05 PM");
    }

    #[test]
    fn military_with_seconds_and_weekday() {
        let cfg = TimeAppletConfig { military_time: true, show_seconds: true, show_weekday: true, ..Default::default() };
        assert_eq!(format_label(&t(), &cfg), "Sat Nov 21  14:05:09");
    }

    #[test]
    fn strftime_overrides() {
        let cfg = TimeAppletConfig { format_strftime: "%H:%M".into(), ..Default::default() };
        assert_eq!(format_label(&t(), &cfg), "14:05");
    }

    #[test]
    fn time_only_when_date_hidden() {
        let cfg = TimeAppletConfig { show_date_in_top_panel: false, ..Default::default() };
        assert_eq!(format_label(&t(), &cfg), "2:05 PM");
    }
}

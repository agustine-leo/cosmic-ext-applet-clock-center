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

/// Longest single sleep of the minute tick. `tokio::time::sleep` is monotonic and
/// pauses during suspend, so short sleeps bound how stale the label is after resume.
const MAX_TICK_SLEEP: std::time::Duration = std::time::Duration::from_secs(5);

/// How long the minute tick sleeps: until just past the next minute boundary, at most 5s.
pub fn tick_delay(now: &jiff::Zoned) -> std::time::Duration {
    let into_minute = now.second() as u64 * 1000 + now.millisecond() as u64;
    let until_next = std::time::Duration::from_millis(60_000 - into_minute + 20);
    until_next.min(MAX_TICK_SLEEP)
}

/// Wall-clock minute (Unix minutes); a Tick is emitted whenever it changes.
pub fn minute_key(now: &jiff::Zoned) -> i64 {
    now.timestamp().as_second().div_euclid(60)
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
    fn tick_delay_capped_at_five_seconds() {
        assert_eq!(tick_delay(&t()), std::time::Duration::from_secs(5));
    }

    #[test]
    fn tick_delay_aligns_to_minute_when_close() {
        let z: jiff::Zoned = "2026-11-21T14:05:57.5[Asia/Singapore]".parse().unwrap();
        assert_eq!(tick_delay(&z), std::time::Duration::from_millis(2_520));
    }

    #[test]
    fn minute_key_changes_only_on_minute() {
        let a: jiff::Zoned = "2026-11-21T14:05:00[Asia/Singapore]".parse().unwrap();
        let b: jiff::Zoned = "2026-11-21T14:05:59.9[Asia/Singapore]".parse().unwrap();
        let c: jiff::Zoned = "2026-11-21T14:06:00[Asia/Singapore]".parse().unwrap();
        assert_eq!(minute_key(&a), minute_key(&b));
        assert_ne!(minute_key(&b), minute_key(&c));
        // After a suspend the key differs even though only one wake happened.
        let d: jiff::Zoned = "2026-11-21T16:30:02[Asia/Singapore]".parse().unwrap();
        assert_ne!(minute_key(&a), minute_key(&d));
    }

    #[test]
    fn time_only_when_date_hidden() {
        let cfg = TimeAppletConfig { show_date_in_top_panel: false, ..Default::default() };
        assert_eq!(format_label(&t(), &cfg), "2:05 PM");
    }
}

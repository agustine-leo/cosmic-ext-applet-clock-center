// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use cosmic::{
    widget::{button, column, row, text},
    Element,
};
use zbus::zvariant::Value;

use crate::app::Message;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timer {
    minutes: u32,
    ends_at: Instant,
}

impl Timer {
    pub fn start(minutes: u32, now: Instant) -> Self {
        Self { minutes, ends_at: now + Duration::from_secs(minutes as u64 * 60) }
    }
    pub fn remaining(&self, now: Instant) -> Duration {
        self.ends_at.saturating_duration_since(now)
    }
    pub fn is_done(&self, now: Instant) -> bool {
        now >= self.ends_at
    }
}

/// At most one running timer.
#[derive(Debug, Default)]
pub struct TimerState {
    pub current: Option<Timer>,
}

impl TimerState {
    pub fn start(&mut self, minutes: u32, now: Instant) {
        self.current = Some(Timer::start(minutes, now));
    }
    pub fn cancel(&mut self) {
        self.current = None;
    }
    /// Returns `Some(minutes)` exactly once when the timer finishes, then clears it.
    pub fn tick(&mut self, now: Instant) -> Option<u32> {
        let t = self.current?;
        if t.is_done(now) {
            self.current = None;
            Some(t.minutes)
        } else {
            None
        }
    }
}

pub fn format_remaining(d: Duration) -> String {
    let s = d.as_secs();
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// Critical urgency (byte 2) so the banner is not swallowed by Do Not Disturb.
pub async fn send_done_notification(minutes: u32) -> zbus::Result<()> {
    let conn = zbus::Connection::session().await?;
    let mut hints: HashMap<&str, Value<'_>> = HashMap::new();
    hints.insert("urgency", Value::U8(2));
    conn.call_method(
        Some("org.freedesktop.Notifications"),
        "/org/freedesktop/Notifications",
        Some("org.freedesktop.Notifications"),
        "Notify",
        &(
            "Clock Center",
            0u32,
            "alarm-symbolic",
            "Timer done",
            format!("{minutes} min"),
            Vec::<&str>::new(),
            hints,
            0i32,
        ),
    )
    .await?;
    Ok(())
}

pub fn view<'a>(presets: &'a [u32], state: &'a TimerState, now: Instant) -> Element<'a, Message> {
    let mut col = column![].spacing(8).push(text::heading("⏰ Start a timer"));
    match &state.current {
        Some(t) => {
            col = col.push(
                row![]
                    .spacing(8)
                    .align_y(cosmic::iced::Alignment::Center)
                    .push(text::title3(format_remaining(t.remaining(now))))
                    .push(button::standard("✕").on_press(Message::CancelTimer)),
            );
        }
        None => {
            let mut r = row![].spacing(8);
            for &m in presets {
                r = r.push(button::standard(format!("{m} min")).on_press(Message::StartTimer(m)));
            }
            col = col.push(r);
        }
    }
    col.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remaining_counts_down() {
        let t0 = Instant::now();
        let t = Timer::start(5, t0);
        assert_eq!(t.remaining(t0 + Duration::from_secs(28)), Duration::from_secs(272));
    }
    #[test]
    fn done_at_end() {
        let t0 = Instant::now();
        assert!(Timer::start(1, t0).is_done(t0 + Duration::from_secs(60)));
    }
    #[test]
    fn format_mm_ss() {
        assert_eq!(format_remaining(Duration::from_secs(272)), "04:32");
    }
    #[test]
    fn starting_new_timer_replaces_old() {
        let t0 = Instant::now();
        let mut s = TimerState::default();
        s.start(1, t0);
        s.start(5, t0);
        assert_eq!(s.current, Some(Timer::start(5, t0)));
        assert_eq!(s.tick(t0 + Duration::from_secs(61)), None);
        assert_eq!(s.tick(t0 + Duration::from_secs(300)), Some(5));
    }
    #[test]
    fn cancelled_timer_never_fires() {
        let t0 = Instant::now();
        let mut s = TimerState::default();
        s.start(1, t0);
        s.cancel();
        assert_eq!(s.tick(t0 + Duration::from_secs(61)), None);
    }
    #[test]
    fn fires_exactly_once() {
        let t0 = Instant::now();
        let mut s = TimerState::default();
        s.start(1, t0);
        assert_eq!(s.tick(t0 + Duration::from_secs(60)), Some(1));
        assert_eq!(s.tick(t0 + Duration::from_secs(61)), None);
        assert!(s.current.is_none());
    }
    #[test]
    #[ignore = "sends one real notification"]
    fn live_notification() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(send_done_notification(1)).unwrap();
    }
}

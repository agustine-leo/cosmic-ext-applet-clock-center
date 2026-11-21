// SPDX-License-Identifier: GPL-3.0-only

use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
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
    ends_at: SystemTime,
}

impl Timer {
    pub fn start(minutes: u32, now: SystemTime) -> Self {
        Self { minutes, ends_at: now + Duration::from_secs(minutes as u64 * 60) }
    }
    pub fn remaining(&self, now: SystemTime) -> Duration {
        self.ends_at.duration_since(now).unwrap_or_default()
    }
    pub fn is_done(&self, now: SystemTime) -> bool {
        now >= self.ends_at
    }
}

/// Deadlines use wall-clock time so suspended time counts (monotonic clocks pause in suspend).
/// At most one running timer.
#[derive(Debug, Default)]
pub struct TimerState {
    pub current: Option<Timer>,
}

impl TimerState {
    pub fn start(&mut self, minutes: u32, now: SystemTime) {
        self.current = Some(Timer::start(minutes, now));
    }
    pub fn cancel(&mut self) {
        self.current = None;
    }
    /// Returns `Some(minutes)` exactly once when the timer finishes, then clears it.
    pub fn tick(&mut self, now: SystemTime) -> Option<u32> {
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

/// Critical urgency (byte 2). cosmic-notifications still hides banners under Do Not
/// Disturb, so the caller also plays a sound then (see `should_play_sound`).
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

/// A finished timer only needs a sound when Do Not Disturb hides its banner.
pub fn should_play_sound(do_not_disturb: bool) -> bool {
    do_not_disturb
}

/// Play the freedesktop "complete" sound; errors are only logged.
pub async fn play_done_sound() {
    let result = tokio::process::Command::new("canberra-gtk-play")
        .args(["-i", "complete", "-d", "Clock Center timer"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await;
    match result {
        Ok(status) if !status.success() => tracing::warn!(%status, "canberra-gtk-play failed"),
        Ok(_) => {}
        Err(err) => tracing::warn!(?err, "Could not run canberra-gtk-play"),
    }
}

pub fn view<'a>(presets: &'a [u32], state: &'a TimerState, now: SystemTime) -> Element<'a, Message> {
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
        let t0 = SystemTime::now();
        let t = Timer::start(5, t0);
        assert_eq!(t.remaining(t0 + Duration::from_secs(28)), Duration::from_secs(272));
    }
    #[test]
    fn done_at_end() {
        let t0 = SystemTime::now();
        assert!(Timer::start(1, t0).is_done(t0 + Duration::from_secs(60)));
    }
    #[test]
    fn format_mm_ss() {
        assert_eq!(format_remaining(Duration::from_secs(272)), "04:32");
    }
    #[test]
    fn starting_new_timer_replaces_old() {
        let t0 = SystemTime::now();
        let mut s = TimerState::default();
        s.start(1, t0);
        s.start(5, t0);
        assert_eq!(s.current, Some(Timer::start(5, t0)));
        assert_eq!(s.tick(t0 + Duration::from_secs(61)), None);
        assert_eq!(s.tick(t0 + Duration::from_secs(300)), Some(5));
    }
    #[test]
    fn fires_immediately_after_clock_jump() {
        let t0 = SystemTime::now();
        let mut s = TimerState::default();
        s.start(5, t0);
        assert_eq!(s.tick(t0 + Duration::from_secs(6 * 60)), Some(5));
    }
    #[test]
    fn cancelled_timer_never_fires() {
        let t0 = SystemTime::now();
        let mut s = TimerState::default();
        s.start(1, t0);
        s.cancel();
        assert_eq!(s.tick(t0 + Duration::from_secs(61)), None);
    }
    #[test]
    fn fires_exactly_once() {
        let t0 = SystemTime::now();
        let mut s = TimerState::default();
        s.start(1, t0);
        assert_eq!(s.tick(t0 + Duration::from_secs(60)), Some(1));
        assert_eq!(s.tick(t0 + Duration::from_secs(61)), None);
        assert!(s.current.is_none());
    }
    #[test]
    fn sound_only_under_dnd() {
        assert!(should_play_sound(true));
        assert!(!should_play_sound(false));
    }
    #[test]
    #[ignore = "sends one real notification"]
    fn live_notification() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(send_done_notification(1)).unwrap();
    }
}

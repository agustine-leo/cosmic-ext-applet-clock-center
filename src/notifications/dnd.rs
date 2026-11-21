// SPDX-License-Identifier: GPL-3.0-only

//! Do Not Disturb state, stored in cosmic-notifications' own config.

use cosmic::cosmic_config::{Config, CosmicConfigEntry};
use cosmic_notifications_config::NotificationsConfig;

fn helper() -> Option<Config> {
    Config::new(cosmic_notifications_config::ID, NotificationsConfig::VERSION).ok()
}

/// Current value (false when the config is unreadable).
pub fn read() -> bool {
    helper().is_some_and(|h| read_from(&h))
}

/// Persist the value; cosmic-notifications watches the file and reacts.
pub fn write(value: bool) {
    if let Some(h) = helper() {
        write_to(&h, value);
    }
}

/// While the popup is open, banners are suppressed by switching Do Not Disturb
/// on, so new notifications land in the list instead of popping over it
/// (GNOME behaviour). Only a value this applet forced is undone on close.
#[derive(Debug, Default)]
pub struct PopupDnd {
    forced: bool,
}

impl PopupDnd {
    /// Popup opened; returns the value to write, if any.
    pub fn on_open(&mut self, user_dnd: bool) -> Option<bool> {
        self.forced = !user_dnd;
        self.forced.then_some(true)
    }

    /// Popup closed; returns the value to write, if any.
    pub fn on_close(&mut self) -> Option<bool> {
        std::mem::take(&mut self.forced).then_some(false)
    }

    /// The user toggled Do Not Disturb themselves: their choice wins.
    pub fn on_user_set(&mut self) {
        self.forced = false;
    }

    pub fn forced(&self) -> bool {
        self.forced
    }
}

/// Marker recording that Do Not Disturb is currently forced by an open popup,
/// so a crash can be repaired at the next start.
pub fn marker_path() -> Option<std::path::PathBuf> {
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".local/state")))?;
    Some(state.join("cosmic-ext-applet-clock-center").join("dnd-forced"))
}

pub fn set_marker(path: &std::path::Path, present: bool) {
    let result = if present {
        path.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(path, b""))
    } else {
        match std::fs::remove_file(path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            r => r,
        }
    };
    if let Err(err) = result {
        tracing::warn!(?err, ?path, "Failed to update dnd-forced marker");
    }
}

/// True (and removes it) if a marker was left behind.
pub fn take_marker(path: &std::path::Path) -> bool {
    std::fs::remove_file(path).is_ok()
}

fn read_from(h: &Config) -> bool {
    match NotificationsConfig::get_entry(h) {
        Ok(c) => c.do_not_disturb,
        Err((errs, c)) => {
            tracing::warn!(?errs, "Failed to read notifications config");
            c.do_not_disturb
        }
    }
}

fn write_to(h: &Config, value: bool) {
    let mut cfg = NotificationsConfig::get_entry(h).unwrap_or_else(|(_, c)| c);
    cfg.do_not_disturb = value;
    if let Err(err) = cfg.write_entry(h) {
        tracing::error!(?err, "Failed to write do_not_disturb");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_forces_dnd_only_when_it_is_off() {
        let mut p = PopupDnd::default();
        assert_eq!(p.on_open(false), Some(true));
        assert!(p.forced());
        let mut q = PopupDnd::default();
        assert_eq!(q.on_open(true), None);
        assert!(!q.forced());
    }

    #[test]
    fn closing_restores_only_what_was_forced() {
        let mut p = PopupDnd::default();
        p.on_open(false);
        assert_eq!(p.on_close(), Some(false));
        assert_eq!(p.on_close(), None);
        let mut q = PopupDnd::default();
        q.on_open(true);
        assert_eq!(q.on_close(), None);
    }

    #[test]
    fn user_toggle_while_open_cancels_forcing() {
        let mut p = PopupDnd::default();
        p.on_open(false);
        p.on_user_set();
        assert_eq!(p.on_close(), None);
    }

    #[test]
    fn recover_clears_a_leftover_marker_once() {
        let dir = std::env::temp_dir().join(format!("dnd-marker-{}", std::process::id()));
        let marker = dir.join("dnd-forced");
        set_marker(&marker, true);
        assert!(take_marker(&marker));
        assert!(!take_marker(&marker));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn write_then_read_roundtrip() {
        let dir = std::env::temp_dir().join(format!("dnd-test-{}", std::process::id()));
        let h = Config::with_custom_path(
            cosmic_notifications_config::ID,
            NotificationsConfig::VERSION,
            dir.clone(),
        )
        .unwrap();
        write_to(&h, true);
        assert!(read_from(&h));
        let f = dir.join("cosmic").join(cosmic_notifications_config::ID).join("v1/do_not_disturb");
        assert_eq!(std::fs::read_to_string(f).unwrap().trim(), "true");
        write_to(&h, false);
        assert!(!read_from(&h));
        let _ = std::fs::remove_dir_all(dir);
    }
}

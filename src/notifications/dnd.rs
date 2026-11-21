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

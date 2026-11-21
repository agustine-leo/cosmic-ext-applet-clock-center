// SPDX-License-Identifier: GPL-3.0-only

//! Do Not Disturb state, stored in cosmic-notifications' own config.

use cosmic::cosmic_config::{Config, CosmicConfigEntry};
use cosmic_notifications_config::NotificationsConfig;

fn helper() -> Option<Config> {
    Config::new(cosmic_notifications_config::ID, NotificationsConfig::VERSION).ok()
}

/// Current value (false when the config is unreadable).
pub fn read() -> bool {
    helper()
        .and_then(|h| NotificationsConfig::get_entry(&h).ok())
        .is_some_and(|c| c.do_not_disturb)
}

/// Persist the value; cosmic-notifications watches the file and reacts.
pub fn write(value: bool) {
    let Some(h) = helper() else { return };
    let mut cfg = NotificationsConfig::get_entry(&h).unwrap_or_else(|(_, c)| c);
    cfg.do_not_disturb = value;
    if let Err(err) = cfg.write_entry(&h) {
        tracing::error!(?err, "Failed to write do_not_disturb");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn write_then_read_roundtrip() {
        let dir = std::env::temp_dir().join(format!("dnd-test-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &dir);
        super::write(true);
        assert!(super::read());
        let f = dir.join("cosmic/com.system76.CosmicNotifications/v1/do_not_disturb");
        assert_eq!(std::fs::read_to_string(f).unwrap().trim(), "true");
        super::write(false);
        assert!(!super::read());
        let _ = std::fs::remove_dir_all(dir);
    }
}

// SPDX-License-Identifier: GPL-3.0-only

pub mod channel;
mod view;

pub use channel::ChannelStatus;
pub use view::view;

use cosmic_notifications_util::Notification;
use std::collections::HashSet;

/// Notifications received this session, oldest first, plus which are unread.
#[derive(Debug, Default)]
pub struct NotificationStore {
    items: Vec<Notification>,
    unread: HashSet<u32>,
}

impl NotificationStore {
    /// Add a notification; one with an existing id replaces it and moves to the top.
    pub fn push(&mut self, n: Notification) {
        self.items.retain(|old| old.id != n.id);
        self.unread.insert(n.id);
        self.items.push(n);
    }

    pub fn dismiss(&mut self, id: u32) {
        self.items.retain(|n| n.id != id);
        self.unread.remove(&id);
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.unread.clear();
    }

    pub fn mark_all_read(&mut self) {
        self.unread.clear();
    }

    pub fn has_unread(&self) -> bool {
        !self.unread.is_empty()
    }

    pub fn get(&self, id: u32) -> Option<&Notification> {
        self.items.iter().find(|n| n.id == id)
    }

    pub fn ids(&self) -> Vec<u32> {
        self.items.iter().map(|n| n.id).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn iter_newest_first(&self) -> impl Iterator<Item = &Notification> {
        self.items.iter().rev()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn store() -> NotificationStore {
        NotificationStore::default()
    }

    fn n(id: u32) -> Notification {
        Notification {
            id,
            app_name: "test".into(),
            app_icon: String::new(),
            summary: format!("summary {id}"),
            body: String::new(),
            actions: Vec::new(),
            hints: Vec::new(),
            expire_timeout: -1,
            time: SystemTime::now(),
        }
    }

    #[test] fn push_sets_unread() { let mut s = store(); s.push(n(1)); assert!(s.has_unread()); }
    #[test] fn mark_read_clears_dot() { let mut s = store(); s.push(n(1)); s.mark_all_read(); assert!(!s.has_unread()); }
    #[test] fn replace_keeps_single_entry() { let mut s = store(); s.push(n(7)); s.push(n(7)); assert_eq!(s.iter_newest_first().count(), 1); }
    #[test] fn dismiss_last_unread_clears_dot() { let mut s = store(); s.push(n(1)); s.dismiss(1); assert!(!s.has_unread()); }
    #[test] fn newest_first() { let mut s = store(); s.push(n(1)); s.push(n(2)); assert_eq!(s.iter_newest_first().next().unwrap().id, 2); }
}

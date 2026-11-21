// SPDX-License-Identifier: GPL-3.0-only

pub mod channel;
mod view;

pub use channel::ChannelStatus;
pub use view::{view, Expansion};

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

    /// Notifications grouped by app name; groups ordered by their newest
    /// notification, newest first, and newest first within each group.
    pub fn groups_newest_first(&self) -> Vec<(String, Vec<&Notification>)> {
        let mut groups: Vec<(String, Vec<&Notification>)> = Vec::new();
        for n in self.iter_newest_first() {
            match groups.iter_mut().find(|(app, _)| *app == n.app_name) {
                Some((_, ns)) => ns.push(n),
                None => groups.push((n.app_name.clone(), vec![n])),
            }
        }
        groups
    }

    /// Ids of all notifications from this app.
    pub fn ids_for_app(&self, app_name: &str) -> Vec<u32> {
        self.items.iter().filter(|n| n.app_name == app_name).map(|n| n.id).collect()
    }

    pub fn contains(&self, id: u32) -> bool {
        self.items.iter().any(|n| n.id == id)
    }
}

/// Plain-text body: tags stripped, lines trimmed, blank runs collapsed.
pub fn normalize_body(body: &str) -> String {
    let text = decode_entities(&strip_tags(body));
    let mut out: Vec<&str> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() && out.last().is_none_or(|l| l.is_empty()) {
            continue;
        }
        out.push(line);
    }
    while out.last().is_some_and(|l| l.is_empty()) {
        out.pop();
    }
    out.join("\n")
}

/// Remove `<tag>`s; a `<` only starts a tag when followed by a letter or `/`
/// and a closing `>` exists, so plain comparisons like "x < 5" survive.
fn strip_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('<') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let starts_tag = after.starts_with(|c: char| c.is_ascii_alphabetic() || c == '/');
        match after.find('>').filter(|_| starts_tag) {
            Some(end) => rest = &after[end + 1..],
            None => {
                out.push('<');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn decode_entities(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

/// First non-empty line of a body, for the collapsed preview.
pub fn first_line_preview(body: &str) -> String {
    normalize_body(body).lines().next().unwrap_or_default().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn store() -> NotificationStore {
        NotificationStore::default()
    }

    fn n(id: u32) -> Notification {
        na(id, "test")
    }

    fn na(id: u32, app: &str) -> Notification {
        Notification {
            id,
            app_name: app.into(),
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

    #[test] fn groups_by_app_newest_first() {
        let mut s = store();
        s.push(na(1, "a")); s.push(na(2, "b")); s.push(na(3, "a"));
        let g = s.groups_newest_first();
        let ids: Vec<(String, Vec<u32>)> = g.into_iter().map(|(app, ns)| (app, ns.iter().map(|n| n.id).collect())).collect();
        assert_eq!(ids, vec![("a".to_string(), vec![3, 1]), ("b".to_string(), vec![2])]);
    }
    #[test] fn body_lines_are_trimmed_and_blank_runs_collapsed() {
        assert_eq!(normalize_body("\n   FIRING · 1\n\n\n   Kube…\n  vault\n"), "FIRING · 1\n\nKube…\nvault");
    }
    #[test] fn body_markup_is_stripped() {
        assert_eq!(normalize_body("<b>Hi</b> &amp; <i>bye</i>"), "Hi & bye");
    }
    #[test] fn first_line_preview_returns_first_non_empty_line() {
        assert_eq!(first_line_preview("\n  \n  hello \nworld"), "hello");
    }
    #[test] fn strip_tags_keeps_plain_angle_brackets() {
        assert_eq!(normalize_body("x < 5 and y > 3"), "x < 5 and y > 3");
        assert_eq!(normalize_body("<b>bold</b> a<3"), "bold a<3");
    }
}

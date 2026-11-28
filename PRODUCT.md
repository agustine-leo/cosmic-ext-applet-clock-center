# Product

<!-- impeccable:product-schema 1 -->

## Platform

linux-desktop (COSMIC 1.11 panel applet on Fedora; native Rust, libcosmic/iced popup — not web, not mobile)

## Users

One person on their own ThinkPad (Fedora + COSMIC, dark desktop), mid-work. They click the clock in the top panel to catch up and get back to work within seconds.

## Product Purpose

One date/time button in the panel whose popup gathers notification history, a calendar, media controls and Do Not Disturb (a GNOME-date-menu-like hub). Success: every job in the popup is done at a glance, and the popup has an identity of its own instead of reading as a stock COSMIC/GNOME clone.

## Positioning

The only COSMIC applet combining clock, notification history, calendar and MPRIS media in one popup. It installs under the stock notifications applet's desktop id and receives notifications from the daemon over the panel socket.

## Operating Context

- Jobs, all used: notification history (catch up, act, dismiss, clear per app or all), calendar glance (date, weekday, month), media control (current MPRIS player: play/pause, previous, next; title, artist, player name, album art), Do Not Disturb toggle.
- Notification traffic: chat (WhatsApp via Whatsie, sender pictures), browser/web apps (Zen), system (battery, disk, downloads, Tailscale).
- Sits directly above the notification banners drawn by the sibling fork cosmic-ext-notifications, which use the "Nordic Timetable" world (see that repo's DESIGN.md). Clock Center deliberately gets its own world.
- Panel button shows time/date per COSMIC's time-applet settings (24-hour), an unread dot and animated equalizer bars while media plays.

## Capabilities and Constraints

- Popup is a fixed 761px wide, at least 792px tall (two columns: 416 left, 288 right, 1px divider); max 1000px tall.
- Notifications: grouped per app, newest first; expand long bodies, "+N more" per group, dismiss one or a group, click to activate, up to three app actions; urgency available (low/normal/critical).
- Media data available: player identity, title, artist, art URL, playback status. No track position or length today (not fetched).
- Calendar: month grid with first-day-of-week setting, previous/next month; no events (no calendar data source).
- Panel restarts are the only way to reload the applet mid-session; the popup opens only from a click.

## Brand Commitments

None binding. Current look rejected: notification list, media card, plain calendar and layout proportions all called out.

## Evidence on Hand

- README.md describes features.
- No screenshots yet; no events data, no track progress data. Do not invent either.

## Product Principles

1. Catch up in seconds: newest and most urgent first, every action one click.
2. Never break the pipeline: notifications, DnD and activation working beats any look.
3. Show only data the system really has.

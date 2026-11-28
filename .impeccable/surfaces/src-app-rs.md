---
version: 1
slug: "src-app-rs"
primary_target: "src/app.rs"
related_targets: []
---

## Scope

Clock Center popup (`src/app.rs` popup_content and the views it calls: media, notifications, calendar, world clocks, weather). Mode: Operate. Jobs: notification catch-up, calendar glance, media control, DND; plus world clocks (Tokyo, London) and weather (Kuala Lumpur) under the calendar, requested by the user.

## Direction contract

THESIS: The popup is the back of a record sleeve: now playing is the cover, notifications are set as the tracklist, the month and the extras are the liner-note credits. It refuses GNOME's date menu (rounded cards + accent pill) and the banners' timetable world.

OWN-WORLD: Matte sleeve black (#16130f) ground, cream type (#efe6d2), warm grey secondary (#9c9282), hairline rules (#2a251e), one vinyl-label orange (#e2542b) for attention only (unread dot, critical time, playing control). Cantarell; tracklist rows one size (14px), hierarchy by weight and the flush-right time column in fixed cells. Album art and sender pictures are the only images. No cards, no shadows.

STORY: The user sees what's playing and controls it, scans the tracklist newest first, acts/dismisses/clears, reads the date, the other zones' times and the weather, toggles DND.

FIRST VIEWPORT: Left 416px: cover row (140px art, bold track, artist · player, prev/play-pause/next), then per-app sides (tracked small-caps app name + count, rule) with rows: picture or index, bold title — grey body, time flush right; DND and Clear on the bottom catalogue line. Right 288px: 64px bold day figure, weekday, month-year; rule; month nav; grid with today in a cream cell; credits: world clocks rows and weather row.

FORM: LP Sleeve, assigned direction (position 5 of my ordered list). seed:2dc7384c38e54b4a. Signature move: notifications typeset as a tracklist with flush-right times, under the now-playing cover.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

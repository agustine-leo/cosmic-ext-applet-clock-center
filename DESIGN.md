---
name: cosmic-ext-applet-clock-center
description: Clock Center's popup set like the back of an LP sleeve: now playing as the cover, notifications as the tracklist, the month and extras as liner-note credits, in the active COSMIC theme's colours.
colors:
  # Theme roles, resolved from the active COSMIC theme at draw time (src/sleeve.rs).
  sleeve: "theme popup background (applet popup container)"
  print: "theme.on_bg_color"
  dim: "theme.on_bg_color @ 68%"
  rule: "theme.bg_divider"
  attention: "theme.accent_text_color"
  attention-fill: "theme.accent_color"
  on-attention: "theme.on_accent_color"
  critical: "theme.destructive_text_color"
  hover-fill: "theme.background.component.hover"
  pressed-fill: "theme.background.component.pressed"
typography:
  face: "Cantarell (desktop UI face)"
  day-figure: { size: "64px", weight: 700 }
  track-title: { size: "20px", weight: 700 }
  row: { size: "14px", weight: "700 summary / 400 body" }
  credits-figure: { size: "14px / 20px (temperature)", weight: 700 }
  caption: { size: "11px", weight: 700, case: "uppercase" }
rounded:
  button: "4px"
  today-cell: "2px"
spacing:
  popup-padding: "16px"
  column-gap: "12px"
  left-sections: "20px"
  row-vertical: "10px"
  right-sections: "16px"
components:
  cover: "120px album art, track title, artist · player, prev / play-pause / next"
  side: "uppercase app caption + count + clear, hairline, rows, '+N more'"
  track-row: "24px picture (sender picture or app icon), bold summary over dim body, unread dot, time flush right, chevron, dismiss"
  calendar: "64px day figure + weekday + month, hairline, month nav, 30px day cells, today in an accent cell"
  credits: "WORLD CLOCK rows (label, offset, time), WEATHER row (place, condition, temperature, H/L)"
  catalogue-line: "hairline, Do Not Disturb toggler (native), Clear (flat text)"
---

# Design System: cosmic-ext-applet-clock-center

## Overview

The popup reads as the back of a record sleeve. The cover (now playing) sits at the top left, the notifications are typeset as a tracklist grouped into "sides" per app, and the right column holds the credits: today's date at large scale, the month grid, then world clocks and weather. Hierarchy comes from size and weight, not boxes: there are no cards, no shadows and no tinted panels, only hairlines from the theme's divider colour.

All colour comes from the active COSMIC theme (user decision, 2026-11-28: "same color with our current themes"), so the popup follows dark/light mode and the accent colour. The sleeve structure carries the identity.

- Album art and sender pictures are the only images.
- Times are set in fixed cells so they line up flush right like track lengths.
- Dimensions are fixed by the popup: 761px wide (416 + 1 + 288 + gaps + padding), at least 792px tall.

## Colors

### Attention
- **Attention** (`attention`, theme accent text): the unread dot, the play/pause control while playing.
- **Attention Fill** (`attention-fill`, theme accent) with `on-attention` text: today's calendar cell.
- **Critical** (`critical`, theme destructive text): the time of a critical-urgency row. Never used for anything else.

### Neutral
- **Print** (`print`, theme foreground): summaries, track title, day figures, world-clock times, temperature, action labels.
- **Dim** (`dim`, foreground at 68%): bodies, bylines, captions, offsets, non-critical times, dismiss and chevron glyphs at rest.
- **Rule** (`rule`, theme divider): every hairline (side headers, between rows, above the credits, above the catalogue line) and the 1px column divider.

### Named Rules
**The Attention Rule.** Accent marks only what wants a look now: unread, playing, today. Critical urgency takes the destructive colour instead, so it never reads as merely unread.

**The No Boxes Rule.** Nothing sits in a card or tinted panel. Hairlines and space organise the sleeve.

## Typography

One face, Cantarell. The tracklist uses one size (14px): the summary in bold, the body in regular dim. Scale contrast does the rest: 64px day figure, 20px track title and temperature, 11px uppercase captions. Figures that form columns (times, counts, temperatures) sit in fixed cells (0.6em per digit, half for the colon) because iced cannot request tabular figures.

## Layout

Two columns, 416px and 288px, separated by a 1px rule with 12px gaps; 16px popup padding.

- **Left:** cover (only while a player exists), then the sides list filling the height (scrolls), then the catalogue line pinned at the bottom. Sections 20px apart.
- **Right:** calendar header, hairline, month nav, grid; hairline; credits. Sections 16px apart.
- **Rows:** 10px vertical padding; picture column 28px; actions (expanded rows only) indented 26px so the first label lines up with the summary.

## Components

### Cover
120px album art (or a divider-coloured tile with a generic audio glyph), bottom-aligned with a 20px bold title (two lines max), a dim "artist · player" byline and three 20px flat controls. Play/pause uses the attention colour while playing.

### Side and Track Row (signature)
A side opens with the app name as an 11px uppercase caption, its row count and a clear-group button, over a hairline. Only the newest row shows until "+N more". A row: 24px sender picture (image data/file) or the app's icon (unresolvable names fall back to a generic glyph, never a blank slot), bold summary over a dim one-line body, then the unread dot, the time flush right (critical in the destructive colour), the chevron (when there is more to show or actions) and dismiss. Expanded rows show the full summary and body and the app's actions as bold flat text buttons.

Rows stay unread (dot visible) while the popup is open and are marked read when it closes.

### Calendar
64px bold day figure beside the weekday (15px bold) and month-year (13px dim). Month nav: dim flat arrows around the bold month name. Weekday initials are uppercase captions. Day cells are 30px; today is an accent-filled square with on-accent bold figures.

### Credits
"WORLD CLOCK": one row per configured zone: bold label, dim offset from local time (+1h, −8h), bold time. "WEATHER": place, dim condition, 20px bold temperature, dim "H 33° · L 25°" line under it; a dash until a reading exists. Both come from `io.github.agustine_leo.CosmicExtAppletClockCenter` (`world_clocks`, `weather`); weather is fetched from Open-Meteo every 30 minutes with the system curl, sending only rounded coordinates.

### Buttons
Flat everywhere: no fill at rest, the theme's component hover and pressed fills under the pointer, 4px radius. The Do Not Disturb toggler stays COSMIC's native control.

## Do's and Don'ts

### Do:
- Take every colour from the theme roles above.
- Keep times in fixed cells, flush right.
- Use hairlines and space to separate sections.

### Don't:
- Don't put rows, the cover or the credits in cards, panels or shadows.
- Don't add a label above a heading (no "NOW PLAYING" eyebrow; the player name lives in the byline).
- Don't use the accent for critical urgency, or the destructive colour for anything but critical.
- Don't show data the system does not have (no events, no track progress).

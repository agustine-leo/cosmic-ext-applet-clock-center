# Spacing polish + timer removal — design (short)

Date: 2026-11-21 · Base: v0.1.1 (aef7276)

## Intent
The user finds the popup "not polished as the GNOME one in terms of the spacing"
(reference: GNOME date menu). Make spacing consistent and aligned, and drop the
quick timers, which the user doesn't need.

## Decisions (user)
- Polish spacing to GNOME-like consistency.
- **Remove the quick timers entirely** (view, state, tick, critical/sound
  expiry, presets config).
- **Keep the calendar full-size** — no compact version: keep full weekday
  names (Sun, Mon, …) and the current cell/row size.

## Decisions (my calls)
- One spacing scale from the COSMIC theme spacing tokens: popup padding 16px
  (`space_s`), gap between sections 12px (`space_xs`), padding inside cards 12px,
  gap between cards 8px (`space_xxs`).
- Left column: every item shares the card edges — group header, cards,
  "+N more", and the Do Not Disturb / Clear row align to the same left and right
  edges; the group ×, the card's chevron/× and Clear share a right edge.
- Media player rendered as a card (same container style as notification cards).
- Right column: calendar only, with the same 16px inner padding as the left
  column; header text sizes unchanged unless misaligned.
- Popup height: driven by the taller of the calendar and a minimum (keep 520px);
  width recalculated so both columns keep their current proportions after the
  timer column content is gone (≈60/40).
- README and config: remove timer feature text and the `timer_presets` key;
  existing user config files with the key must still load (ignore unknown key).

## Approaches considered
- A. Fixed spacing constants from theme tokens (chosen — one sensible way;
  matches upstream COSMIC applets).
- B. User-configurable spacing now — deferred to the planned customization work.

## Non-goals
Calendar compaction, single-letter weekdays, today circle, customization
settings, behaviour changes to notifications/media/DND.

## Release
v0.2.0 (removes a feature). Verification via tests/build/`just install`; visual
check by the user after their next login (no panel/applet restarts).

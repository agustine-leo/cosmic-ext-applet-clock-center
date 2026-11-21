# Spacing Polish + Timer Removal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the quick timers and give the popup a consistent, GNOME-like spacing with aligned edges, released as v0.2.0.

**Architecture:** Delete the timer feature end to end (module, messages, subscription, view, own-config presets). Then route every popup spacing value through COSMIC theme spacing tokens, wrap the media player in the same card style as notifications, and align all left-column rows to the card edges. Calendar sizes stay as they are.

**Tech Stack:** Rust 2021, libcosmic 430f3d4 (locked), just.

**Spec:** `docs/superpowers/specs/2026-11-21-spacing-polish-design.md` (commit 81f6433)

## Global Constraints

- Spacing from theme tokens: popup padding = `space_s` (16), gap between sections = `space_xs` (12), padding inside cards = `space_xs` (12), gap between cards = `space_xxs` (8).
- Calendar keeps full weekday names (Sun, Mon, …) and its current cell/row size — no compaction.
- Left-column items (group header, cards, "+N more", DND/Clear row) share the card's left and right edges; group ×, card chevron/×, and Clear share a right edge.
- Media player rendered in the same card container style as notification cards.
- Popup keeps min height 520px; columns keep ≈60/40 proportions (LEFT_WIDTH 416, RIGHT_WIDTH 288).
- Never restart cosmic-panel, cosmic-notifications or the applet process; verification = `cargo test` + `cargo build --release --locked` + `just install`. The user checks visuals after their next login.
- Release as v0.2.0; never move existing tags.

## Review Focus

1. A user who still has `~/.config/cosmic/io.github.agustine_leo.CosmicExtAppletClockCenter/v1/timer_presets` on disk — applet must start normally (it simply no longer reads that config) → Task 1 verification step greps that nothing references OWN_CONFIG_ID.
2. Do Not Disturb still toggles after timer removal (the sound helper used `do_not_disturb`) → Task 1 keeps `dnd` tests green.
3. Very long app names in group header must not push the group × past the right edge → Task 2 `group_header` uses Fill + ellipsis.
4. Media card with no art and very long title stays one line, controls stay right-aligned → Task 2.
5. Popup width constant stays consistent with paddings after changing POPUP_PADDING → Task 2 test `popup_width_matches_layout`.

---

### Task 1: Remove the quick timers

**Files:**
- Delete: `src/timer.rs`
- Modify: `src/main.rs` (drop `mod timer;`), `src/app.rs` (fields `timer`, `timer_now`; messages `StartTimer`, `CancelTimer`, `TimerTick`; their `update` arms; `timer_tick` subscription; `crate::timer::view` in `view_window`; own-config load/watch and `Message::OwnConfigChanged` if they only served presets), `src/config.rs` (remove `ClockCenterConfig`, `sanitized`, its tests, and `OWN_CONFIG_ID` if unused), `README.md` (remove timer feature text and the canberra note; "calendar" only on the right), `i18n/en/*.ftl` (remove timer strings).

**Interfaces:**
- Produces: `App` without any timer state; popup right column = calendar only.

- [ ] **Step 1:** Make the removals above.
- [ ] **Step 2: Verify nothing references timers:** `grep -rniE 'timer|canberra|timer_presets|OWN_CONFIG_ID|OwnConfigChanged' src README.md i18n` → no matches (except unrelated words, report any).
- [ ] **Step 3:** `cargo test` → all remaining tests pass (expect 49 minus the removed timer/config tests); `cargo build --release --locked` → 0 warnings.
- [ ] **Step 4: Commit + push:** `git commit -m "feat!: remove quick timers"`.

### Task 2: Consistent spacing and aligned edges

**Files:**
- Modify: `src/app.rs` (constants at lines ~23-30 and `view_window`), `src/notifications/view.rs` (group header, card, "+N more", `bottom_row`), `src/media.rs` (`view`), `src/calendar.rs` (outer padding only).

**Interfaces:**
- Consumes: theme spacing via `cosmic::theme::spacing()` (fields `space_xxs`, `space_xs`, `space_s`).
- Produces: `pub fn popup_width(left: f32, right: f32, padding: f32, gap: f32) -> f32` in `src/app.rs` (= left + 1 divider + right + 2*gap + 2*padding).

- [ ] **Step 1: Failing test** in `src/app.rs`:
```rust
#[test] fn popup_width_matches_layout() {
    assert_eq!(popup_width(416.0, 288.0, 16.0, 12.0), 416.0 + 1.0 + 288.0 + 2.0*12.0 + 2.0*16.0);
}
```
- [ ] **Step 2:** `cargo test popup_width` → FAIL (fn missing).
- [ ] **Step 3: Implement** `popup_width`; set `POPUP_PADDING = 16.0`, `COLUMN_SPACING = 12.0`; compute `POPUP_WIDTH` via `popup_width` (const-compatible arithmetic is fine; keep the popup_limits override in sync). Apply spacing:
  - `view_window`: outer padding `space_s`; column gap `space_xs`; divider full height between columns.
  - Left column: one container with no extra horizontal padding so every row's left/right edge = card edge; sections separated by `space_xs`; cards by `space_xxs`; card inner padding `space_xs`.
  - `media::view`: wrap in the same card container class as notification cards, inner padding `space_xs`, art 48px, title ellipsized one line, controls right-aligned.
  - Group header row: app name `Length::Fill` ellipsized, × right-aligned to the card edge.
  - "+N more" / "Show less": aligned to the card's left edge (no indent).
  - `bottom_row`: "Do Not Disturb" label left at card edge; toggler + "Clear" right, Clear's right edge = card right edge.
  - Right column: calendar with the same `space_s`-consistent inner alignment; no size changes to cells, header text or weekday labels.
- [ ] **Step 4:** `cargo test` → PASS; `cargo build --release --locked` → 0 warnings; `just install`.
- [ ] **Step 5: Commit + push:** `git commit -m "style: consistent spacing and aligned edges"`.

### Task 3: Release v0.2.0

**Files:** Modify `Cargo.toml` (version 0.2.0), `Cargo.lock` (via `cargo build --locked` after bump: run `cargo update -p cosmic-ext-applet-clock-center --precise 0.2.0` or plain `cargo build` once to refresh the own-package entry), `README.md` (features list without timers).

- [ ] **Step 1:** Bump version; `cargo build --release --locked` passes; `just install`.
- [ ] **Step 2:** Commit `chore: v0.2.0`, push, `git tag v0.2.0 && git push origin v0.2.0`.

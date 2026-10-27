---
id: TASK-085
title: UI design adjustments (interactive session with the user)
status: Done
assignee:
  - '@supervisor'
created_date: '2026-10-27 19:45'
updated_date: '2026-10-27 19:49'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - assets/ui/README.md
  - crates/rb-ui-art/src/parts.rs
  - crates/rb-ui-art/src/layout.rs
priority: medium
type: enhancement
ordinal: 106000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested by the user on 2026-10-27: an interactive design session on the rb-ui panel. The user ran the app after each round and listed what to change; the supervisor changed the art generator (crates/rb-ui-art) and the app. The section rectangles stay the original's (docs/ui_spec.md); the placement of the controls inside them becomes rb's own design, so the art test that compared layout.tsv with the spec's control rectangles is removed (ui_spec.md records facts about the original and must not follow the redesign).

The changes, by area:
- Transport: no SYNC/MIDI; an icon-only 18x18 loop toggle right of LENGTH; a 22 px shuffle knob centered in its space; no box behind the logo. Every two-position switch flips on a click anywhere.
- Pattern: 13x13 flat on/off lamp, 17x20 bank buttons, a 30x12 SHUFFLE button level with the steps display.
- 303: one button size; LEDs 3 px above and centered on their keys; Step/Back as up/down arrows; the key band in three sections 6 px apart (pitch/clear, keys on gray, one row of DOWN/UP/ACCENT/SLIDE/NOTE under the edit step display); waveform switch centered over pitch/clear with no title; knobs 40 px apart; NOTE/REST is one LED (lit = note).
- 808: steps centered under the legend; a larger 38x38 selector whose pointer snaps to the selected drum; TUNE labels.
- 909: knobs and labels centered in their columns; steps under the name cells; AC selected from its chip; FLAM knob at the bottom of the AC column above a 14x8 flam light; no empty space at the bottom.
- Mixer, master, effects: MIX/DIST/COMP titles; 14x14 flat on/off lamps and 8x14 bare meters in every header; 22 px knobs; narrower routing buttons; CLIP scale label; label placement (FEEDBACK, MODE, fader names at the bottom).
- Labels: the stroke font's letter gap grows with the thickened small-label stroke, so letters no longer merge.
- App icon: a 303 knob on a slate tile (part app_icon), set as the window icon.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The panel art and layout carry the changes listed in the description (assets/ui regenerated; ui-art check reports 0 problems)
- [x] #2 Every two-position switch flips on a click anywhere; the 808 selector snaps to the selected drum; the 909 AC is selected from its chip
- [x] #3 The window has the app_icon icon
- [x] #4 assets/ui/README.md describes the current parts; cargo test -p rb-ui -p rb-ui-art, clippy and fmt pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: ui-design-1 (supervisor, interactive with the user).
Rounds: the user's first change list, then 303 band spacing/PITCH/FLAM/header borders/PAN, NOTE into the flag row + EDIT STEP above it, flat headers, 303 top row placement (centered, right, left), font letter gap, shuffle knob/logo box/waveform alignment, a grille tried and removed in favor of 40 px knob spacing, the app icon.
Removed parts: knob_18, led_11_*, btn_step_*, btn_pill_15x8_*, sel_22x18, meter_13x16_*/meter_14x15_*, power_13x14/16x16/17x15. Removed test layout_matches_the_spec_rectangles (docs/ui_spec.md is the original's layout). rb-ui depends on resvg 0.45.1 (the version egui_extras already uses) to render the icon.
Reproduce: cargo run -q -p rb-ui-art -- gen && cargo run -q -p rb-ui-art -- check; cargo test -q -p rb-ui -p rb-ui-art (121 pass).

Review round 1 (send back): README selector angle step fixed (270/11); stale sync_led/midi_led removed from crates/rb-ui/tests/wired.rs OTHER; resvg declared with default-features = false (lock gains only the rb-ui dependency line, no new packages). The Co-Authored-By line is correct: the supervisor runs on Opus 5.5.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Interactive UI design session with the user on branch ui-design-1: panel art, layout and app changes listed in the description, the font letter-gap fix, the app icon, README updated. ui-art check: 163 parts x 3 scales, 0 problems. cargo test -p rb-ui -p rb-ui-art: 121 pass; clippy and fmt clean.
<!-- SECTION:FINAL_SUMMARY:END -->

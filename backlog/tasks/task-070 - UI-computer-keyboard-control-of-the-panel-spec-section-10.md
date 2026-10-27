---
id: TASK-070
title: 'UI: computer-keyboard control of the panel (spec section 10)'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 13:25'
updated_date: '2026-10-27 14:53'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-044
  - TASK-054
references:
  - docs/ui_spec.md
  - crates/rb-ui/src/app.rs
priority: medium
type: feature
ordinal: 95000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Task-044 built the panel with only Space (play/stop) and Up/Down (focus). Add the rest of docs/ui_spec.md section 10 to crates/rb-ui (app.rs keyboard(), model.rs), with the mapping decided in task-054 (physical US positions): numpad transport keys (Enter play, 0 stop, 4 rewind, 5 fast forward, 7/8 bar down/up, 1/2 locate to loop start/end, * and Insert record, +/- tempo, / loop), Select Patterns from Keyboard (1-8, Q-I, A-K, Z-comma per row) and Program Synth from Keyboard (303 keys C F V G B N J M K , L . /, Enter Step, Backspace Back, P Accent, [ Slide, ; Down, ' Up, the key right of 0 (- on US; = as alias) Note/Pause, Tab tap and Shift+Tab delete of the playing step; the 808/909 tap keys, AC on the key right of 0 with = as alias), with the Options-menu mode as app state. Key presses act like mouse presses on the buttons. egui does not report numpad keys on every platform: record what works.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Keys by physical position (physical_key, logical fallback) drive the transport, Select Patterns and Program Synth keys, 303 Tab/Shift+Tab and drum taps; '-' and '=' work on a non-US logical layout (tests/keyboard.rs)
- [x] #2 No key reaches the panel while a browser is open; Ctrl+F/Ctrl+G set the mode; what egui cannot report (numpad vs main keys) is recorded in docs/ui_spec.md section 10
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. keys.rs: pure map from the physical key (egui Event::Key.physical_key, logical key if missing) + mode + focus to an Action. 2. model.rs: locate_loop, 303 and drum tap methods, playing_step303 (live byte 9, link.rs). 3. app.rs: key mode state (Ctrl+F/Ctrl+G), event-driven keyboard() with held-key state (pressed art, repeat schedule of the wind/repeat buttons), nothing while a browser is open. 4. tests/keyboard.rs headless (events+messages via wired.rs pattern, '-' and '=' on a non-US logical layout). 5. docs/ui_spec.md section 10/16 update incl. what egui cannot tell (numpad vs main keys).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The key mapping is settled by task-054 (Done 2026-10-27, option C): keys by physical US position (scancode) per the manual's pictures. The TB-303 Note/Pause key and the drum AC key are the key right of '0' ('-' on a US keyboard); '=' is an alias for both. Numpad 4 / 5 are rewind / fast forward. The dependency is satisfied.

Branch: worktree-agent-a6cfddca977cf810d

Evidence: egui-winit 0.37 key_from_key_code merges Numpad0..9/Enter/Subtract/Divide into the main keys' Key; NumpadMultiply has no Key (Insert records); NumpadAdd = Plus is distinct. winit always supplies physical_key for known scancodes; the Key event is absent only if both logical and physical are unknown. Resolution of the collisions by mode/focus is in keys::map and docs/ui_spec.md section 10. Failed hypothesis: telling numpad from main keys via Event::Key (impossible). Not done / unconfirmed: drums' Shift+Tab deletion (ui_spec 15, which instrument?); locate 1/2 details and drum-tap selection are [G]. Tests: cargo test --release -p rb-ui (keyboard.rs 10 tests, keys unit tests; interaction.rs space test now releases the key because egui marks a second press of a held key as repeat). Linked test checks the messages: Play {1,1}, 303 steps sub 45, pattern select sub 40.

Review round 1: (1) commit area changed to tools. (2) PgUp/PgDn are not wired: rb's panel is a fixed letterboxed 624x510 image, no scrolling rack; stated in ui_spec sections 10 and 16 (the [G] in 15 stays open). (3) Stated plainly in section 10: in Program Synth (default) main-row digits 0 1 2 4 5 7 8 are transport keys, Enter is Play outside a 303 and / is loop, unlike the original; pattern keys only in Select Patterns; forced by egui 0.37 having no numpad identity. (4) Test a_held_key_shows_its_button_pressed (App::key_held, which drives the pressed art, now pub). (5) Shift+= is tempo up (keys.rs), not the Note/Pause/AC alias.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Computer keyboard of the panel (spec section 10) in crates/rb-ui: keys.rs maps physical-position keys (logical fallback) to actions; App::keyboard applies them like mouse presses (pressed art while held, repeat schedule for tempo/bar/rewind/ff, nothing while a browser is open or with Ctrl); Ctrl+F/G key mode; Panel::locate_loop, tap_303, tap_drum, playing_step303 (live byte 9). Numpad-vs-main ambiguity of egui 0.37 resolved by mode/focus and documented in ui_spec section 10. Tests: cargo test --release (see commit). Branch worktree-agent-a6cfddca977cf810d, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

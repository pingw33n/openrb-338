---
id: TASK-105
title: 'rb-ui: Help > Keyboard Shortcuts window'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:55'
updated_date: '2026-10-28 12:24'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-102
references:
  - crates/rb-ui/src/keys.rs
  - crates/rb-ui/src/shortcuts.rs
  - docs/ui_spec.md
priority: low
type: feature
ordinal: 125000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for Help > Keyboard Shortcuts, which opens a window explaining all of rb-ui's keyboard shortcuts. task-102 adds the Help menu (with About); this item goes in it.

What:
- A Help menu item 'Keyboard Shortcuts' that opens a centered, closable, scrollable window; Escape closes it, as the other windows do.
- The window lists every key rb-ui handles, grouped:
  - the menu shortcuts: New, Open, Save, and the two keyboard modes, with the platform's text from crates/rb-ui/src/shortcuts.rs (Cmd on macOS);
  - the always-active transport keys;
  - Program Synth mode: the 303's piano keys, Step/Back, Accent, Slide, the octave keys, Note/Pause and Tab/Shift+Tab, and the drums' instrument keys and AC;
  - Select Patterns mode: the four key rows and their sections;
  - focus and other keys.
- It marks the current keyboard mode.
- The content comes from what rb actually does: crates/rb-ui/src/keys.rs (keys::map) and docs/ui_spec.md section 10, including rb's deviations (a key that has no effect in a mode, such as tempo down in Program Synth, and the numpad keys that egui cannot tell apart). Build the table from one data source shared with, or checked against, keys::map, so the window cannot drift from the bindings. A test enforces that every listed key maps to the listed action, and that every mapped key is listed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Help > Keyboard Shortcuts opens a centered window listing all keys by group with the platform's modifier text; the current mode is marked; Escape and the close button close it
- [x] #2 A test checks the listed keys against keys::map (each listed key does what it says, no mapped key is missing)
- [x] #3 docs/ui_spec.md section 18 lists the item; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. New module crates/rb-ui/src/key_help.rs: one table (groups of rows; each row has the key text, the description and Bind entries: key, shift, scope (mode and focus rows), expected keys::Action). Menu-shortcut rows take their text from shortcuts.rs. 2. Window: Help > Keyboard Shortcuts (HELP_ITEMS, Cmd::KeyHelp), centered like About (pivot, default_pos, id per open), scrollable, closable, Escape consumed after the other windows, current mode marked. 3. Tests in key_help.rs: every Bind equals keys::map; every (key, shift, mode, focus) over egui Key::ALL that keys::map maps is covered by a Bind with the same action; the menu rows match shortcuts.rs; mutation check noted. Integration test in tests/menu.rs for open/center/Escape/close/mode mark. 4. docs/ui_spec.md section 18 gets the item. 5. fmt, clippy, cargo test --release.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a7f4aa12da165b035

Design: crates/rb-ui/src/key_help.rs holds one table (key_help::groups(is_mac)); each row lists Binds (key, Shift state, Scope = modes and focused rows, expected keys::Action). Unit tests: every_listed_key_maps_to_its_action (each bind through keys::map in every mode/row it covers), every_mapped_key_is_listed (all egui Key::ALL x shift x mode x row: any Some(action) from keys::map must be covered by a bind with that action), the_binds_do_not_contradict_each_other, menu_rows_use_the_platform_text (Cmd/Ctrl from shortcuts.rs). Integration test the_key_help_window_opens_centered_marks_the_mode_and_closes in tests/menu.rs (centered twice, mode line for both modes, Escape, close button).
Mutation check: temporarily changed keys.rs 'Key::P => "accent"' to "slide": every_listed_key_maps_to_its_action failed (P shift=false Synth focus=0, left Press(tb303_1.slide), right Press(tb303_1.accent)) and every_mapped_key_is_listed failed (P ... -> Press(tb303_1.slide) is not listed). Reverted.
The group '(current mode)' mark is not asserted in the integration test: the 303 group is below the scroll area's fold and its text is culled from the shapes; the top line 'Keyboard mode now: ...' is asserted for both modes.
Final checks: cargo fmt, clippy --all-targets -D warnings clean, cargo test --release -q: 521 passed, 0 failed (RB_MOD_DIR set).

Review round 1 (send back), fixed: (1) new group 'Prompts and questions' (save prompt S/Y/Enter, D/N, Esc; browser overwrite question Y / Enter,Esc,N) and the prompt's Esc in the Escape row; (2) a test of the prompt and browser handlers would need the full browser harness, so those groups are marked in their notes and in docs/ui_spec.md section 18 as listed by hand and unchecked; (3) is_current(&Group, KeyMode) is a pure fn, unit-tested (the_current_mode_marks_its_groups: Synth marks the two Program Synth groups, Patterns only Select Patterns; replaces each_mode_has_groups); (4) row renamed 'Numpad 0 (and the main-row 0)'. fmt, clippy clean; cargo test --release -q: 521 passed, 0 failed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Help > Keyboard Shortcuts: crates/rb-ui/src/key_help.rs (table, window, tests), Cmd::KeyHelp and a HELP_ITEMS entry in menu.rs, key_help_open/serial in app.rs, drawn after About. Centered like About, resizable, scrollable, closes with Escape or the close button, names the current keyboard mode and marks its groups (pure fn is_current, unit-tested). Groups: menu shortcuts (platform text from shortcuts.rs), always active, mode-dependent transport keys, Program Synth with a 303 or a drum machine focused, Select Patterns, windows and browser, prompts and questions. rb's deviations are written in it. The key groups are checked against keys::map both ways (a keys.rs mutation fails both tests); the browser and prompt groups are listed by hand and marked so in the window and in docs/ui_spec.md section 18. cargo test --release: 521 pass; clippy clean. Branch worktree-agent-a7f4aa12da165b035, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

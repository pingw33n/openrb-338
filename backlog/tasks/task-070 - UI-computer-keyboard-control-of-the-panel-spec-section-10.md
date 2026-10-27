---
id: TASK-070
title: 'UI: computer-keyboard control of the panel (spec section 10)'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-27 13:25'
updated_date: '2026-10-27 13:29'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The key mapping is settled by task-054 (Done 2026-10-27, option C): keys by physical US position (scancode) per the manual's pictures. The TB-303 Note/Pause key and the drum AC key are the key right of '0' ('-' on a US keyboard); '=' is an alias for both. Numpad 4 / 5 are rewind / fast forward. The dependency is satisfied.
<!-- SECTION:NOTES:END -->

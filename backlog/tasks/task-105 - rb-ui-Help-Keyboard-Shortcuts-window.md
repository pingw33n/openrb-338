---
id: TASK-105
title: 'rb-ui: Help > Keyboard Shortcuts window'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:55'
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
- [ ] #1 Help > Keyboard Shortcuts opens a centered window listing all keys by group with the platform's modifier text; the current mode is marked; Escape and the close button close it
- [ ] #2 A test checks the listed keys against keys::map (each listed key does what it says, no mapped key is missing)
- [ ] #3 docs/ui_spec.md section 18 lists the item; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

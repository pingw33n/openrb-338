---
id: TASK-092
title: 'rb-ui: menu shortcut text says Cmd on macOS'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:50'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/menu.rs
priority: low
type: bug
ordinal: 112000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user reported (2026-10-28) that on macOS the menu shows shortcuts as Ctrl+O, Ctrl+S, Ctrl+M, Ctrl+F, Ctrl+G..., but the bindings use egui Modifiers::COMMAND (app.rs, browser.rs), which is Cmd on macOS. The texts are hard-coded strings in menu.rs (item(ui, "Open...", "Ctrl+O"), the radio labels "Program Synth (Ctrl+F)" and "Select Patterns (Ctrl+G)", and any others).

What: build every shortcut text from the binding, for example with egui's KeyboardShortcut::new(Modifiers::COMMAND, Key::O) and ctx.format_shortcut (which gives the macOS symbol form) or a small helper that writes Cmd+ on macOS and Ctrl+ elsewhere; ideally the same constant is used for the binding and the text so they cannot drift. Check the module doc comment of menu.rs and docs/ui_spec.md for Ctrl+ wording that should name both.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On macOS every menu shortcut text shows Cmd (or the egui macOS symbol form); on other platforms Ctrl
- [ ] #2 Each shortcut's text and binding come from one definition
- [ ] #3 A test checks the text for both platforms (cfg or an explicit OS parameter)
- [ ] #4 cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

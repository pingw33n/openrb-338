---
id: TASK-092
title: 'rb-ui: menu shortcut text says Cmd on macOS'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:50'
updated_date: '2026-10-28 06:33'
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
- [x] #1 On macOS every menu shortcut text shows Cmd (or the egui macOS symbol form); on other platforms Ctrl
- [x] #2 Each shortcut's text and binding come from one definition
- [x] #3 A test checks the text for both platforms (cfg or an explicit OS parameter)
- [x] #4 cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Add crates/rb-ui/src/shortcuts.rs: KeyboardShortcut consts (New, Open, Save, Keys Synth/Patterns, EditPath) used by both consume_shortcut and menu/tooltip text (ModifierNames::NAMES, is_mac from ctx.os()). Test text_for for mac and non-mac.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-adf3822114ea52248

Shortcuts live in crates/rb-ui/src/shortcuts.rs: KeyboardShortcut consts used by consume_shortcut (app.rs, browser.rs) and by the text (ModifierNames::NAMES, is_mac from ctx.os(); symbol form not in the default font). Test shortcuts::tests checks Cmd+X and Ctrl+X for all six. No Ctrl+M item exists in menu.rs. Browser tooltip also derived. Not run in the GUI.

Review round 1 done: spec rows and comments name Ctrl and Cmd.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Menu shortcut texts (File New/Open/Save, Options radios) and the browser path tooltip now come from the same KeyboardShortcut constants as the key bindings (shortcuts.rs): Cmd on macOS, Ctrl elsewhere. Test for both platforms; clippy clean; cargo test --release 455 pass. Branch worktree-agent-adf3822114ea52248.
<!-- SECTION:FINAL_SUMMARY:END -->

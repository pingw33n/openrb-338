---
id: TASK-091
title: 'rb-ui: File > New resets to the state of a fresh start'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:50'
updated_date: '2026-10-28 05:51'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-090
references:
  - crates/rb-ui/src/menu.rs
  - crates/rb-ui/src/app.rs
  - docs/ui_spec.md
priority: medium
type: feature
ordinal: 111000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for File > New. rb-ui has no File New (docs/ui_spec.md section 18 says so); at start it opens the empty 4.2 song (rbs::Song::empty) with the standard mod (App::start). The original's File New (command 100, Ctrl+N, sub_40A581 -> sub_426CA8; docs/ui_spec.md sections 12 and 18) opens a new Untitled song from a template; the user wants rb's version to reset the state as if the app was freshly started instead.

What: add File > New (Ctrl+N, Cmd+N on macOS; the shortcut bound with egui Modifiers::COMMAND like Open and Save) at the top of the File menu. It goes through the same unsaved-changes prompt as Open; on confirm it stops playback and puts the app in the state App::start gives without a command-line song: the empty song, Untitled, the standard mod, not modified, the start state of the transport and panel. Whether the startup sound plays follows App::start's rule (docs/ui_spec.md 18.1). Settings (the settings file) are not reset. Factor the start state so New and start share one function.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 File > New with its shortcut is in the File menu and asks the unsaved-changes prompt for a modified song
- [ ] #2 After New the app state equals a fresh start without a song argument (empty Untitled song, standard mod, stopped, unmodified); settings are kept
- [ ] #3 Start and New share the code that builds the start state; a test compares the state after New with the state after start
- [ ] #4 docs/ui_spec.md section 18 lists the item; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

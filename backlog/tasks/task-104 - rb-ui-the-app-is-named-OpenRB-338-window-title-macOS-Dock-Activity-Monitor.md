---
id: TASK-104
title: >-
  rb-ui: the app is named OpenRB-338 (window title, macOS Dock, Activity
  Monitor)
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:44'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/Cargo.toml
  - crates/rb-ui/src/main.rs
  - crates/rb-ui/src/menu.rs
priority: medium
type: enhancement
ordinal: 124000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that the desktop app be named OpenRB-338 in the window title, in the macOS Dock tooltip, and in the macOS Activity Monitor process list. Today the title is built by App::update_title (song name and rb-ui), and the process is the binary rb-ui, so the Dock and Activity Monitor show rb-ui.

What:
- The window title says OpenRB-338 where it says rb-ui now (keep the song name and the modified mark).
- eframe::run_native's app name and the viewport's app_id are OpenRB-338.
- On macOS the Dock and Activity Monitor show the process name. For an unbundled binary that is the executable's file name. The straightforward fix is to name rb-ui's binary OpenRB-338 ([[bin]] name in crates/rb-ui/Cargo.toml; the crate stays rb-ui). Then update every mention of target/release/rb-ui and the rb-ui command in docs/, AGENTS.md, the usage text and tests (e.g. env!("CARGO_BIN_EXE_rb-ui")). If you find that setting the process name at run time (NSProcessInfo setProcessName) also covers both places, compare the two and choose; record why. Verify on this Mac: run the binary, check ps -o comm= for the process, and say in the notes what the Dock tooltip shows (a screenshot via screencapture if you can, otherwise state it was not checked).
- Out of scope: task-068 (the project-wide rename: the CLI binary orb, the crate prefixes, the ORB_ environment variables). Do not do that here; mention in task-068's notes (backlog CLI) that the UI's name was done by this task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The window title shows OpenRB-338 (with the song name and modified mark)
- [ ] #2 On macOS the process is listed as OpenRB-338 (ps / Activity Monitor) and the Dock shows OpenRB-338; how it was checked is in the notes
- [ ] #3 Every mention of the old binary name in docs, AGENTS.md, usage text and tests is updated; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

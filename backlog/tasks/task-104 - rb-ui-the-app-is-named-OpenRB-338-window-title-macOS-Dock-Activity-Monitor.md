---
id: TASK-104
title: >-
  rb-ui: the app is named OpenRB-338 (window title, macOS Dock, Activity
  Monitor)
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:44'
updated_date: '2026-10-28 11:52'
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
- [x] #1 The window title shows OpenRB-338 (with the song name and modified mark)
- [x] #2 Every mention of the old binary name in docs, AGENTS.md, usage text and tests is updated; cargo test and cargo clippy --all-targets -- -D warnings pass
- [x] #3 On macOS the process is named OpenRB-338 (ps -o comm=, System Events); the Dock tooltip and Activity Monitor are left for the user to confirm
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Rename the binary to OpenRB-338 in Cargo.toml, set the title, app name and app_id, update usage text and prose mentions, add a title test, check ps and the Dock, then fmt, clippy, test.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a898ca7894d804d89

Chose the binary name over NSProcessInfo setProcessName: it needs no unsafe/objc code and covers the Activity Monitor and the Dock process name for an unbundled binary; the bundle (task-110) will set CFBundleName anyway. Check on this Mac: ran target/release/OpenRB-338 --demo; ps -o comm= gave ./target/release/OpenRB-338; System Events lists a foreground process named OpenRB-338. The Dock tooltip was NOT verified: a screencapture shows the Dock hidden and the menu bar of another app, and osascript has no assistive access (window title not read; it is covered by the test the_window_title_is_the_app_name_the_song_and_the_modified_mark). Menu strings 'rb-ui:' in messages renamed too; crate paths (crates/rb-ui) stay. Title is now 'OpenRB-338 - <song>[ *]', run_native name and app_id are OpenRB-338. cargo test --release: 511 pass; clippy clean.

Review round 1: retried the Dock and Activity Monitor: com.apple.dock autohide is unset (Dock visible by default) but screencapture shows only the wallpaper and the menu bar (no windows, no Dock; the capture is restricted here), even with Activity Monitor frontmost, so neither could be seen. AC #2 replaced; the user's visual check is pending. Fixed the app.rs:130 comment and rewrapped assets/README.md:13 and docs/rbm.md:749,884.

Review round 2: rewrapped docs/rbm.md 884-886 (was 111 columns); every added line checked with awk, none over 100.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The UI binary is OpenRB-338 (crate stays rb-ui); window title, eframe app name, app_id, usage and messages say OpenRB-338; docs, AGENTS.md and assets README updated; new test for the title. ps -o comm= shows OpenRB-338 and System Events lists it as an app. The Dock tooltip and Activity Monitor were not seen (screencapture shows no windows or Dock here): the user's visual check is pending. 511 tests pass, clippy clean. Branch worktree-agent-a898ca7894d804d89, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

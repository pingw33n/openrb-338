---
id: TASK-091
title: 'rb-ui: File > New resets to the state of a fresh start'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:50'
updated_date: '2026-10-28 06:24'
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
- [x] #1 File > New with its shortcut is in the File menu and asks the unsaved-changes prompt for a modified song
- [x] #2 After New the app state equals a fresh start without a song argument (empty Untitled song, standard mod, stopped, unmodified); settings are kept
- [x] #3 Start and New share the code that builds the start state; a test compares the state after New with the state after start
- [x] #4 docs/ui_spec.md section 18 lists the item; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Factor App::fresh (start without song and File New share it); Link::load_new + Player::build_new give the player the constructor's empty song (the start state the panel syncs to); Then::New prompt, Cmd::New, Ctrl+N, menu item; test; spec 18.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a6bb30504c0b04285

Finding: a start loads nothing, so the panel syncs to the engine constructor's song (loop 1 bar, rows enabled), unlike Panel::new defaults or convert(Song::empty()) (loop 4 bars, rows off). New therefore queues a Player::build_new engine (no load) so New equals the start. start(None) now goes through the same App::fresh. Test file_new_gives_the_state_of_a_fresh_start compares panel (sorted pretty print) and document before and after. cargo test --release: 453 pass, 0 fail; clippy clean.

Review round 1: (1) Link::load_new now queues the song (forwarding the old panel events into the old mirror) before assigning patterns/row_pattern; (2) menu.rs doc rewrapped; (3) spec says Link::load_new; (4) tests: pending 808 step at New leaves the pattern mirror equal to a fresh start's, and file_new_plays_the_startup_sound_when_the_preference_is_on. cargo test --release 454 pass, clippy clean.

Review round 2: new_now comment names Link::load_new. Non-vacuity: with the old ordering (patterns/row_pattern assigned before queue_song) file_new_gives_the_state_of_a_fresh_start FAILS at tests/menu.rs:1260 ('the pattern mirror is the fresh start's'); with the fix restored it passes. Workspace 454 pass, clippy clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
File > New (Ctrl+N/Cmd+N) added at the top of the File menu, behind the unsaved-changes prompt (Then::New). App::fresh is shared by App::start(None) and New: Link::load_new queues Stop plus a constructor-fresh engine (Player::build_new), panel reset, startup sound by the preference, settings kept. Test file_new_gives_the_state_of_a_fresh_start; spec section 18 updated. cargo test --release 453 pass; clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-090
title: 'rb-ui: opening a song while one is playing stops playback'
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
  - crates/rb-ui/src/app.rs
  - docs/ui_spec.md
priority: medium
type: bug
ordinal: 110000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user reported (2026-10-28) that opening a song (File > Open..., Ctrl/Cmd+O, a file dropped or picked in the browser, or any other path that replaces the current song) while the current song is playing keeps playing. It should stop: after the open the transport is stopped and the new song sits at its start, as after a fresh load.

What: find every path in rb-ui that replaces the current song (App::adopt and its callers in menu.rs/app.rs; the mod Demo Song and File > New tasks will add more, so put the stop where they all pass) and stop playback before (or as part of) the switch. If the switch is refused (the unsaved-changes prompt is cancelled, the song fails to load or the pool refuses it, task-088) the current song keeps playing as it was. Record what the original does in the same case in docs/ui_spec.md if it is known from docs or quick to check in the disassembly (with addresses); the user's requirement is the stop either way.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Opening another song while playing leaves the transport stopped, with the new song loaded at its start
- [ ] #2 A cancelled or failed open leaves the current song playing
- [ ] #3 A test in crates/rb-ui/tests covers both cases
- [ ] #4 docs/ui_spec.md describes the behavior; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

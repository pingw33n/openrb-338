---
id: TASK-090
title: 'rb-ui: opening a song while one is playing stops playback'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:50'
updated_date: '2026-10-28 06:07'
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
- [x] #1 Opening another song while playing leaves the transport stopped, with the new song loaded at its start
- [x] #2 A cancelled or failed open leaves the current song playing
- [x] #3 A test in crates/rb-ui/tests covers both cases
- [x] #4 docs/ui_spec.md describes the behavior; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Link::load (crates/rb-ui/src/link.rs), the one place every song replacement passes (App::adopt from open, start, later New/Demo), queues Command::Stop before Command::Load, after the node pool accepted the song (a refused load returns early, so the song keeps playing; a cancelled prompt or failed read never reaches load). Stop on the current song makes activate() switch to the new song at once, stopped, at its start. 2. Test in crates/rb-ui/tests/menu.rs: play, open another song, transport stopped + position 0 + new doc; refused/failed open keeps playing. 3. docs/ui_spec.md section 18 text. 4. fmt, clippy, tests.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a05822fcf0c0590f9

Fix in Link::load (crates/rb-ui/src/link.rs): Command::Stop queued before Command::Load, after the pool accepted the song. Player activate() then sees a stopped current song and makes the new one current at once. Old behavior left the new song pending behind the playing one; wired.rs a_song_loaded_while_another_plays_waits_for_play encoded it and was rewritten (..._stops_it). New test menu.rs opening_a_song_while_playing_stops_it_and_a_refused_open_does_not covers success, unreadable file, cancelled prompt, pool refusal. Original's UI behavior on open not traced (marked [G] in docs/ui_spec.md); the player-level pending-song behavior is from docs/export_state.md section 3.

Review round 1: (1) docs/ui_spec.md original-behavior claim cut to what export_state.md section 3 supports, rest marked [G]; (2) rewrapped Link::load doc comment; (3) commit Testing line gives the count (452 pass).

Review round 2: rewrapped the whole Link::load doc paragraph; awk 'length > 100' on link.rs is empty.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Opening a song while one plays now leaves the transport stopped with the new song at its start; refused/cancelled/failed opens keep playing. Stop is queued in Link::load before Command::Load. Tests: new menu.rs test, wired.rs test rewritten; docs/ui_spec.md section 18 describes it (original's UI behavior marked [G]). fmt, clippy -D warnings and cargo test --release pass (RB_STANDARD_MOD, RB_MOD_DIR set). Branch worktree-agent-a05822fcf0c0590f9, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

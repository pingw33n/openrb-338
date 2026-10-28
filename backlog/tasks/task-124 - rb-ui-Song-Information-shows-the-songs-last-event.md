---
id: TASK-124
title: 'rb-ui: Song Information shows the song''s last event'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 14:43'
updated_date: '2026-10-28 18:39'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-122
references:
  - crates/rb-ui/src/info.rs
  - crates/rb-cli/src/info.rs
priority: low
type: enhancement
ordinal: 143000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that File > Info (the Song Information window) show the bar of the song's last track event, as `rb info` does after task-122 ('Last event: bar N', SongFacts in crates/rb-cli/src/info.rs). An .rbs song has no length field; the original plays to the end of bar 999 (tick 767231) unless the loop is on, so the last event is the useful 'length'.

What: add a row 'Last event: bar N' (or 'none') to the Song Information window among the read-only facts, taken from SongFacts (no second computation). It follows the current song like the other rows; for a 1.x song it needs the real facts (task-123 fixes that path; coordinate or depend on it). Update docs/ui_spec.md section 18 and the window's test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Song Information shows 'Last event: bar N' (or none) from SongFacts, for 4.x and 1.x songs; a test checks the row
- [x] #2 docs/ui_spec.md section 18 updated; cargo +nightly fmt --check, cargo test and clippy pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Add SongInfo.last_event_bar from SongFacts::last_event_bar, a 'Last event' row, unit test, ui_spec. Review round 1: follow recordings via rb_cli::info::last_tick over the Link's recorded tracks; after Save doc.source = composed song for any song; test the_last_event_row_follows_a_recording_and_save (4.x and 1.x).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a01260dbd21b6daea

Test last_event_row_from_the_facts fails with the field forced to None (left: None, right: Some(3)) and passes with it. fmt check, clippy -D warnings, cargo test -p rb-ui (52 pass), release workspace test with CD ISO and third-party dirs: all ok.

NOTE: the first plan was recorded after the code was written. Review round 1 (send back): row stale after recording/Save. Fixed: App::song_info passes Link::recorded_tracks to SongInfo::of, which takes the tick from rb_cli::info::last_tick (also used by SongFacts::of_1x); Save sets doc.source to the composed song for any song (menu.rs); test the_last_event_row_follows_a_recording_and_save in tests/menu.rs (fails with recorded tracks ignored: left Some(1), right Some(3)); ui_spec updated. Counts: cargo test -p rb-ui (debug) 193 pass summing every test result line; release workspace with the three env vars 580 pass, 0 failed. fmt and clippy clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
File > Info shows 'Last event: bar N' (or none) for 4.x and 1.x songs, from SongFacts::last_event_bar (rb_cli::info), following recordings (Link recorded tracks via rb_cli::info::last_tick, also used by SongFacts::of_1x) and Save (doc.source becomes the composed song after any successful Save). Tests: unit last_event_row_from_the_facts and menu test the_last_event_row_follows_a_recording_and_save (4.x and 1.x). Branch worktree-agent-a01260dbd21b6daea, one commit. cargo test -p rb-ui 193 pass; release workspace 580 pass; fmt, clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->

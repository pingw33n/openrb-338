---
id: TASK-143
title: Save always writes the engine's tracks, as the original does
status: Done
assignee:
  - '@agent-worker'
created_date: 2026-10-29 14:27
updated_date: '2026-11-10T13:24:00Z'
labels:
  - ui
  - engine
dependencies:
  - TASK-080
references:
  - crates/orb-ui/src/document.rs
  - crates/orb-player/src/lib.rs
  - crates/orb-engine/src/seq.rs
  - docs/ui_spec.md
  - docs/song_load.md
priority: medium
type: enhancement
ordinal: 162000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on 2026-10-29 the user changed task-080's decision from option A to option B. OpenRB-338's Save must always write the engine's tracks, as the original's save does: the exe's TRAK writer sub_437CFF reads each track back from the DLL with song msg 22 {track, from, from + 0x504} (0x437DE6) and writes pos = (tick << 3) / 0xC0 as deltas (docs/song_load.md 12.6). Today (task-057's rule), Save writes the file's own track events unless recording changed them (Link::recorded_tracks), so an unedited save is the file byte for byte. The user wants the original's behavior instead.

What changes for a saved song, every one of them from the engine's loaded state:
- Events the loader dropped are gone: ticks at or above 768000, or not a multiple of 24 (exe sub_42FD60), and ticks above 767231 (DLL 0x1001477B). An invalid event does not advance the running position, so later events land where the original put them (load.rs). 35 songs of the user's third-party collection carry events past bar 999, the far end at bar 15145.
- The same-tick, same-controller dedup of the insert (the later value wins), and the stop prune (sub_100149F0, seq.rs Track::prune: an event repeating its controller's previous value is removed unless it is at tick 0), as far as a stop has run before the save.
- The song constructor's tick-0 events of every controller are present (section 4.9).
- The DLL's order within a tick.

What:
- crates/orb-ui/src/document.rs compose and its callers write the TRAK chunks from the engine's tracks (orb_player::Player::tracks, published after every load, merge and stop) for every save, not only after recording. Check when the tracks are taken: the original's save reads the DLL's state at the moment of saving, which may be before or after a prune. Match it; do not run an extra prune.
- A 1.x song's save already goes through the engine (task-125); make the two paths one.
- Remove or rework the byte-identity test of an unedited save (saving_an_unedited_corpus_song_gives_back_the_file in crates/orb-ui/tests/menu.rs). The new test: an unedited save of a song equals what the original would write for the same engine state, for example a song with an event past bar 999 loses it, a song with a repeated value after a stop loses that event, and the tick-0 constructor events appear. If the corpus has songs saved by the original ReBirth, add a check that loading and saving one gives back the file.
- docs/ui_spec.md (the Save sections) and the document.rs crate docs: state the new rule, with task-080's change of decision.
- Out of scope: anything that does not change what Save writes.

Not in a milestone at the user's request (2026-10-29); it is scheduled by the user.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every Save writes the TRAK chunks from the engine's tracks (dropped events gone, insert dedup, prune as of the save, tick-0 constructor events, the DLL's order); a 1.x and a 4.x song share the path
- [x] #2 Tests cover a song with an event past bar 999, a repeated value after a stop, and the tick-0 events; the unedited-save byte-identity test is replaced; docs/ui_spec.md and document.rs state the rule; fmt, clippy, cargo test --release pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented by an outside contributor (ghstcoder47) in PR #4 (github.com/pingw33n/openrb-338/pull/4), branch fix/save-engine-tracks: commits 8f34804 (the change) and 5888ece (review round 1). Merged by the user's supervisor session as one squashed commit, with the follow-ups below.

Review round 1 (2026-11-10, the user's review on the PR, 7 inline comments): all addressed in 5888ece.
- link.rs queue_song: the Stop-before-Load invariant stated, debug_assert!(!snapshot.pending) (debug, not assert: a release panic would lose unsaved work; the tests run debug builds). Checked: every UI load goes through queue_song (Stop then Load, FIFO flush), Stop clears the play flag at once (seq.rs Sequencer::stop -> halt), so activate always takes make_current (generation += 1) and tracks_from = max(tracks_from, generation) + 1 holds.
- A stop's prune changes what Save writes but does not mark the song modified (by design: Track::prune, sub_100149F0, only removes events repeating their controller's previous value, not at tick 0); documented in link.rs refresh_tracks and docs/ui_spec.md; tested with and without the UI following the callback.
- tests/menu.rs: the catch-all *x != y of the older-song check restored, with tracks and the derived batches excluded.
- The dead 1.x adopt-time set_engine_tracks removed (menu.rs adopt_as); document.rs source_of_1x and docs/song_load.md 12.6 updated.
- Link::recorded is derived from loaded (recorded_at), not a second copy.
- The test helper uses document::set_engine_tracks; the corpus loop calls it.
- switch_values.rs doc fixed; the device-byte tests renamed.

Merge follow-ups (in the squashed commit):
- File > Info's Last event row took the file's own tracks unless the song was recorded, so it disagreed with Save (e.g. 1001 for a song whose event past bar 999 the load drops, 3 after saving it). App::song_info now passes Link::loaded_tracks always (info.rs SongInfo::of, docs/ui_spec.md 18). New test tests/menu.rs the_last_event_row_is_the_engine_tracks_before_and_after_save (fails without the fix: left Some(1001), right Some(3)).
- The task's optional check (an original-saved corpus song saves back byte for byte) was not added.

Tests at 5888ece: cargo test --release --workspace with ORB_CD_ISO, ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods, ORB_THIRDPARTY_SONGS_DIR=~/rb-songs/songs: 679 passed, 0 failed, 3 ignored (thirdparty: 320 mods load; 1268 songs render equal to rbhost); debug cargo test -p orb-ui 258 passed. With the Info fix: cargo +nightly fmt --check, clippy -D warnings clean; cargo test -p orb-ui debug 259 passed, release 258 passed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Save writes the engine's tracks for every song (4.x, 1.x, new), as the original's TRAK writer sub_437CFF reads them back from the DLL: dropped events gone, insert dedup, the stop prune as of the save (no extra prune; a prune does not mark the song modified), tick-0 constructor events, the DLL's order. 1.x and 4.x share compose's path. The unedited-save byte-identity test is replaced by an engine-tracks check; File > Info's Last event row now follows the engine's tracks too. Docs: ui_spec.md 16 and 18, document.rs, song_load.md 12.6, rbm.md.

From PR #4 by ghstcoder47 (8f34804, 5888ece) plus the merge follow-up (Info). Tests: full release workspace with the CD image and the third-party collection 679 passed / 3 ignored; orb-ui debug 259, release 258; fmt and clippy clean.

Not done: the optional byte-for-byte check of an original-saved corpus song.
<!-- SECTION:FINAL_SUMMARY:END -->

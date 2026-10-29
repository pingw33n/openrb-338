---
id: TASK-143
title: 'Save always writes the engine''s tracks, as the original does'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-29 14:27'
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
- [ ] #1 Every Save writes the TRAK chunks from the engine's tracks (dropped events gone, insert dedup, prune as of the save, tick-0 constructor events, the DLL's order); a 1.x and a 4.x song share the path
- [ ] #2 Tests cover a song with an event past bar 999, a repeated value after a stop, and the tick-0 events; the unedited-save byte-identity test is replaced; docs/ui_spec.md and document.rs state the rule; fmt, clippy, cargo test --release pass
<!-- AC:END -->

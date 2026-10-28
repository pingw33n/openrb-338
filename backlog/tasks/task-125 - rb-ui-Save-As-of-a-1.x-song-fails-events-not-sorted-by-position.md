---
id: TASK-125
title: 'rb-ui: Save As of a 1.x song fails: events not sorted by position'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-28 15:26'
updated_date: '2026-10-28 15:42'
labels:
  - ui
  - re
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/document.rs
  - crates/rb-engine/src/load/rb1x.rs
  - crates/rbs/src/lib.rs
  - docs/song_load.md
priority: high
type: bug
ordinal: 144000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: reported by the user (2026-10-28). Steps:
1. Open ~/rb_stuff/songs/Razor Topz.RBS (a ReBirth 1.x MIDI-file song, file version 3.1, from the user's third-party collection; read only, never commit it).
2. Change the mod to Pitch Black Edition 2.0 (Mods menu).
3. File > Save As.
The status line shows: 'cannot write the song: chunk TRAK at offset 0: events not sorted by position'.

Cause found by the supervisor (to confirm):
- The 1.x import (crates/rb-engine/src/load/rb1x.rs, around 0x430CFA-0x430FEF) appends each device's events per MIDI track in MIDI-track order. Several MIDI tracks feed the same device, for example track 5's CC 17/31 go to the mixer, and the 909 off goes at tick 0. So an engine track (engine.tracks[t]) is not sorted by tick.
- rb-ui's document::source_of_1x / set_tracks (crates/rb-ui/src/document.rs:70-90) copies those lists as they are into the rbs::Song. rbs Track::build (crates/rbs/src/lib.rs:838-851) writes TRAK deltas and refuses a position lower than the one before it.

What to find out, in the disassembly, with addresses:
- What the DLL does with the msg-37 events of an unsorted batch list. Does it insert sorted, keep the order, or something else? Does rb's engine store and play them the same way? If not, it is an engine fidelity question; check the 1.x renders and document it.
- What the original's Save of such a song writes: the TRAK writer sub_437CFF, which reads the tracks back from the DLL (song msg 22, 0x437DE6) and writes 8*tick/192 deltas (0x437E9E). Are its deltas from sorted tracks, or does it write wrapping u32 deltas? rb should write what the original writes when that is a valid file; otherwise write the sorted order and record the difference in docs/original_bugs.md if it is a defect.
- Whether a Save without the mod change also fails, and why the mod change matters (compose(), the 'current' version path).

Fix:
- Save of a 1.x song (and of any document whose tracks come from the engine, including recorded_tracks) writes a valid file in the original's event order for equal ticks.
- A stable sort by tick is the likely answer, but the order of same-tick events must be the one the original's song holds.
- Re-opening the saved file must give the same engine tracks as the 1.x import, apart from the order the original also changes.

Coordinate with task-123 (File > Info facts of a 1.x song), which also touches the 1.x path in rb-ui; whichever lands second rebases.

Tests:
- A 1.x song built in the test (as crates/rb-cli/tests/info.rs does), with events of two MIDI tracks on one device out of tick order. Save As must succeed, and the reloaded song must render the same as the original import (rb render, or the engine message stream).
- A gated test on RB_THIRDPARTY_DIR: every 1.x song of songs/ composes and writes (rbs build Ok).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Save As of a 1.x song with interleaved device events (Razor Topz.RBS with any mod) writes a valid .rbs; reopening it gives the same tracks (order of equal ticks as the original's song holds), documented with addresses
- [x] #2 The DLL's handling of unsorted msg-37 events and the original's TRAK writer behavior for such a song are documented in docs/song_load.md (and docs/original_bugs.md if a defect); the engine's storage matches the DLL's or a follow-up task exists
- [x] #3 Tests: a synthetic 1.x song saves and reloads equal; a gated RB_THIRDPARTY_DIR check writes every 1.x song; cargo +nightly fmt --check, clippy, cargo test (debug rb-ui and release workspace) and the corpus (175/175) pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Confirm in the disassembly: the DLL insert sub_10014750 (sorted, before the first node of equal tick, same tick+ctrl replaces), the msg-22 reply sub_10020EC0 (list order), the TRAK writer sub_437CFF (delta = 8*tick/192 - previous).
2. rb-ui: Link::load keeps the tracks of the engine it built (what msg 22 would read back right after the load). App::adopt_as gives a 1.x song's document those tracks; source_of_1x no longer copies the import's unsorted per-device lists.
3. Tests: a synthetic 1.x song with two MIDI tracks feeding the mixer out of tick order: Save As succeeds, the reopened song's engine tracks equal the import's apart from the same-tick reversal the DLL insert makes, and both render the same. A gated RB_THIRDPARTY_DIR test: every 1.x song of songs/ composes and writes.
4. docs/song_load.md 12.x: the unsorted batches, the save of a 1.x song, why the mod change does not matter. original_bugs.md only if a defect.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-aefd3b75ae7151b4b

IDA (read-only copies in the scratchpad, closed save=false), 2026-10-28:
- DLL sub_10014750: tick quantised (sub_100146E4), dropped if > 0xBB4FF (0x1001477B); walk 0x10014794..0x100147A2 'cmp eax,[next+4]; jg' advances while the new tick is greater, so it stops before the first node whose tick is >= the new one; 0x100147B4..0x100147E3 scans the nodes of equal tick for the same controller (+8) and stores the value (+9, 0x10014820); else a node from sub_10014434 (0x1001480F) is linked in there (0x10014818..0x1001481C). So the batch order does not matter for the tick order: every list is sorted by tick at all times; among equal ticks a new controller goes in front of the ones already there (a later insert comes first), and a repeat of (tick, ctrl) replaces the value in place (last write wins). rb's seq::insert_into models exactly this (partition_point(t > e.tick), replace on tick+ctrl).
- DLL sub_10020EC0 (msg 22): from..to clamped to the count (sub_10014670), node walk ebp = [ebp] (0x10020FB3), tick [node+4] (sub_1000DEE0), ctrl +8, value +9: the list order.
- Exe sub_437CFF: batches of 0x504 (msg 22 at 0x437DE6) until a reply shorter than 1284 (0x437EF4); per event pos = (tick << 3) / 0xC0 signed idiv (0x437E93..0x437E9E), delta = pos - previous pos (var_20, 0x437EA4), VLQ sub_4A10CC; the event count var_18 is written after (0x437F25..0x437F49, back-patched at the saved stream position). As the lists are sorted the deltas are never negative: the original saves a valid file with the DLL's order.
- The rb-ui failure is rb's: source_of_1x copied load_1x's per-buffer lists (import order: buffer 4 gets the 909 off at tick 0 first, then MIDI tracks 2..5 each append to the mixer buffer), and rbs Track::build refuses a decreasing position. compose() does not depend on the mod, so the mod change in the report is incidental (checked by test below).

Fix (rb-ui): Link::load keeps the built engine's track lists (Link::loaded_tracks); App::adopt_as gives a 1.x song's document those tracks (document::set_engine_tracks, shared with the recorded-tracks path of compose); source_of_1x no longer copies load_1x's buffers. No engine change.
Not a defect of the original: its save writes the DLL's sorted lists. Not added to original_bugs.md.
Reopen order: an RBS load inserts in file order, so each tick's events come out reversed, except replacements; every 1.x tick-0 event replaces a constructor node (checked on all 131 files of the collection), so a reopen gives the import's lists with each tick>0 group reversed, as the original does.
Tests (2026-10-28):
- crates/rb-ui/tests/menu.rs a_1x_song_with_unsorted_imports_saves_and_reopens_the_same: the import's mixer ticks are unsorted (asserted); Save As writes the engine's lists exactly; reopened lists = reopened_order(lists); 60 engine calls of play equal and audible.
- every_1x_song_of_the_collection_saves (RB_THIRDPARTY_DIR=~/rb_stuff, RB_MOD_DIR=recrypted mods): 131 1.x files (129 distinct) save and reopen with the import's lists in that order; Razor Topz.RBS plays equal for 200 calls after the save; saves after the change to PBE2 (Pitch Black Edition 2.0).
- cargo +nightly fmt --all --check clean; clippy -D warnings clean; cargo test -p rb-ui (debug) all pass; cargo test --release -q 553 pass, 0 fail; rb-cli thirdparty (gated) 2 pass; corpus compare 175 of 175 equal.
Test machine not used: the original's Save is read from the disassembly ([G] in song_load.md 12.6 that it was not run).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Save As of a ReBirth 1.x song now writes a valid .rbs. The cause was rb-ui copying load_1x's per-device buffers into the document. Those are in MIDI-track order, not tick order: MIDI tracks 2..5 all feed the mixer. rbs Track::build refused them. The mod change in the report was incidental.

The original (disassembly, docs/song_load.md 12.6):
- The DLL's msg-37 insert sub_10014750 keeps every list sorted by tick (walk 0x10014794..0x100147A2). Within a tick a new controller goes first, and a repeated tick and controller replaces the value (0x10014820). rb's seq::insert_into already models this, so the engine is unchanged.
- The TRAK writer sub_437CFF reads the lists back with msg 22 (sub_10020EC0, list order) and writes pos = (tick<<3)/0xC0 deltas (0x437E9E, 0x437EA4). No delta is ever negative, so the original's file is valid. This is not a defect.

rb: rb-ui gives a 1.x document the tracks of the engine it built (Link::loaded_tracks, document::set_engine_tracks, App::adopt_as). That is what the original saves: sorted, with the constructor's tick-0 events. A reopen gives the same lists with each tick>0 group reversed, which the original's load does too.

Branch worktree-agent-aefd3b75ae7151b4b, one commit.

Tests:
- A synthetic unsorted 1.x song saves, reopens and plays the same.
- Gated on RB_THIRDPARTY_DIR: 131 files (129 distinct) save and reopen. Razor Topz plays the same after a save and saves after the change to PBE2.
- fmt and clippy are clean. rb-ui debug tests pass; the release workspace has 553 passing; thirdparty passes.
- The corpus is 175 of 175 equal.
<!-- SECTION:FINAL_SUMMARY:END -->

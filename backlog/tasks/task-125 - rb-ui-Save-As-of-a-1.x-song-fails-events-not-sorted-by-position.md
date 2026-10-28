---
id: TASK-125
title: 'rb-ui: Save As of a 1.x song fails: events not sorted by position'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-28 15:26'
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
- [ ] #1 Save As of a 1.x song with interleaved device events (Razor Topz.RBS with any mod) writes a valid .rbs; reopening it gives the same tracks (order of equal ticks as the original's song holds), documented with addresses
- [ ] #2 The DLL's handling of unsorted msg-37 events and the original's TRAK writer behavior for such a song are documented in docs/song_load.md (and docs/original_bugs.md if a defect); the engine's storage matches the DLL's or a follow-up task exists
- [ ] #3 Tests: a synthetic 1.x song saves and reloads equal; a gated RB_THIRDPARTY_DIR check writes every 1.x song; cargo +nightly fmt --check, clippy, cargo test (debug rb-ui and release workspace) and the corpus (175/175) pass
<!-- AC:END -->

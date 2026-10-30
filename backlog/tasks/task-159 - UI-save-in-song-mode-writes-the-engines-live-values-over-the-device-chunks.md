---
id: TASK-159
title: UI save in song mode writes the engine's live values over the device chunks
status: To Do
assignee: []
created_date: '2026-10-30 10:30'
labels:
  - ui
  - re
milestone: m-5
dependencies:
  - TASK-156
references:
  - crates/orb-ui/src/document.rs
  - crates/orb-ui/src/link.rs
  - docs/song_load.md
priority: medium
type: bug
ordinal: 176000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: found in task-156 on 2026-10-30. In song mode the panel follows the engine's live device bytes: Link::follow in link.rs calls panel.adopt for every controller from the snapshot. document::compose then writes each panel value into the device chunk whenever it differs from what the load made of the chunk. So a song-mode song saved without an edit, after at least one UI frame, gets its device chunks (the pattern-mode parameters) overwritten by the engine's live values, which come from the tracks' tick-0 events.

Reproduction:
- Take tests/songs/diag/songs/303_accent0.rbs (song mode) and set tb303[0].params[3] (cutoff) = 10 and params[8] (waveform) = 1, while its track 1 keeps the tick-0 events cutoff 50 and waveform 0.
- Open it in App (a Link with the built-in kit), render 4 calls, draw 4 frames, then call App::song_bytes.
- The saved chunk has cutoff 50 and waveform 0.

The same happens to out-of-range switch bytes whose tick-0 events the load drops by its range check: waveform 2 becomes 0, 808 selector 255 becomes 0, PCF mode 2 becomes 1, delay mode 127 becomes 1. The corpus round-trip test (menu.rs saving_an_unedited_corpus_song_gives_back_the_file) draws no frame, so it misses this.

The original saves what the DLL's device blocks hold (sub_43920E and its siblings, docs/song_load.md 4.10): the pattern-mode parameters, not the live values. That is [G]: check which block the DLL returns in song mode.

What:
1. Find what the original's save writes for a song-mode song: the DLL's data block in song mode, either the pattern-mode parameters or the live bytes.
2. Make compose take the device chunk from the right source in song mode, so that an unedited save writes the file that was read. One way is to keep the panel's pattern-mode values apart from the live values it shows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The original's device block in song mode is identified with addresses and recorded in docs/song_load.md
- [ ] #2 A song-mode song opened, drawn for several frames and saved without an edit writes the file read byte for byte: a test, including the task-156 song with out-of-range switches in crates/orb-ui/tests/switch_values.rs, whose song-mode case then saves after frames
- [ ] #3 fmt, clippy and cargo test pass
<!-- AC:END -->

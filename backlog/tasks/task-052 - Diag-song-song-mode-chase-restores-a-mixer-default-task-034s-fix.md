---
id: TASK-052
title: 'Diag song: song-mode chase restores a mixer default (task-034''s fix)'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-27 04:14'
labels:
  - tests
  - engine
milestone: m-1
dependencies:
  - TASK-034
references:
  - docs/song_load.md
  - tests/songs/diag/README.md
priority: low
type: task
ordinal: 29700
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From task-034's review (2026-10-27). task-034 fixed MixerDevice::init_track: the song constructor's slot 8 (0x10011985 → mixer slot 8 at 0x100160F0) inserts a tick-0 event for every automatable mixer controller (1, 2, 6..9 + 6c), so in song mode a chase back to the loop start (loop wrap, locate, song end) restores the constructor default for a controller that has no tick-0 event in the file. No corpus song has that shape: all 163 corpus .rbs files have a tick-0 event for every automatable mixer controller, so the golden test cannot see the fix (it is 164/164 with and without it).

Work: add one diag song to tests/songs/diag/ (spec in gen_specs.py, built by build.py, README row) with: song mode, loop on, loop start 0, a mixer level (controller 6, channel 0) with no tick-0 event and its first event at a later bar setting a non-default value, a 303 or drum playing on that channel so the level is audible, and at least one loop wrap after the event. Remove the tick-0 event for that controller in the built song (the builder may add one by default; say how). Capture its reference with rbhost at --idle-blocks 9088 (tools/rbcorpus.py capture for that one entry; one redc-win-run permit; check `locky list`), add it to tests/refs (FLAC + manifest entry; do not touch other entries), and show that the golden test passes with the task-034 fix and fails without it (revert init_track in a scratch worktree). No app capture is needed (diag references are rbhost renders, decision task-013).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A new diag song with the described shape is built from a spec and listed in the diag README
- [ ] #2 Its rbhost reference (idle 9088) is in tests/refs with a manifest entry; no other entry changes
- [ ] #3 The golden test passes with the fix and fails without it (shown in the notes)
<!-- AC:END -->

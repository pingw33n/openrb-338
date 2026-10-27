---
id: TASK-082
title: 'Recording: one shared node pool per song instead of 100000 nodes per list'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-27 16:44'
labels:
  - engine
milestone: m-3
dependencies:
  - TASK-057
references:
  - crates/rb-engine/src/seq.rs
priority: low
type: enhancement
ordinal: 54000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found by the task-057 review (2026-10-27): crates/rb-engine/src/seq.rs:184 and :188 reserve POOL_NODES (100000, the DLL's shared node pool, 0x186A0 at 0x1001431B) of capacity for each event list so the audio callback never grows a Vec. An Event is 8 bytes, so a track's two lists reserve 1.6 MB and a song's 9 tracks 14.4 MB per Engine (seq.rs:426); rb-player's Tracks copy (lib.rs:307, :309-311) reserves 9 × 800 KB = 7.2 MB per copy, ×3 for its triple buffer ≈ 21.6 MB per Player. Mostly untouched virtual memory, but every Engine the tests, the player and the CLI build pays for it, and the DLL's limit is one pool shared by all lists of a song, not 100000 per list.

Do: model the pool as the DLL does — one arena per song of POOL_NODES nodes shared by all lists (sub_10014434 refuses a node when one or none is free), allocated once when the song is built (off the audio thread), with the lists as indices into it; or keep Vecs but reserve the shared budget once and size each list's capacity on demand off the thread — choose and say why. Keep the callback allocation-free (no_alloc covers recording), keep the recording sessions' byte-identity with the DLL (crates/rb-player/tests/live_messages.rs) and the golden test; measure the reserved bytes per Engine and per Player before and after, and the release test runtime.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The event lists of a song share one pool of POOL_NODES nodes (or an equivalent shared budget), refusing as sub_10014434 does; reserved memory per Engine and per Player is measured before and after in the notes
- [ ] #2 The callback stays allocation-free (no_alloc), the recording sessions still equal the DLL byte for byte, and the golden test and compare (164/164) pass
<!-- AC:END -->

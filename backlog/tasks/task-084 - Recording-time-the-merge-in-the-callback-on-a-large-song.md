---
id: TASK-084
title: 'Recording: time the merge in the callback on a large song'
status: To Do
assignee: []
created_date: '2026-10-27 17:20'
labels:
  - engine
  - tests
milestone: m-3
dependencies:
  - TASK-082
references:
  - crates/rb-engine/src/seq.rs
  - crates/rb-player/tests/no_alloc.rs
priority: low
type: task
ordinal: 105000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From the task-082 review (2026-10-27). Task-082 put a song's 18 event lists back to back in one buffer (seq::Pool, crates/rb-engine/src/seq.rs). An insert into a track list (list k < 9) shifts every node after it, i.e. the rest of that list, the later tracks' lists and the recording lists (Vec::insert's memmove within the reserved capacity). The merge (Track::merge, Sequencer::merge_all, sub_1001F520) runs in the audio callback on record off, the loop wrap and the song end, and inserts every recorded node up to the merge tick plus one return node per marked controller. Estimate: ~2-4 us per insert on a realistic song, 40-80 us worst case (a full pool, 800 KB moved), so ~1000 recorded nodes on a ~10000-node song may cost milliseconds against a 1.45 ms block (64 frames at 44.1 kHz).

Do: write a release benchmark or timing test (a song of ~10000 events, ~1000 recorded nodes across tracks, then set_record(0)) that measures merge_all; report the time per merge and per insert. If it can miss the block deadline, make the merge cheaper without changing its result (e.g. build each track's merged list in one pass into the gap, or order the work so the tail moves once per track), keeping the callback allocation-free and the result byte-identical.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The merge time for ~1000 recorded nodes on a ~10000-node song (and on a nearly full pool) is measured in release and recorded in the notes
- [ ] #2 If a merge can exceed a block (1.45 ms), it is made cheaper with identical results: seq unit tests, no_alloc, live_messages recording sessions, golden and compare 164/164 pass
<!-- AC:END -->

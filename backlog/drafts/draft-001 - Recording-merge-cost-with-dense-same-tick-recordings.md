---
id: DRAFT-001
title: 'Recording: merge cost with dense same-tick recordings'
status: Draft
assignee: []
created_date: '2026-10-27 20:54'
updated_date: '2026-10-29 08:57'
labels:
  - engine
dependencies:
  - TASK-084
references:
  - crates/orb-engine/src/seq.rs
  - crates/orb-engine/src/seq/merge_tests.rs
priority: low
type: enhancement
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From task-084. The one-pass merge (Track::merge / add / find / flush in crates/orb-engine/src/seq.rs) takes 0.13 ms for ~1000 recorded nodes on a ~10000-node song and 0.34 ms on a nearly full pool (M1 Pro, release), but its stress case in merge_tests.rs::merge_timing (20000 nodes recorded on the mixer track over 8 bars, 80 controllers at every 24-tick step, so 80 nodes per tick) takes ~1.5 ms, about one 64-frame block (1.45 ms). The time goes to the per-node work, not the pool moves (a 16x larger MERGE_CHUNK changed nothing): Track::find scans the tick's nodes in the buffer and the list for the controller, and the insert into MergeBuf moves the tick's nodes and the returns. Such density is not reachable from the panel today (1250 moves per second on one device); revisit if MIDI or automation input can record that much. Possible fix: a per-tick controller mask for the buffer's newest tick group, or keeping the returns apart from the recorded nodes; the result must stay equal to merge_reference (the_merge_matches_the_reference, a_merge_of_several_chunks_matches_the_reference).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The stress case of merge_timing runs well under 1.45 ms in release, recorded in the notes
- [ ] #2 The merge equivalence tests, no_alloc, live_messages, golden and compare 164/164 pass
<!-- AC:END -->

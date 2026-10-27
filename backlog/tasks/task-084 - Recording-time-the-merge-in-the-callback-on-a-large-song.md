---
id: TASK-084
title: 'Recording: time the merge in the callback on a large song'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 17:20'
updated_date: '2026-10-27 20:54'
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
- [x] #1 The merge time for ~1000 recorded nodes on a ~10000-node song (and on a nearly full pool) is measured in release and recorded in the notes
- [x] #2 If a merge can exceed a block (1.45 ms), it is made cheaper with identical results: seq unit tests, no_alloc, live_messages recording sessions, golden and compare 164/164 pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Timing test (#[ignore], release) in crates/rb-engine/src/seq/tests.rs: a 9-track song of ~10000 events, ~1000 recorded nodes across the tracks through record_hook while the position moves, then merge_all; the same on a nearly full pool (~98000 nodes). Report us per merge and per inserted node, with the current Track::merge.
2. If a merge can exceed 1.45 ms: rewrite Track::merge so every track moves the pool's tail a bounded number of times: the removal walk compacts in one pass (one drain), the returns and the recorded inserts are emulated into a preallocated sorted scratch (key tick asc, arrival desc, the original's replace/drop/free rules exactly), then flushed into the track list with one tail move and a backward merge; the scratch is fixed-size (chunked flush when full), allocated with the Sequencer, so the callback stays allocation-free.
3. Keep the old merge as a test-only reference and add a randomized equivalence test (lists, next, free, hold) over random songs, recordings, quantisations, loop-wrap ticks and a nearly full pool.
4. Measure after; seq tests, no_alloc, live_messages, golden, compare 164/164, full release tests, clippy, fmt.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a0326ccd5ed7a0aa1 (worktree .claude/worktrees/agent-a0326ccd5ed7a0aa1), on main 6054777.

Before (Apple M1 Pro, macOS 24.6, release; cargo test --release -p rb-engine --lib merge_timing -- --ignored --nocapture; 21 runs on clones, median), the insert-by-insert merge of task-057/082:
- ~10000-event song (9927 events over 64 bars, 48 controllers, nine tracks), 998 nodes recorded over bars 0-8, merge_all(6143): 1778 us (best 1735), 1.78 us per recorded node. Exceeds the 1.45 ms block.
- nearly full pool (97525 events over 999 bars, free 1484), 991 recorded: 17331 us, 17.5 us per recorded node (each insert memmoves the pool after its list, up to ~780 KB).
The merge can miss the block deadline, so AC #2 applies.

Change (crates/rb-engine/src/seq.rs): Track::merge keeps the DLL's insert-by-insert result but moves the pool tail a few times per track: (1) the walk compacts the kept nodes in place and frees the removed ones with one drain (the applied index drops for each removed node whose pre-walk index is below it; equivalent to the per-remove rule, shown in the notes of the code); (2) the returns and the recorded nodes go through Track::add, which applies sub_10014750's rules (round_up of the quantisation, > 767231 dropped, a node of the same tick and controller takes the value, first in the held buffer then in the list, as the buffer's nodes go before the list's of their tick; the free check counting the held nodes) into MergeBuf, a Vec of MERGE_CHUNK = 4096 nodes reserved with the Sequencer (sorted tick ascending, latest insert first in a tick); (3) Track::flush moves the tail once and merges the list and the buffer from the back; the applied index grows by the held nodes of tick <= the applied node's tick (sub_10014750's 'before the first node of a tick not below it'); a full buffer is flushed early. A track with an empty recording only clears its hold mask. The Sequencer gains a 32 KB buffer (rb-player/tests/reserved.rs updated: pool + MERGE_CHUNK*8).
Equivalence: crates/rb-engine/src/seq/merge_tests.rs keeps the old merge as merge_reference and checks Sequencer equality (pool node for node, ends, applied pointers, hold, counters) over 400 random cases (40 in debug: songs up to 3000 events, 3/8/40/128 controllers, 303 bar quantisation, playing/stopped recording, loop-wrap tick rule with odd loop starts, random merge tick and applied pointers, every 5th case the pool filled to 0..39 free) and over 8 cases of ~12000 recorded moves on two tracks (several chunks, the pool running out before/within/after the second chunk). Both pass.
Failed hypothesis: the stress case's remaining 1.5 ms is not the tail moves (MERGE_CHUNK 65536 in a temporary experiment gave the same 1.64-1.70 ms); it is the per-node tick-group scan and buffer insert at 80 nodes per tick. Follow-up: DRAFT-001.
After (same machine and command, median of 21):
- ~10000-event song, 998 recorded: 1778 us -> 129 us (0.13 us per recorded node), 9% of a 1.45 ms block.
- nearly full pool (free 1484), 991 recorded: 17331 us -> 342 us (0.35 us per recorded node).
- stress (69770 events, 20000 recorded on the mixer track, 80 per tick): 147.7 ms -> 1.52 ms.
Reproduce: cargo test --release -p rb-engine --lib merge_timing -- --ignored --nocapture
Tests: cargo test --release (RB_STANDARD_MOD, RB_MOD_DIR set): 446 passed, 0 failed, 2 ignored; rb-player no_alloc, live_messages (the two recording sessions vs the DLL captures), reserved pass; corpus_golden passes; rb compare --set diag,app,shipped: 164/164 EQUAL, exit 0; debug cargo test -p rb-engine --lib seq:: 1.6 s; clippy -D warnings and fmt clean. docs/initial_investigation.md 5.3 gets rb's merge and the numbers (no new fact about the original).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Measured and fixed. The insert-by-insert merge took 1.78 ms for ~1000 recorded nodes on a ~10000-node song and 17.3 ms on a nearly full pool (M1 Pro, release), above the 1.45 ms block. Track::merge now compacts the walk in one pass, holds the new nodes in a reserved 4096-node MergeBuf under sub_10014750's exact rules, and merges them into the list from the back with one tail move per chunk: 0.13 ms and 0.34 ms; a 20000-node stress case 148 ms -> 1.5 ms (follow-up DRAFT-001). Byte-identical: a randomized equivalence test against the kept reference merge (400 + 8 cases incl. full pools and multi-chunk merges); seq tests, no_alloc, live_messages, reserved, golden, compare 164/164, full release tests 446 pass. Timing: cargo test --release -p rb-engine --lib merge_timing -- --ignored --nocapture. Branch worktree-agent-a0326ccd5ed7a0aa1, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

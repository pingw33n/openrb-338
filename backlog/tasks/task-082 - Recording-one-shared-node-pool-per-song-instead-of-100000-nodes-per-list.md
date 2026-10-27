---
id: TASK-082
title: 'Recording: one shared node pool per song instead of 100000 nodes per list'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 16:44'
updated_date: '2026-10-27 17:20'
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
- [x] #1 The event lists of a song share one pool of POOL_NODES nodes (or an equivalent shared budget), refusing as sub_10014434 does; reserved memory per Engine and per Player is measured before and after in the notes
- [x] #2 The callback stays allocation-free (no_alloc), the recording sessions still equal the DLL byte for byte, and the golden test and compare (164/164) pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Verify in ReBirth.dll.i64 (headless copy) where the node pool is built and who takes nodes from it (per song or per process; song events and recording lists).
2. Add a byte-counting test (crates/rb-player/tests/reserved.rs) and measure the live bytes of Sequencer::default(), Tracks::default() and Player::new() on the current code.
3. Replace the 18 per-list Vecs (each reserving POOL_NODES) by one arena per song, seq::Pool: one Vec<Event> of capacity POOL_NODES holding the 18 lists back to back (tracks 0..8, then their recording lists 9..17), with the end of each list. A list stays a sorted contiguous slice, so the insert, merge, chase, advance and prune keep their code and order; an insert or a removal shifts the nodes after it inside the reserved capacity (no allocation). The free count is POOL_NODES minus the nodes held, as sub_10014434 sees it. Track keeps its metadata (device, applied index, hold, quant) and takes the pool as an argument; TrackMut (track + pool) is what a device's init_track gets.
   Why not linked nodes in an arena: the slices keep every algorithm and every caller (events() as a slice, partition_point) as they are, at the cost of a memmove per insert, which is of the order of the DLL's own list walk.
   Why not per-list Vecs sized on demand: the callback cannot grow a Vec, and sizing it from the UI thread races the recording.
4. rb-player's Tracks holds one Pool too (lists 0..8 copied in one extend), so the UI's t.events[i] still indexes a slice.
5. Re-measure; run cargo test (debug and release golden), clippy, no_alloc, live_messages, compare --manifest with the Standard kit (164/164, and 175/175 with RB_MOD_DIR). Update docs (§5.3 pool, seq.rs module doc).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a517918f64210aca9

IDA (headless copy of ida/ReBirth.dll.i64, closed save=false):
- The pool is per engine instance (process), not per song: EngineOpen 0x1001B0E0 -> sub_1001C7B0 -> sub_10021A10 (call at 0x1001C889) -> operator new(0x14) + sub_100142C0 (0x10021A51), stored in dword_10100230 (0x10021A58). Freed only by sub_1001EC00 (from sub_1001DCE4, the close). sub_10021A10 also builds dword_101245F0 (sub_100215F0: 32 more lists, sub_100144B0 each) whose users sub_10020440/sub_100208F0/sub_10020BC0 take from the same pool (not traced further).
- The song's track events and the recording lists draw from the same pool: sub_10014750 is the only insert (sub_10014434 at 0x1001480F, pool from dword_10100230 at 0x100147F6); the song ctor's slot 8 inserts go sub_1001F900 -> sub_10014750 (0x1001F967), msg 37 sub_10021010 -> sub_10014750 (0x100210C0), the record hook and the merge likewise. sub_1000DEE0 = mov eax,[ecx+4] (the free count).
- sub_10014434: cmp [pool+4],1; jle -> returns 0 (0x1001443A..0x10014468): refuses when one or none is free (as task-057 had it).
- New: msg 37 refuses a whole batch whose count (last-first, 0x10021042) is above the free count (cmp ebx,eax; ja 0x1002105B -> 0x100210ED: sub_1001DE00(22) instead of sub_1001DF80(73)). rb does not model it; follow-up task-083 (only differs for a song of ~100000 events).
Design chosen: seq::Pool, one Vec<Event> of capacity POOL_NODES per song holding the 18 lists back to back (tracks 0..8, recordings 9..17) with their ends; a list stays a contiguous sorted slice, so insert_into/merge/chase/advance/prune keep their logic and order; an insert/removal is a memmove inside the reserved capacity (no allocation). The free count is POOL_NODES - nodes held, which equals the old per-batch counters (the merge frees a removed node at once and the recording list after the inserts, as before). Track keeps device, list index, next, hold, quant; TrackMut (track + pool) is what SongDevice::init_track gets; Sequencer::events(i)/recorded(i)/track_mut(i). rb-player's Tracks.events is a Pool too (copy_tracks_from: one extend within capacity), Index<usize> keeps t.events[i] in rb-ui/src/link.rs unchanged. rb-ui changed only in tests/menu.rs (two reads of seq.tracks[i].events(), forced by Track no longer owning its list).
Rejected: linked nodes in an arena (the DLL's shape, but every caller and algorithm would change from slices); per-list Vecs sized on demand from the UI thread (the callback cannot grow a Vec, and resizing from the UI races the recording).
Measured (crates/rb-player/tests/reserved.rs, a thread-local live-bytes allocator; cargo test --release -p rb-player --test reserved -- --nocapture):
- before: Sequencer::default 14400000 B, Tracks::default 7200000 B, Player::new 36700380 B (Engine 14.4 MB + 3 x 7.2 MB + 0.7 MB rest)
- after: Sequencer::default 800000 B, Tracks::default 800000 B, Player::new 3900188 B (4 pools + 700188 B rest)
Release test runtime (scratchpad task-082/test.sh, cargo test --release --no-fail-fast after --no-run, RB_STANDARD_MOD + RB_MOD_DIR): before 38 s (428 pass + the new reserved test failing, 1 ignored), after 38 s (433 pass, 0 fail, 1 ignored). Debug cargo test: 429 pass, 5 ignored. clippy -D warnings clean.
Corpus: rb compare --manifest tests/refs/manifest.json --standard-mod target/Standard.rbm: 164 of 164 equal; with RB_MOD_DIR: 175 of 175 equal. Golden test 4/4, no_alloc 3/3, live_messages 8/8 (the recording sessions' pinned DLL hashes).

Review round 1 (send back, docs only): (1) documented the pending-song case in seq.rs's module doc and docs/initial_investigation.md section 5 (node pool, rb paragraph): rb-player's Audio holds current and pending, two Pools are live while a song waits for Play, where the DLL's one per-instance pool would serve both; it matters only when the two songs together exceed 100000 nodes. (2) Track::insert's doc notes the unchecked insert can grow the pool past POOL_NODES (song ctor and tests only, never the callback). (3) Filed task-084 (m-3, engine, low, unassigned, dep task-082): time merge_all in the callback on a large merge and fix it if it can miss the 1.45 ms block. No code change; cargo doc/clippy/fmt rerun.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The 18 event lists of a song now share one node pool, seq::Pool: one buffer of POOL_NODES (100000) nodes reserved when the song is built, the lists back to back in it, the free count POOL_NODES minus the nodes held, refused as sub_10014434 does (one or none free). Verified in the DLL that the pool is one per engine instance (EngineOpen -> sub_10021A10 -> sub_100142C0, dword_10100230) and that the song ctor's events, msg 37, the recording and the merge all take from it through sub_10014750; docs/initial_investigation.md section 5 updated. Found msg 37's whole-batch refusal (0x1002105B), not modelled: task-083.

Reserved memory: Sequencer 14.4 MB -> 0.8 MB per Engine; Tracks 7.2 MB -> 0.8 MB per copy; Player::new 36.7 MB -> 3.9 MB (crates/rb-player/tests/reserved.rs pins it). Release tests 38 s before and after.

Tests: cargo test --release 433 pass, 1 ignored; debug 429 pass, 5 ignored; clippy clean; corpus compare 164/164 and 175/175 with RB_MOD_DIR; golden, no_alloc and the live_messages recording sessions pass. Branch worktree-agent-a517918f64210aca9, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

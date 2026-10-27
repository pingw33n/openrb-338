---
id: TASK-073
title: 'Flaky rb-player live test: spin-wait bounded by iterations, not time'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 14:23'
updated_date: '2026-10-27 15:07'
labels:
  - tests
milestone: m-3
dependencies:
  - TASK-043
references:
  - crates/rb-player/tests/live.rs
priority: medium
type: bug
ordinal: 45500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found by the task-046 review (2026-10-27): crates/rb-player/tests/live.rs failed once at its line 140 with "the change never showed" during a full-workspace `cargo test --release`, and passed on rerun. The test (parameters sent from another thread while an audio thread renders full_mix) waits for each change with `wait`, which spins `for _ in 0..100_000 { if f(p.snapshot()) { return } std::thread::yield_now() }`: a bound in iterations, not in time. Under a loaded parallel test run the render thread can get too little CPU within 100,000 yields, so the wait gives up although the player is correct.

Fix: bound the wait by wall-clock time (e.g. up to 10 s with a short sleep or yield between polls), keep the failure message with the last snapshot; check the other spin-waits in crates/rb-player/tests (live.rs, live_messages.rs, no_alloc.rs) and crates/rb-ui/tests (wired.rs) for the same pattern. Show it is stable: run the test 50 times under load (e.g. `cargo test --release -p rb-player --test live -- --test-threads=1` in a loop while the workspace tests run in parallel), all pass.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The live tests' waits are bounded by wall-clock time, not iteration counts, with the last snapshot in the failure message
- [x] #2 The live test passes 50 runs in a row under load (the command and result in the notes); other spin-waits in rb-player and rb-ui tests are checked and fixed
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Bound live.rs wait by a 30 s wall-clock deadline. 2. Check other loops in rb-player/rb-ui tests. 3. 50 runs under load (20 busy loops).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a09a0ff5db6533286

With only the 30 s wall-clock deadline, 3 of 50 loaded runs still failed: the free-running audio thread, in song mode, replays the tracks' tick-0 controller state (mixer ch0 pan 40, device controllers such as tb303_1 cutoff ctrl 3) at each loop wrap (seq.rs:302-316), overwriting the commands between polls. Fix: the test sends SongMode(false) then Play (chase/replay only in song mode), asserts playing && !song_mode in every wait, and has a drop guard that stops the audio thread. Other loops in rb-player/rb-ui tests are fixed call counts on the synchronous harness; no other waits.

Reproduce: 20 busy loops + 50x cargo test --release -p rb-player --test live. Before (deadline only, song mode): 3/50 failed. After: 0/50.

Review round 1: subject shortened, pattern mode instead of stopped, drop guard, diagnosis corrected. Review round 2: the per-device wait predicate also asserts playing && !song_mode; 20-load 50 runs re-run (see below).

Round 2 result: 0 of 50 failed under 20 busy loops.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
live.rs wait bounded by a 30 s wall-clock deadline (last snapshot in the panic). The remaining flake was the song-mode loop wrap replaying tick-0 controller state over the commands; the test now plays in pattern mode (still asserting playing), with a drop guard stopping the audio thread. 50/50 under 20 busy loops; other tests have no spin-waits. Workspace cargo test --release passes.
<!-- SECTION:FINAL_SUMMARY:END -->

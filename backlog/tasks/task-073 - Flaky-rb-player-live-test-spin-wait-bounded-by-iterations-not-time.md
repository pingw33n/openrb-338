---
id: TASK-073
title: 'Flaky rb-player live test: spin-wait bounded by iterations, not time'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-27 14:23'
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
- [ ] #1 The live tests' waits are bounded by wall-clock time, not iteration counts, with the last snapshot in the failure message
- [ ] #2 The live test passes 50 runs in a row under load (the command and result in the notes); other spin-waits in rb-player and rb-ui tests are checked and fixed
<!-- AC:END -->

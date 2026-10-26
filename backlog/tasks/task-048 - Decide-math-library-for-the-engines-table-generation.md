---
id: TASK-048
title: 'Decide: math library for the engine''s table generation'
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-26 17:01'
updated_date: '2026-10-26 17:28'
labels:
  - decision
  - engine
milestone: m-1
dependencies: []
references:
  - crates/rb-engine/src/tables/mod.rs
  - docs/initial_investigation.md
priority: medium
type: spike
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Context (task-019). crates/rb-engine/src/tables generates every run-time table of the original (T, PAN, 303 pitch and FILT, PCF LP/BP, dist W, T808, T909, DT, the two cutoff smoothers) with std's f64 powf/exp/cos/sin, and every entry is bit-identical to the original's dump. std's functions call the platform's libm: the tests ran on macOS (the system libm). On Linux (glibc) or Windows (the MSVC CRT) the last ulp of pow/exp/cos/sin can differ. The f32 stores have a margin of 1358 f64 ulps to the nearest rounding midpoint, so most such differences vanish, but T808[64] = trunc(pow(s,64)*(256/s^64)) is exactly 256.0 only when powf returns the squared value bit for bit; one ulp less gives 255. The tables test would catch any difference on the platform it runs on.

Options:
1. Keep std (platform libm). No dependency; tables may differ on an untested platform, caught only by running cargo test there.
2. Use the pure-Rust libm crate (musl port, pinned, e.g. =0.2.16) for the four functions. Same results on every platform. Experiment in task-019: with libm 0.2.16 instead of std every table is still bit-identical to the dumps (all 11 tables tests pass).
3. Embed the dumped tables (about 440 KB) as data and drop the generators.

Recommendation: option 2, a small pinned pure-Rust dependency that makes the tables platform-independent and is already shown to match the original.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The decision is recorded in this task's final summary with the date
- [x] #2 If option 2 or 3 is chosen, a follow-up task applies it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Applied in task-019's commit (branch worktree-agent-a734adc1e6f8c6bea, review round 1): crates/rb-engine/Cargo.toml libm = "=0.2.16"; tables/mod.rs uses libm::{pow, exp, cos, sin}; all table tests pass in debug and --release, bit-identical to the dumps. No separate follow-up task was needed (AC 2).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Decided by the user 2026-10-26: option 2, libm =0.2.16; applied in task-019's commit
<!-- SECTION:FINAL_SUMMARY:END -->

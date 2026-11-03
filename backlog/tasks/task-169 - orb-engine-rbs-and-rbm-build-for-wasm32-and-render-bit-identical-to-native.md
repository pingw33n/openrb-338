---
id: TASK-169
title: 'orb-engine, rbs and rbm build for wasm32 and render bit-identical to native'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-11-03 09:16'
labels:
  - engine
  - tests
milestone: m-6
dependencies: []
references:
  - crates/orb-engine/Cargo.toml
  - crates/rbs
  - crates/rbm
  - tests/refs/manifest.json
  - .github/workflows/ci.yml
  - docs/building.md
priority: high
type: feature
ordinal: 185000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: m-6 puts OpenRB-338 in the browser (the user's goal of 2026-11-03: a web version on GitHub Pages that plays songs and mods from github.com/pingw33n/rb-songs and github.com/pingw33n/rb-mods or from local files). Everything above it (the web UI, the web audio output) needs the engine and the file-format crates on wasm32-unknown-unknown first, and the web version must sound like the native one: the same samples, sample for sample.

Context:
- orb-engine depends only on rbs and libm (pinned =0.2.16), so its transcendental math does not come from the platform; f64 arithmetic is IEEE on wasm too, and wasm has no FMA contraction. So bit identity is expected but unproven: check it, don't assume it. Watch for anything that differs on a 32-bit target (usize width, hashing, f32/f64 to int casts, `as` saturation is the same) and for the std float functions that on wasm32 come from compiler-builtins rather than the system libm (e.g. f64::sqrt is a wasm instruction and exact; powf/exp/sin through std are not the same code as on native: every such call must go through libm, as in native).
- rbm uses sha2 and toml; check both build for wasm32-unknown-unknown (no getrandom or C code pulled in).
- No std::fs, threads or time in these three crates' library code on wasm (tests may keep them behind cfg).

What to build:
- `cargo build --target wasm32-unknown-unknown -p orb-engine -p rbs -p rbm` succeeds (add the target to rust-toolchain.toml's targets).
- A check that wasm renders equal native: run the corpus renders (tests/refs/manifest.json; at least the diag and app sets with the built-in kit, and the shipped songs and mod songs when ORB_CD_ISO is set) compiled for wasm32-wasip1 under a wasm runtime (wasmtime as a cargo runner, or a small wasmtime-embedding test harness) and compare each render's SHA-256 with the native render's. Pick the simplest setup that runs in CI on Linux; document it in docs/building.md.
- A CI job (ci.yml) that builds the three crates for wasm32-unknown-unknown and runs the wasm-vs-native comparison on the songs that need no ORB_CD_ISO.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cargo build --target wasm32-unknown-unknown -p orb-engine -p rbs -p rbm succeeds with the pinned toolchain; the target is listed in rust-toolchain.toml
- [ ] #2 Every corpus song of the diag and app sets renders under wasm32 with the same SHA-256 as the native render (the command and the count in the notes); with ORB_CD_ISO set, the shipped and mod songs too
- [ ] #3 Any engine change made for wasm is checked against the reference corpus (corpus_golden passes natively)
- [ ] #4 ci.yml builds the three crates for wasm32-unknown-unknown and runs the wasm-vs-native comparison of the songs that need no external input
- [ ] #5 docs/building.md describes the wasm build and the comparison; any wasm-specific fact about the numerics goes into the engine docs
<!-- AC:END -->

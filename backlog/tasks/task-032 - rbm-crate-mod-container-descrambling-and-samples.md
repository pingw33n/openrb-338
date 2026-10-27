---
id: TASK-032
title: 'rbm crate: mod container, descrambling and samples'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-27 05:13'
labels:
  - rbm
  - engine
milestone: m-2
dependencies:
  - TASK-020
  - TASK-030
  - TASK-053
priority: high
type: feature
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the `rbm` crate from docs/rbm.md: parse the container, derive the key table, descramble the EMBF chunks, parse the AIFF samples, and map them by file name to the engine's 46 sample slots with per-sample fallback to the built-in kit. Tests: the corpus mods load and their samples equal the original's (the dumps from the RE task).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every corpus mod loads and its samples equal the original's, checked by cargo test
- [x] #2 A missing sample falls back to the built-in one (tested)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
New crates/rbm (lib + tests/rbm.rs) on rb-engine's samples: parse CAT/PRBM, key sums/pages, KeyTable::read from Read+Seek (magic check, two pages), descramble, load_samples by the 46 names with builtin fallback (SampleSet::from_shared added). Tests: synthetic scrambled mod over a virtual 128 MiB DAT, fuzz, real files vs tools/rbm.py extract.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-27: key material per task-053 (decided): the crate takes the key pages from a caller-supplied RB20FUL.DAT (a reader over the file: only the two 2048-byte pages a mod needs are read, at the offsets docs/rbm.md section 5 gives); nothing of RB20FUL.DAT is embedded or committed; tests read RB_RB20FUL and RB_RBM_DIR and skip with a message when unset. tools/rbm.py (task-030) is the reference implementation to test against.

Branch: worktree-agent-a44a3796022c7e143

Result: crates/rbm added. Found: Metallicon.rbm AIFFs have a 12-byte COMM chunk; the engine AIFF reader rejected them (COMM < 18 bytes). The DLL/rbm.py read the rate field through the chunk end, so aiff.rs now takes COMM from its start to the file end (documented in docs/rbm.md section 9). Tests: 8 rbm tests + doctest; with RB_RB20FUL/RB_RBM_DIR (the 4 shipped mods) 106 files each and all 46 samples equal tools/rbm.py extract; without the env vars the real test skips with a message. cargo test all pass, clippy -D warnings clean, fmt clean. Missing-sample fallback is tested synthetically (samples_map_by_name_with_builtin_fallback).

Review round 1 (send back) fixed: addresses and Metallicon COMM counts (19x12 bytes, 27x18, verified) in docs/rbm.md 9 and initial_investigation.md 7.4; reworded the short-COMM error; synthetic tests a_short_comm_chunk_reads_through_into_ssnd (accept 12-byte COMM before SSND, reject short COMM at file end); real test asserts no repeated file names and fails (not skips) when python cannot run. AC #1 reference is tools/rbm.py, which task-030 verified against the samples the DLL loaded (docs/rbm.md 8). cargo test --release pass with and without env (golden 164 passed), clippy and fmt clean.

Review round 2: the golden result is cargo test --release -p rb-engine --test corpus_golden, 4 passed (164 corpus entries: 145 diag, 11 app, 8 shipped); the earlier '164 passed' was the rb-engine unit-test count. The short-COMM-at-end test now puts SSND first so the short-COMM error is reached.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
crates/rbm: CAT/PRBM parse (warns on non-PRBM), key pages read from the caller's RB20FUL.DAT (2 pages, magic checked), descramble, INFO fields, files by name, load_samples onto the 46 slots with per-slot built-in fallback; rb-engine gained SampleSet::from_shared and a more lenient COMM read (Metallicon's 12-byte COMM). All four shipped mods equal tools/rbm.py (106 files + 46 samples each). cargo test pass (with and without env), clippy, fmt. Branch worktree-agent-a44a3796022c7e143, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

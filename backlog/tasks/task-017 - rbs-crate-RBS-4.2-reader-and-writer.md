---
id: TASK-017
title: 'rbs crate: RBS 4.2 reader and writer'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 16:24'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-015
  - TASK-014
references:
  - docs/RBS42.txt
priority: high
type: feature
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement the `rbs` crate: parse RBS 4.2 (docs/RBS42.txt) into a typed model (HEAD, GLOB, USRI, the nine DEVL chunks, the nine TRAK tracks with absolute event positions) and write it back. Strict errors with the chunk and offset for malformed input; unknown chunks preserved on write. Tests: every song of the reference corpus and the shipped songs round-trip byte-identical, and the model matches tools/rbs.py's JSON for them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All corpus and shipped songs round-trip byte-identical in cargo test
- [x] #2 The model equals tools/rbs.py json for each of them (a test compares them)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Mirror tools/rbs.py in crates/rbs: typed chunks with private raw copies for byte-exact write-back, unknown RB40 chunks preserved, errors with chunk+offset. Tests in crates/rbs/tests/corpus.rs: round trip and JSON equality against /usr/bin/python3 tools/rbs.py json on 18 shipped + 145 diag songs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ad65c08e6f482dd88

Equality test shells out to /usr/bin/python3 tools/rbs.py json per song (no fixtures committed); model is converted to serde_json::Value in the test (serde_json is a dev-dependency only). cargo test: 4 pass (15.9 s debug), clippy -D warnings clean. Decisions: strict sizes (HEAD 256, GLOB 512, USRI 712, ...); pad bytes must be zero; unknown chunks only tolerated directly under RB40 (DEVL/TRKL order is fixed, as in rbs.py). Python writes the version as 'M.m' from header bytes 5,6; the model keeps major/minor.

Review round 1 (2026-10-26): reader stays lenient (any RB40 item order, non-minimal deltas, empty TRAK, nonzero first delta, TRAK trailing bytes kept); writer canonical by design (HEAD GLOB USRI DEVL TRKL, minimal deltas), documented in the crate docs with the corrected field meanings. Params/Tb303/Drums build return Err on wrong lengths or != 32 patterns; error tags trimmed and escaped. Accepted deviation (supervisor decision 2026-10-26): unknown chunks inside DEVL/TRKL stay an error since RBS42 fixes their order. New unit tests in lib.rs (reorder, non-canonical delta+trailer, empty track, bad model shapes, tag text). cargo test 9 pass, clippy and fmt clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
crates/rbs implements the RBS reader/writer (Song::parse / Song::write, model mirroring tools/rbs.py, controller_name helper and parameter name tables). All 163 shipped and corpus songs round-trip byte-identical and the model equals rbs.py json. Review round 1 fixes applied. cargo test 9 pass; clippy clean. Branch worktree-agent-ad65c08e6f482dd88, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

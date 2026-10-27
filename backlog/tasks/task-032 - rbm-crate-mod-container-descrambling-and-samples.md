---
id: TASK-032
title: 'rbm crate: mod container, descrambling and samples'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-27 04:58'
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
- [ ] #1 Every corpus mod loads and its samples equal the original's, checked by cargo test
- [ ] #2 A missing sample falls back to the built-in one (tested)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-27: key material per task-053 (decided): the crate takes the key pages from a caller-supplied RB20FUL.DAT (a reader over the file: only the two 2048-byte pages a mod needs are read, at the offsets docs/rbm.md section 5 gives); nothing of RB20FUL.DAT is embedded or committed; tests read RB_RB20FUL and RB_RBM_DIR and skip with a message when unset. tools/rbm.py (task-030) is the reference implementation to test against.
<!-- SECTION:NOTES:END -->

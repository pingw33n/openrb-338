---
id: TASK-032
title: 'rbm crate: mod container, descrambling and samples'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
labels:
  - rbm
  - engine
milestone: m-2
dependencies:
  - TASK-030
  - TASK-020
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

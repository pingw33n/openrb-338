---
id: TASK-010
title: 'Experiment: determinism of the original''s export per capture protocol'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
labels:
  - tests
  - testmachine
milestone: m-0
dependencies:
  - TASK-004
  - TASK-007
  - TASK-008
  - TASK-009
priority: high
type: spike
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Measure how deterministic the original's export is under each capture protocol, using tools/rb_export.py and tools/rbcmp.py on a subset of the diagnostic songs (silence, a 303 note, an 808 snare pattern, a delay with feedback, a compressor song): fresh process per export; two exports in a row in one process; export after playing for some seconds; export after loading another song first. Repeat each at least three times. For every pair report equality or the first difference, and explain each difference with the state from docs/export_state.md (for example 303 phase, leftover buffer, snare counter). Conclude which protocol gives byte-identical output, or what residual variation remains and how large it is. Results (hashes, rbcmp output) in the notes and a summary section appended to docs/export_state.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each protocol was run at least three times on at least five songs, with the hashes and rbcmp results in the notes
- [ ] #2 Every observed difference is attributed to a named piece of engine state, or listed as unexplained
- [ ] #3 docs/export_state.md has a results section with the recommended protocol
<!-- AC:END -->

---
id: TASK-017
title: 'rbs crate: RBS 4.2 reader and writer'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
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
- [ ] #1 All corpus and shipped songs round-trip byte-identical in cargo test
- [ ] #2 The model equals tools/rbs.py json for each of them (a test compares them)
<!-- AC:END -->

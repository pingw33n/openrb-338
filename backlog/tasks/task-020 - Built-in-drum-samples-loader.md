---
id: TASK-020
title: Built-in drum samples loader
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 16:27'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-015
  - TASK-016
priority: medium
type: feature
ordinal: 23800
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Load the built-in drum samples as the decision on their source says: parse the PE resource directory of ReBirth.dll (type "AIFF", IDs 1000..1045, the ID-to-instrument map in docs/initial_investigation.md section 7.3/7.4), parse AIFF (COMM, SSND, 16-bit big-endian, channel 0 only, sample rate ignored), keep raw i16 with 512 zero samples of padding as CWaveSample does. Tests: all 46 samples load from original/ReBirth.dll with the expected frame counts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All 46 built-in samples load per the decision, with frame counts checked in cargo test
- [ ] #2 The AIFF parser rejects non-16-bit and missing-chunk files with clear errors (tested)
- [ ] #3 The rb CLI takes --rebirth-dll PATH (and a default location) to load the samples, and errors clearly when drums are needed but no ReBirth.dll is found
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26, per the user: runs just before task-024 (the first task that needs samples), after the CLI (task-028). This task also adds the sample source to the rb CLI per the task-016 decision: `--rebirth-dll PATH` (and a default location), with a clear error when a song needs drum samples and no ReBirth.dll is found; 303-only rendering keeps working without it.
<!-- SECTION:NOTES:END -->

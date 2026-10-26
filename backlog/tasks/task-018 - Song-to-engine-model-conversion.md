---
id: TASK-018
title: Song to engine model conversion
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-017
  - TASK-003
priority: high
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Convert the rbs model into the engine's initial state and event stream exactly as Rebirth.exe does at song load and export (docs/song_load.md from m-0): global parameters (tempo BPM*1000, loop start/end in ticks, shuffle, the sound model flag), the devices' internal pattern formats (303 98-byte patterns with the 6-byte steps, 808/909 194-byte patterns with their step values), the mixer and effect parameter bytes, and the track events at 192 PPQ. Unit tests against the mappings in docs/song_load.md, plus a test comparing the converted message stream with rbhost's log of the messages it sends for the same song (if rbhost can log them; otherwise against hand-checked values for three songs).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every mapping of docs/song_load.md is implemented and unit-tested
- [ ] #2 The conversion of at least three corpus songs equals the original's messages (rbhost log or hand-checked values in the test)
<!-- AC:END -->

---
id: TASK-025
title: TR-909 device
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-024
priority: high
type: feature
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The TR-909 device (docs/initial_investigation.md sections 7.2, 7.3): descending trigger order, step values and gains (x0.8, x2.0), flam (flamTick, retrigger at x1.5), BD sample switching and alternation, the snare noise with SEQ909, toms/crash/ride pitched voices with kv, CH/OH decay and choke, the DT table. Tests: every 909 corpus song matches its reference.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every TR-909 corpus song (including flam) matches its reference per the pass criterion
<!-- AC:END -->

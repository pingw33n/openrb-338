---
id: TASK-027
title: 'Effects: PCF and compressor'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-021
  - TASK-023
priority: medium
type: feature
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The PCF insert (docs/initial_investigation.md section 8.2: per-block step from the absolute position, the static patterns, the cutoff smoother, the envelope per 4 samples with slow/instant attack, the LP and BP filters and their tables, the state stores at block end) and the compressor (section 8.3: the 16-block update, the detector per 4 samples, hold, attack/release, floor, the linear gain ramp, mono on a channel and linked stereo on master). Tests: the PCF and compressor corpus songs match their references.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every PCF and compressor corpus song matches its reference per the pass criterion
<!-- AC:END -->

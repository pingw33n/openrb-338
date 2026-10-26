---
id: TASK-027
title: 'Effects: PCF and compressor'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 18:24'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26, from task-022's review: the mixer's set-param and msg 143 also write dirty bytes (0x81, 0x82, 0x85+6c+k at 0x10015cc8, 0x10015ce4, 0x10015cfb, 0x1001593f, 0x10015981); MixerDevice::set_param does not model them and their readers (0x10015f9b-0x1001609e) are unchecked. Confirm they are panel-only, or model them, as part of the compressor/mixer work.
<!-- SECTION:NOTES:END -->

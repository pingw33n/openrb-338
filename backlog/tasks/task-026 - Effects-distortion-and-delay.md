---
id: TASK-026
title: 'Effects: distortion and delay'
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
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The distortion insert (docs/initial_investigation.md section 8.1: 16-block update with the 5x5 grid interpolation, the 0.9/0.1 gain smoothing, the waveshaper table lookup, the "1.5 mode" for shape 0) and the delay send effect (section 8.4: the global 131072-sample buffer prefilled with 1e-6, the integer delay-length formula with the mode polarity resolved in docs/song_load.md, feedback, pan via PAN row 127, the never-reset write pointer as initial state). Tests: the distortion and delay corpus songs match their references.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every distortion and delay corpus song matches its reference per the pass criterion
<!-- AC:END -->

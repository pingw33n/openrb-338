---
id: TASK-021
title: 'Engine frame: block loop, mixer, output conversion and export driver'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-019
priority: high
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The engine frame (docs/initial_investigation.md sections 3, 4, 9): the 64-frame block loop with the leftover buffer and the 0x4000 cap, the mixer block (1e-6 bus fill, per-channel device then inserts dist, PCF, comp, the send accumulation, the 0.93/0.07 smoothing with the in-block ramp, the delay, the master comp, the master gain), the 16-block mixer update with the counter starting at 10, the T and PAN curves, routing, the float-to-int16 output conversion, and the export driver (the double-precision chunking of section 3 with 2048-frame calls). An explicit initial-state struct lets a caller set the state the decided capture protocol leaves (the leftover buffer, the counters, the smoothed gains) per docs/export_state.md. Devices are traits so the synths, drums and effects plug in later; test with a stub device against hand-computed mixer outputs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The block loop, mixer and output conversion follow sections 3 and 4 with unit tests for smoothing, pan law, routing order and int16 conversion
- [ ] #2 The export driver's frame count equals the original's for the corpus songs' tempos and loop lengths (test)
- [ ] #3 The silence song of the corpus renders exactly equal to its reference
<!-- AC:END -->

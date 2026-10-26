---
id: TASK-024
title: Drum voices and the TR-808 device
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 17:52'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-020
  - TASK-022
priority: high
type: feature
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The four drum voice classes (SIMPLE, FILTER, DUAL, PITCHED: docs/initial_investigation.md section 7.1, the pair-wise gain update, block-end f32 stores, end conditions, padding) and the TR-808 device (sections 7.2, 7.3: step function with ascending trigger order, accent, BD voice alternation with the global flag, snare noise from SEQ808 with the global counter, CH choke, selectors, the tune table, the fixed voice summation order). The global counters/flags are part of the initial-state struct (their values come from the capture protocol). Tests: every 808 corpus song matches its reference.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every TR-808 corpus song matches its reference per the pass criterion
- [ ] #2 The voice classes have unit tests against hand-computed block outputs
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26, from task-021's review: the 808 and 909 snare and bass-drum counters are process-global, not per song (docs/export_state.md:359-362, section 7.3; rbhost's --drum-counters). The frame's BlockContext (crates/rb-engine/src/frame/mod.rs:71-77) is immutable and has no seam for process state: add an engine-owned process-state handle shared by both drum devices, with its initial value settable by the export entry point (to match rbhost --drum-counters).
<!-- SECTION:NOTES:END -->

---
id: TASK-023
title: TB-303 devices (2.0 and vintage sound)
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 10:12'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-022
priority: high
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The two TB-303 devices (docs/initial_investigation.md section 6; the sound-model selection in docs/song_load.md section 3): parameters, the step function (gate 24 ticks, slide tie, bar-line pattern latch, patStart), note-on/note-off with the ^64/^4 coefficients, glide, the pitch table, the ReBirth 2.0 sound engine 0x100248C4 (GLOB[482] = 0, global flag +21 zero; per 64-sample block, interpolated resonance rows, linear coefficient ramp, click-free retrigger; used by every shipped song) and the Vintage sound engine inline in 0x10023F10 (GLOB[482] = 1; control values every 4 samples), the oscillator, one-pole, 2-pole filter and DC blocker, with exact f32 store points. The survey first had these two engine labels swapped; task-003 corrected them in docs/initial_investigation.md sections 6.2 and 6.3. Verify the uncertain points against the disassembly (0x10023F10, 0x100248C4, 0x10023CD4, 0x10023C74, 0x100253A0) before relying on them. Tests: the single-303-note corpus songs match their references.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every TB-303 corpus song matches its reference per the pass criterion
- [ ] #2 Both sound models are covered by corpus songs and tests
<!-- AC:END -->

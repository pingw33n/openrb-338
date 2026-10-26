---
id: TASK-022
title: 'Sequencer: clock, events, chase, shuffle, loop'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 17:52'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-018
  - TASK-021
priority: high
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The sequencer (docs/initial_investigation.md section 5): the double position advanced by tempo*4.6439909297052154e-06 per block before rendering, integer ticks, the track events applied in the first block whose tick reaches them, the loop-end wrap with chase, locate/start/stop semantics, song end at 767231, shuffle (96*(t/96)+max(0,t%96-shuffle/6)), device step calls only when the tick increments. Unit tests on tick sequences for given tempos and on chase results.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Section 5's clock, chase, locate/start/stop, loop wrap, song end and shuffle are implemented with unit tests
- [ ] #2 Event and step timing (the block in which each fires) is tested for at least three tempos
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26, from task-021's review (non-blocking items for this task):
- crates/rb-engine/src/frame/export.rs:86-87 says export runs "after the song is loaded and activated", but export() does not model activation (msg 53), which per tools/rbhost/rbhost.c:544-555 changes the 303 release constants. Model activation in the export path (or in the 303 task if it only affects the 303) and fix the comment.
- frame/mod.rs:732-739 Engine::reset (msg 33) refills only the one send bus with 1e-6; the DLL also refills +0x170C and +0x180C (0x10014155..0x10014183; 256-byte buses at 0x160C + k*0x100) whose readers are unknown. Find their readers (they may be the drum or 303 buses) and model them where they belong; record the result in docs/initial_investigation.md.
- The frame's Device/Effect/Transport traits pass only what task-021 needed; reshape them here as the sequencer requires.
<!-- SECTION:NOTES:END -->

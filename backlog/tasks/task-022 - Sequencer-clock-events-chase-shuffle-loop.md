---
id: TASK-022
title: 'Sequencer: clock, events, chase, shuffle, loop'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
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

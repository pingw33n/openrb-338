---
id: TASK-013
title: 'Decide: reference capture protocol and the engine''s pass criterion'
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-10-26 09:06'
labels:
  - decision
  - tests
milestone: m-0
dependencies:
  - TASK-012
priority: high
type: spike
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Choose the source of truth for references and the pass criterion for the engine, from the results of the determinism experiment and the rbhost validation (see their tasks and docs/export_state.md).

Options for the reference:
1. The app's export under the recommended protocol (the real thing; any residual variation must be modeled by the engine as initial state).
2. rbhost renders (deterministic by construction; validated against the app).
3. Both: rbhost for the bulk corpus, app exports for a smaller set that checks rbhost.
Options for the pass criterion: byte-exact; within ±1 LSB per sample (x87 transcendental table differences); exact after a settling period of N frames.

Recommendation: option 3, byte-exact for rbhost references, with any tolerance only where a specific, documented cause (for example a table entry that cannot be reproduced) requires it.
<!-- SECTION:DESCRIPTION:END -->

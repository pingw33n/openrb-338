---
id: TASK-012
title: Validate rbhost renders against the app's exports
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
labels:
  - tests
  - testmachine
milestone: m-0
dependencies:
  - TASK-010
  - TASK-011
priority: high
type: task
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
rbhost (task tools/rbhost) is only useful as a reference if its output is what the original app exports. Render the diagnostic songs with rbhost and with tools/rb_export.py under the most deterministic app protocol found by the determinism experiment, compare them with tools/rbcmp.py, and explain any difference (it should be explained by the state the app protocol leaves uncontrolled, or by a mistake in rbhost's song loading to be fixed in rbhost). Results in the notes and a section in docs/export_state.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every diagnostic song was rendered both ways and compared; the comparison table is in docs/export_state.md
- [ ] #2 Each difference is explained or fixed in rbhost (with the fix's commit referenced)
<!-- AC:END -->

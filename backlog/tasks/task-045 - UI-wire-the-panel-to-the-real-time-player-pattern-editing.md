---
id: TASK-045
title: 'UI: wire the panel to the real-time player, pattern editing'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 16:23'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-043
  - TASK-044
priority: high
type: feature
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Wire the panel to the real-time player: every knob, slider and switch drives its engine parameter live; transport and tempo/shuffle; pattern and bank selection with the original's timing (pattern changes at the bar line); the 303 step editor and the 808/909 step buttons and instrument selector edit patterns live; the pattern/song mode switch, song position and loop displays follow playback; LEDs show the current step. Behavior per docs/ui_spec.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every control in docs/ui_spec.md drives its engine parameter live, checked against a list in the notes
- [ ] #2 Pattern editing of the 303s and drums works as the spec says, and edits are heard on the next pass
- [ ] #3 Pattern and song mode play shipped songs; positions and step LEDs follow playback
<!-- AC:END -->

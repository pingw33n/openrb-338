---
id: TASK-164
title: Watch the panel play after the task-153 repaint change
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-10-30 15:19'
labels:
  - ui
  - tests
milestone: m-5
dependencies:
  - TASK-153
references:
  - crates/orb-ui/src/link.rs
  - docs/ui_spec.md
priority: low
type: task
ordinal: 180000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-153 (2026-10-30) made the panel repaint only while something moves, at most 30 fps (Link::moving, METER_FRAME 33 ms, Panel::next_change), and cached part textures and knob-arc meshes. The agent could not watch the new build play because the Mac's screen was locked, so task-153's AC #3 covers only identical screenshots, the frame rate and a headless frame-by-frame test. A person still has to look at it once.
What: run OpenRB-338 from main, play a few songs (pattern and song mode, with a mod), resize the window and change displays, and watch the LEDs, meters, the 303 note flash, the song position and the knobs while dragging. Report any stutter, frozen element or missing art as a bug task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Played at the startup size and maximized: LEDs, meters, the 303 note flash and the song position move smoothly
- [ ] #2 After resizing and moving between displays no panel part is missing or blurred
- [ ] #3 Stopped, the panel still reacts at once to clicks, drags and keys
<!-- AC:END -->

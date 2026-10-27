---
id: TASK-075
title: >-
  RE: the Mods menu's mod change with several song windows open (msg 53 round
  trip)
status: To Do
assignee: []
created_date: '2026-10-27 14:33'
labels:
  - re
  - rbm
  - ui
milestone: m-3
dependencies:
  - TASK-056
references:
  - docs/rbm.md
  - docs/export_state.md
priority: low
type: spike
ordinal: 99000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on a mod change sub_42D80D destroys the song window (sub_47350E) and builds a new one (sub_42DA72). With other song windows open, Windows may activate another song's window in between (MDI activation sub_42D7BE -> sub_40D163 -> top-level msg 53 for that song) before the new window activates this song again (msg 53 back, 0x42E5AE). On stopped songs that runs slot 6 on both songs (the shared delay line refilled, docs/export_state.md section 3); on a playing one the other song becomes pending. docs/rbm.md section 14.2 marks this [G]. rb's UI has one song, so this matters only if rb ever opens several songs.
Do: confirm or refute the round trip, statically (MDI destroy/activate order in the exe's window code) or on the test machine (two songs open, a mod change on one, with an rbhost-style message trace or by ear/export), and record the result in docs/rbm.md section 14 with addresses.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/rbm.md section 14 states, with addresses or a test-machine run, whether msg 53 is sent for another song during a mod change with several songs open
<!-- AC:END -->

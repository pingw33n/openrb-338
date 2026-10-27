---
id: TASK-074
title: >-
  rb-player: the mod's startup sound on a mod change (msg 58, the preview
  player)
status: To Do
assignee: []
created_date: '2026-10-27 14:33'
labels:
  - engine
  - rbm
  - ui
milestone: m-3
dependencies:
  - TASK-056
references:
  - docs/rbm.md
  - docs/export_state.md
  - crates/rb-player/src/lib.rs
priority: low
type: feature
ordinal: 98000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: when the Mods menu changes the mod of an open song, the original plays the new mod's startup sample: sub_426FB3 ends with sub_426C67, which sends top-level msg 58 {u16 mod} when the mod record's word +1088 is 1 (the mod just opened) and the preference 'Play Mod Startup Sound' (app byte +0x533, sub_4098C3) is on; the Mod Selector dialog does the same when an entry is selected (sub_41A0C5, 0x41A180). The DLL's preview player (docs/export_state.md section 8: SafeSoundCallback mixes the mono int16 sample at engine +0x428 from +0x42C after the song, h = s/2 rounded toward zero, L and R saturated; cleared at the sample end and by msg 57) is not modelled in rb-player (task-056 left it out; docs/rbm.md section 14.3). The startup sample is slot 46 of the mod (Startup.aif, resource 1024 for the built-in kit).
Do: add the preview player to rb-player (a Command that starts the slot-46 sample of a sample set, mixed after the song as section 8 describes, allocation-free in the callback), and let the UI send it on a mod change and a logo click (sub_42A799) when the preference is on. A Decide: task is needed only if the preference's default or storage is not obvious from docs/test_machine.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb-player mixes a startup sample after the song as docs/export_state.md section 8 describes, with a unit test of the rounding and saturation
- [ ] #2 the callback stays allocation-free (crates/rb-player/tests/no_alloc.rs covers the new command)
<!-- AC:END -->

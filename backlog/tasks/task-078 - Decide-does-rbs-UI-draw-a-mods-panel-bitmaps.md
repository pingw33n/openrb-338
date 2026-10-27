---
id: TASK-078
title: 'Decide: does rb''s UI draw a mod''s panel bitmaps'
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-27 15:26'
updated_date: '2026-10-27 15:38'
labels:
  - decision
  - ui
milestone: m-3
dependencies: []
references:
  - docs/rbm.md
  - docs/ui_spec.md
priority: low
type: spike
ordinal: 101000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Context: the original redraws the whole song window with the bitmaps of the chosen mod (docs/rbm.md 13.1: 58 numbered JPEG resources a mod replaces; sub_42D80D destroys the song window and builds a new one with the mod's art). task-047's Mods menu changes the kit (samples, song msg 19) and the saved mod name, but rb-ui draws only its own SVG art (decision task-040), so a mod's look is not shown. Options: (1) never: the SVG art stays, a mod changes sound only (the user's UI 'does not have to match the original exactly'); (2) show the mod's JPEGs for the controls they cover, by mapping the 58 bitmaps onto rb's parts (large: the bitmaps differ in geometry from the SVG parts); (3) a skin layer later, after m-3's core. Recommendation: (1) for now.
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Decided by the user on 2026-10-27: rb's UI keeps its SVG art; a mod changes the sound only (no mod panel bitmaps). Reflected in docs/ui_spec.md section 18.
<!-- SECTION:FINAL_SUMMARY:END -->

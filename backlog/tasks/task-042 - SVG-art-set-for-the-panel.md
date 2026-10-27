---
id: TASK-042
title: SVG art set for the panel
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 16:23'
updated_date: '2026-10-27 06:33'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-041
priority: medium
type: feature
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Draw the panel as scalable SVG of our own: the full panel background per section, knobs (with rotation-friendly layering), sliders, switches, LEDs, step buttons, the 7-segment digits, labels and logos of our own (no Propellerhead or Roland marks). The layout follows docs/ui_spec.md so controls sit where the original's do; the look is "similar", not a copy of the original bitmaps. Files under `assets/ui/`, with a README of the parts, their anchor points and how the app composes them; license our own (state it). Check that every part renders correctly with resvg at 1x, 2x and 3x.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 assets/ui/ holds SVG parts for every panel element in docs/ui_spec.md, with a README of parts and anchors
- [ ] #2 No original artwork or third-party marks are used; the license is stated
- [ ] #3 A render check shows every part at 1x, 2x and 3x (screenshots in the notes)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-27, per task-040: the art is drawn by egui through egui_extras' SVG loader (usvg + resvg), loaded at the displayed physical pixel size; check the parts render correctly with resvg (the same renderer).

Supervisor 2026-10-27, per the user: the visual look of the user's experiment is the base for the art set. It is crates/rb-ui-poc on branch worktree-ui-poc (commit 1359e39; not on main): an egui/eframe 0.37 front panel redrawn in a flat style from SVG generated in code (src/art.rs, src/sprites.rs, src/widgets.rs; design size 1200 x 940; palette in art::pal). Read it with `git show worktree-ui-poc:crates/rb-ui-poc/src/art.rs` etc. or the worktree .claude/worktrees/ui-poc. Keep its look (palette, flat style, knob/button styles) and lay the controls out per docs/ui_spec.md.
<!-- SECTION:NOTES:END -->

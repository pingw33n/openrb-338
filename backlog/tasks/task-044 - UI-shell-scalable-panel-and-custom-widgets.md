---
id: TASK-044
title: 'UI shell: scalable panel and custom widgets'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 16:23'
updated_date: '2026-10-27 06:30'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-040
  - TASK-041
  - TASK-042
priority: high
type: feature
ordinal: 44000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The application shell: a window with the chosen toolkit, the panel composed from the SVG art (assets/ui/) at any window size with crisp re-rasterization on scale change, all controls drawn at their docs/ui_spec.md positions, and the custom widget set (knob, slider, switch, LED, step button, 7-segment display) with the original's interaction (drag behavior, fine adjust). Controls are not yet wired to audio. The binary is part of the workspace (e.g. crates/rb-ui, binary `rb-ui`, or a subcommand of `rb`; say which).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The app opens a window showing the full panel from the SVG art, crisp at any size and pixel density (screenshots at two sizes in the notes)
- [ ] #2 Every widget type works with the original's interaction per docs/ui_spec.md
- [ ] #3 cargo test and clippy pass; widget behavior has unit or snapshot tests
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-27, per task-040: egui + eframe; SVG via egui_extras image loaders (svg feature) with size hints = displayed size in physical pixels, reloaded on scale change (no own rasterizer); panel keeps the original aspect ratio, letterboxed; minimum window 0.5 x native size. A prototype crates/rb-ui-poc exists uncommitted in the .claude/worktrees/ui-poc worktree (not from this backlog) — ask the supervisor before using or touching it.
<!-- SECTION:NOTES:END -->

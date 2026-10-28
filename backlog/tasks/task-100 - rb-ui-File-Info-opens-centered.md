---
id: TASK-100
title: 'rb-ui: File > Info opens centered'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:38'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/info.rs
priority: low
type: enhancement
ordinal: 120000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that the Song Information window (File > Info, App::info_ui in crates/rb-ui/src/info.rs) open centered in the app window. Today egui places it at its default position.

What: open it centered over the window (for example pivot Align2::CENTER_CENTER at the content rect's center as the default position, so the user can still drag it); each new open is centered again. It must stay fully inside the window for small window sizes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 File > Info opens the window centered in the app window; reopening centers it again; it stays draggable
- [ ] #2 A headless test checks the window's rect center is within a few points of the screen center at open
- [ ] #3 cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

---
id: TASK-100
title: 'rb-ui: File > Info opens centered'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:38'
updated_date: '2026-10-28 09:11'
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
- [x] #1 File > Info opens the window centered in the app window; reopening centers it again; it stays draggable
- [x] #2 A headless test checks the window's rect center is within a few points of the screen center at open
- [x] #3 cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
SUPERSEDED by review round 1. Shipped design: Window with pivot CENTER_CENTER, default_pos at the content rect center and an id per open (App::info_serial, bumped only when the window was closed), so egui centers it from the first frame and a drag moves the stored place. vscroll + max_size keep it inside a small window. Headless test checks centering, drag, a second File > Info while open, reopen and a 200 pt window.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a080df79d0c71180d

egui 0.37 finding: Window::current_pos is overridden by the stored pivot_pos for a title-bar-drag window (window.rs:607-672), so after the first frame the stored AreaState is set through Areas::set_state (size known from the previous frame; up to 6 frames until centered within 0.5 pt). Window id is now fixed (info_window_id()). Small windows: vscroll + max_size = content rect - 16 so it fits.

Review round 1 (send back): replaced the set_state loop by the plain egui mechanism: Window::pivot(CENTER_CENTER) + default_pos(content_rect center) + an id per open (App::info_serial, incremented in show_info only when the window was closed). Prototyped: centers from the first frame, no loop, never fights a drag; an open window is not moved by a second File > Info; removed redundant constrain(true). Test extended (second File > Info keeps the dragged place; id via App::info_id()). The earlier current_pos finding is moot.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
File > Info opens the Song Information window centered in the app window (App::info_ui, crates/rb-ui/src/info.rs). Window::pivot(CENTER_CENTER) with default_pos at the content rect center and an id per open (App::info_serial, bumped by show_info only when the window was closed), so egui centers it from the first frame; each reopen centers again, dragging moves it, and File > Info on the open window leaves it. vscroll and max_size (window less 16 pt) keep it inside a small window. Test the_info_window_opens_centered_and_stays_draggable (tests/menu.rs) checks centering within 3 pt, a title-bar drag, a second File > Info keeping the dragged place, reopen after Escape centering again, and containment in a 200 pt high window. cargo fmt, clippy -D warnings clean, cargo test --release -q with RB_STANDARD_MOD and RB_MOD_DIR: 484 pass (summed with awk), 0 failed.
<!-- SECTION:FINAL_SUMMARY:END -->

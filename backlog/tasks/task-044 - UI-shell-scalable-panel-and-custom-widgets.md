---
id: TASK-044
title: 'UI shell: scalable panel and custom widgets'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 16:23'
updated_date: '2026-10-27 13:29'
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
- [x] #1 The app opens a window showing the full panel from the SVG art, crisp at any size and pixel density (screenshots at two sizes in the notes)
- [x] #2 Every widget type works with the original's interaction per docs/ui_spec.md
- [x] #3 cargo test and clippy pass; widget behavior has unit or snapshot tests
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. New crate crates/rb-ui (lib + binary rb-ui): build.rs embeds assets/ui/*.svg; layout.rs parses layout.tsv; behavior.rs holds the original's widget math as pure functions with unit tests; model.rs the panel state + engine-bound events; app.rs draws every part as egui::Image (egui_extras SVG loader, size hint = displayed physical px) and runs the gestures.
2. Headless egui tests (synthetic pointer events through App::show) for each widget type, and a load/rasterize check of every part at 1x/2x/3x through the egui loader.
3. Screenshots via rb-ui --screenshot at two window sizes into docs/ui_app/.
4. clippy, fmt, cargo test.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-27, per task-040: egui + eframe; SVG via egui_extras image loaders (svg feature) with size hints = displayed size in physical pixels, reloaded on scale change (no own rasterizer); panel keeps the original aspect ratio, letterboxed; minimum window 0.5 x native size. A prototype crates/rb-ui-poc exists uncommitted in the .claude/worktrees/ui-poc worktree (not from this backlog) — ask the supervisor before using or touching it.

Supervisor 2026-10-27, per the user: the ui-poc experiment (branch worktree-ui-poc, commit 1359e39, crates/rb-ui-poc) was the user's; its visual look is the base. Its code may be reused where it fits (it uses eframe 0.37 and resvg 0.49 directly); task-040 chose egui_extras' SVG loader at the displayed pixel size, so reconcile its rendering path with that or explain why the poc's direct resvg use is kept.

Branch: worktree-agent-aeb3a8e1d1214931f (crate crates/rb-ui, binary rb-ui)

Reconciliation with the ui-poc (branch worktree-ui-poc): its look is the base (the art set of task-042 is generated from the poc's palette and shapes), but its rendering path is not kept. The poc generated SVG text at run time and rasterized it with its own resvg cache (sprites.rs). rb-ui uses egui_extras' SVG loader as task-040 decided: each part is egui::Image::from_bytes(uri, svg).fit_to_exact_size(rect) painted with paint_at, which loads with SizeHint::Size = round(ppp * rect size) (egui does this itself) and so re-rasterizes on a scale change. No own rasterizer and no resvg dependency in rb-ui (only the dev-dependency rb-ui-art, for cross-checks). The poc's widget feel was replaced by the original's (docs/ui_spec.md section 2): the poc's scroll wheel, double-click reset, fixed 180 pt knob range and click-to-cycle selectors are gone.

Evidence and findings:
- All 179 parts load through the egui loader with the exact pixel size round(ppp*w) x round(ppp*h) at 1x, 2x and 3x (test every_part_loads_and_rasterizes_at_1x_2x_3x). Task-042's art needed no change.
- layout.tsv ids are not unique: tb303_N.accent is both the accent knob and the accent flag button. egui widget ids and gesture state are keyed by (kind, id); documented in assets/ui/README.md. Not renamed (the generator and its spec check belong to task-042).
- egui 0.37: Ui::id and Id::new are deprecated (Id::unique used). The root Ui of a headless Context::run_ui reports a 5000x5000 rect on its first pass, so the app uses ctx.content_rect() and the tests warm up one pass.
- Knob pointer: Image::rotate of the pointer layer around (cx, cy); the value arc is a painter polyline (3 degree steps) in the README's arc color. The rotated pointer is a bilinear resample of the SVG raster at the displayed size, a little softer than an unrotated part.
- Interaction per docs/ui_spec.md section 2, each with a test: knob (no jump at mouse down, up increases, horizontal ignored, span 62/36/40/41/64, Shift at mouse down = span x5, sent value (127p+500)/1000 only on change; the measured cases CUTOFF 31 px = 64 and RESO with Shift 31 px = 13 pass); fader (handle grab vs track jump, k=232, no fine mode); repeat and wind buttons (one step at mouse down, +1 per 100 ms from 400 ms, +1 per 20 ms from 2400 ms; held 1.4 s gives 11 steps); two-position switches by half, including a drag across the middle; pattern grid and bank (lit only when the pattern's bank is shown; a click gives the row the focus); TR-808 steps (opposite of the clicked step, a drag paints every crossed step, the running light inverts the playing step); TR-909 steps (off, 1, 2, off; Ctrl 1; Alt 2; Ctrl+Alt or flam 3; accent row off/2; hi-hat exclusion; drag); instrument legends, upper labels, name rows and AC area (a drag keeps selecting; the 808 selector knob and the legend follow each other); 303 step editor (Step/Back mod 16, pitch mode resets to step 1 and keys advance, flag flips, Clear); transport (tempo 20..500, bar and loop arithmetic with clamps, Stop locates, Rewind/FF 10 bars, record-off rules, displays disabled in pattern mode); mixer PCF/Comp exclusivity (ids c+1 and c+2, master comp 1); delay steps 1..32, PCF pattern 0..54, pattern steps 1..16 per pattern. Space = play/stop and Up/Down = focus are the only keys.
- Every change the engine must hear is pushed to Panel::events (model::Event) for task-045. Nothing is wired; crates/rb-player and crates/rb-engine are untouched.
- Screenshots (rb-ui --demo --screenshot; retina display, so the 624x510 point window is 1248x1020 px): docs/ui_app/panel_624x510.png (native size at 2x density), docs/ui_app/panel_1000x600.png (letterboxed, scale 1.176 at 2x), docs/ui_app/panel_312x255.png (the 0.5x minimum, 1:1 pixels). Reproduce: cargo run --release -p rb-ui -- --demo [--size WxH] --screenshot OUT.png.
- Tests: 43 in rb-ui (28 unit incl. cross-checks of backgrounds and knob designs against rb-ui-art, 15 headless egui); whole-workspace cargo test passes; cargo clippy -p rb-ui --all-targets -- -D warnings clean.
Not done: keyboard of section 10 beyond Space/Up/Down (follow-up task created); the rack's vertical scroll in a window lower than 510 (the panel is letterboxed instead, per task-040); the logo click; the pitch-mode note flash time (0.4 s) is a guess [G].

Review round 1 (send back, only task-070): fixed task-070 to the decision task-054 (option C: physical US positions; Note/Pause and drum AC on the key right of 0, = alias; numpad 4/5 rewind/fast forward), replaced its stale dependency note, added type feature. Code unchanged.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added crates/rb-ui (library plus binary rb-ui, a workspace member): the panel window drawn from assets/ui through egui_extras' SVG loader (size hint = displayed physical pixels, reloaded on scale change), letterboxed at the original's aspect ratio, minimum window 312x255 (0.5x). All 352 controls are placed from layout.tsv; widgets (knob with arc and rotated pointer, fader, toggle, trigger, repeat/wind, two-position switch, grids, 808/909 step strips, instrument selection, LEDs, 7-segment displays, meters) follow docs/ui_spec.md section 2 and drive a Panel model with the engine-bound Event queue for task-045. Not wired to audio. Screenshots at three sizes in docs/ui_app/ (AC 1: 624x510 at 2x, 1000x600 letterboxed, 312x255 minimum). Tests: cargo test 43 in rb-ui (28 unit, 15 headless egui including every part loading and rasterizing at 1x/2x/3x), whole-workspace cargo test passes, clippy -D warnings clean. Follow-up: task-070 (keyboard, depends on decision task-054). Branch worktree-agent-aeb3a8e1d1214931f, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

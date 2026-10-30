---
id: TASK-153
title: 'UI performance: cut the CPU the panel uses while a song plays'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-30 08:15'
updated_date: '2026-10-30 13:15'
labels:
  - ui
milestone: m-5
dependencies: []
references:
  - crates/orb-ui/src/app.rs
  - crates/orb-ui/src/art.rs
  - crates/orb-ui/src/link.rs
  - docs/ui_spec.md
priority: medium
type: enhancement
ordinal: 171000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user saw OpenRB-338 use about 40% CPU on macOS (2026-10-30). The supervisor measured it the same day, in a release build on an Apple-silicon Mac:
- with "Alloy Cowboy Resurrection.rbs" open in a maximized window, the app used 1.4 s of CPU in 10 s (about 14%);
- with no song, the app used 0.04 s in 10 s;
- `sample` showed the CoreAudio output thread (cpal-coreaudio-default-output) essentially idle;
- the time is on the main thread in eframe's run_ui_and_paint: orb_ui::app::App::ui building the panel, egui tessellation, and egui_wgpu paint and wgpu command encoding, plus the Metal queues and the window server.

The engine is cheap: `orb render` does a five-minute song in about 1.4 s, about 0.5% of one core in real time. So while playing, nearly all the CPU is the UI redrawing the whole panel at 60 fps: crates/orb-ui/src/app.rs:510 requests a repaint every 16 ms whenever a player link exists. egui is immediate mode and egui_wgpu repaints the whole surface each frame; it has no dirty rectangles. The user cares about the playing case. The idle case (no repaint needed while stopped) is welcome, but secondary.

What, in order, measuring after each step:
1. Measure first. Record CPU% (ps or `top -l`, and the cpu time over a fixed 20 s) while playing a busy song at the startup window size and maximized, with the frame time from egui's or eframe's timing or a puffin profile. Record per-frame counts: shapes, meshes, vertices and texture binds. Keep the numbers in the notes.
2. Repaint only as often as something changes. The engine's snapshot (step LEDs at sixteenth notes, the song position, meters) changes far less often than 60 times a second. Request a repaint when link.sync reports a visible change, at most 30 fps for the meters (the original's own panel refresh rate is worth checking in docs/ui_spec.md or the disassembly before choosing), and none while stopped with nothing animating (LED fades, held buttons, the logo animation and an export keep their own requests).
3. Make a frame cheaper, if steps 1-2 leave it significant: cache the static layer (background, frames, labels: everything that does not change while playing) into an offscreen texture once per size and scale change, through an egui_wgpu paint callback or a TextureHandle rendered from the SVG parts. Each frame then draws one textured rectangle plus the moving parts (knobs, sliders, LEDs, displays, meters). Also look for per-frame waste in App::ui: allocations, text layout redone every frame, and SVG parts re-tessellated instead of drawn as cached textures.
4. Keep the look: a before/after screenshot pair (the hidden --screenshot option, docs/building.md) at two scales must be pixel-identical, or each difference must be explained.

Out of scope: true partial presentation (damage regions) or a renderer outside egui.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Baseline and final CPU while playing (startup size and maximized) and the frame time are recorded in the notes, with the commands
- [ ] #2 Playing a song uses substantially less CPU than the baseline (target: half or less); stopped with nothing animating, the app does not repaint continuously
- [ ] #3 The panel looks the same: the before/after screenshots at two scales are pixel-identical, or the differences are explained; LEDs, meters and the song position still follow playback smoothly
- [ ] #4 fmt, clippy and the orb-ui tests pass; docs/ui_spec.md states the repaint rule
<!-- AC:END -->

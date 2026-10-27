---
id: TASK-040
title: 'Decide: UI toolkit and audio output for the bundled UI'
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-26 16:23'
updated_date: '2026-10-27 06:30'
labels:
  - decision
  - ui
milestone: m-3
dependencies: []
priority: high
type: spike
ordinal: 40000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
m-3 needs a Rust UI toolkit and an audio output library. Requirements (from the user, 2026-10-26): a desktop window whose panel looks and behaves like the original ReBirth (knobs, sliders, switches, LEDs, step buttons, 7-segment displays), drawn from our own SVG art and scalable to any window size without blurring; custom widgets throughout; no native dialogs (file selection is an in-app browser); real-time audio playback with low latency; macOS, Windows and Linux.

Options for the UI:
1. egui + eframe (suggested by the user): immediate mode, custom widgets are straightforward, wgpu or glow backends; SVG through resvg/usvg rasterized to textures at the current scale (re-rasterized on scale change), or tessellated with lyon.
2. iced: retained/Elm-style, built-in canvas and SVG widget.
3. A lower-level stack (winit + wgpu/vello) with our own widget layer: most control over vector rendering, most work.
For audio: cpal (cross-platform, callback-based), with the engine rendering in the audio callback through a lock-free command queue from the UI thread.

Recommendation: egui + eframe with resvg-rasterized SVG at the current pixel density, and cpal for audio. Decide also the minimum window size and whether the panel keeps the original aspect ratio when scaled.
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Decided by @pingw33n on 2026-10-27:
- UI toolkit: egui + eframe.
- SVG: use egui's own SVG support (egui_extras image loaders with the svg feature, which parse with usvg and rasterize with resvg into textures). No separate rasterizer in rb. Load each SVG with a size hint equal to its displayed size in physical pixels (points x pixels_per_point) and reload when the scale changes, so it stays crisp; egui itself draws only textured triangles, so this is the vector path egui offers.
- Audio output: cpal (engine renders in the callback; commands through a lock-free queue).
- Scaling: the panel keeps the original's aspect ratio and is letterboxed to fit the window; minimum window size = 0.5 x the original's native panel size.
<!-- SECTION:FINAL_SUMMARY:END -->

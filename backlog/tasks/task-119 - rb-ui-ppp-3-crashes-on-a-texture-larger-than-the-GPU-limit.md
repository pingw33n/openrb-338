---
id: TASK-119
title: 'rb-ui: --ppp 3 crashes on a texture larger than the GPU limit'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 13:22'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/app.rs
  - crates/rb-ui/src/main.rs
priority: medium
type: bug
ordinal: 139000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: found during task-116 (2026-10-28). `OpenRB-338 --ppp 3` crashes because wgpu refuses a texture 11730 px wide; the limit is 8192. It crashes with or without the About window open. A real display at a high pixel density, or a large window, may hit the same limit. The panel parts are SVGs rasterized by egui_extras' loader at the size they are painted (paint_part, Image::from_bytes(...).fit_to_exact_size), so one part painted very wide at a high density can exceed the GPU's maximum texture size. The texture is probably the wood or a full-width background, or the composed panel.

What:
- Reproduce: --ppp 3 --size 624x510 --screenshot, and a maximized window at the Mac's density. Name the part, the requested size and where it is requested. Record the backtrace in the notes.
- Fix it so that no texture exceeds ctx's maximum texture side: egui exposes it through the painter or render state; or use wgpu's limits, or a safe constant such as 8192. Possible fixes:
  - cap the rasterization size per part and let egui scale it up (a part that wide is a plain fill or wood, so slight softness is acceptable);
  - tile it;
  - draw plain fills as egui rects instead of textures.
  Choose one and say why.
- Test: a headless test that paints the panel at a scale where the old code requested an oversized texture, and asserts that every requested image size stays within the limit (e.g. through the loader's requested sizes, or by computing them from the layout).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 OpenRB-338 --ppp 3 (and a maximized window at high density) runs without a texture-size crash; the cause and the chosen fix are in the notes
- [ ] #2 A test checks that no part is rasterized above the maximum texture side at a high scale; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

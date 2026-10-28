---
id: TASK-119
title: 'rb-ui: --ppp 3 crashes on a texture larger than the GPU limit'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 13:22'
updated_date: '2026-10-28 14:12'
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
- [x] #1 OpenRB-338 --ppp 3 (and a maximized window at high density) runs without a texture-size crash; the cause and the chosen fix are in the notes
- [x] #2 A test checks that no part is rasterized above the maximum texture side at a high scale; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Reproduce, find the oversized part, cap the rasterization size per part in paint_part_rotated, add a headless test of every uploaded texture size.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a2de13662e33e1916

Reproduced: OpenRB-338 --demo --ppp 3 --size 624x510 --screenshot panics in wgpu Device::create_texture, 'egui_texid_Managed(5)', Dimension X 11730 exceeds the limit of 8192. Backtrace: wgpu create_texture <- egui_wgpu Renderer::update_texture <- Painter::paint_and_update_textures <- eframe run_ui_and_paint (first frame, app_did_finish_launching).
Cause: found with a headless run (egui Context, ppp 3, screen 624x532 pt, textures_delta sizes). The FIRST frame after the ppp override draws the panel at a transient wrong View scale (5.34 in the test; the real window's first frame has a different stale size): every part is then painted in a rect 5.34x too big. Image::paint_at rasterizes the SVG at rect.size()*ppp with no cap: wood 85x2724 pt, bg_transport 3162x267 pt -> a 9488x801 px texture in the test (11730 px wide in the app). Later frames are normal (scale 1). A very large window at a high density can reach it for real too (bg_transport 592 pt wide at scale > 8192/592/ppp).
Fix chosen: in paint_part_rotated (app.rs) ask the loader for a smaller raster when rect px size exceeds ctx input max_texture_side (set by eframe from the GPU limits): capped_hint scales both sides by max/side, the texture is loaded with ImageSource::load and painted with Image::from_texture, so egui scales it up. Why this one: it keeps the SVG art for every ordinary size (no change below the cap), needs no tiling, and the part this wide is mostly plain fill so softness is harmless; it covers every part (all go through paint_part), not only the backgrounds.
Test: crates/rb-ui/tests/texture_size.rs asserts every full texture upload (textures_delta.set, pos None) is <= 8192 per side at ppp 3 in a 624x532 window and a 3000x2000 window. On the old code both fail (Managed(2) is 9488x801 at ppp 3). After the fix both pass. Screenshots (--ppp 3 --size 624x510, and maximized --ppp 3) exit 0 with the panel drawn correctly (scratchpad task119/a.png, max.png).
Checks: cargo +nightly fmt --all --check ok; cargo clippy --all-targets -- -D warnings ok; cargo test --release -q: 542 passed, 0 failed (RB_MOD_DIR set).

Review round 1: (1) tests/texture_size.rs and tests/mod_name.rs (task-103) now clear textures_delta (epaint panics in debug on a dropped TexturesDelta); (2) paint_part_rotated paints nothing when the capped load is not Ready, instead of the uncapped path; (3) CORRECTION of the cause: on the first frame ctx.content_rect() is egui's 10000-unit default viewport (about 3333 pt at ppp 3), so View::fit gives scale 5.34; it is not a wrong pixels_per_point; (4) tests: cargo test -p rb-ui in debug 169 pass, cargo test --release -q workspace 542 pass, 0 fail, RB_MOD_DIR set.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
OpenRB-338 --ppp 3 no longer crashes. Cause: on the first frame ctx.content_rect() is egui's 10000-unit default viewport, so View::fit gives an oversized scale (5.34 at ppp 3) and Image::paint_at rasterizes each SVG at its pixel size with no cap; wgpu refused a 11730 px wide texture (limit 8192). Fix: paint_part_rotated in crates/rb-ui/src/app.rs caps the raster size to the input's max_texture_side (capped_hint), loads it and lets egui scale it up; when the capped load is not ready it paints nothing that frame. Test crates/rb-ui/tests/texture_size.rs fails on the old code (9488x801 texture) and passes now; mod_name.rs also clears its textures_delta so it passes in debug. fmt and clippy clean; cargo test -p rb-ui debug 169 pass; cargo test --release workspace 542 pass. Branch worktree-agent-a2de13662e33e1916, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

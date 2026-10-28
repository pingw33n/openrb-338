---
id: TASK-116
title: 'rb-ui: the About window shows the panel''s wordmark logo'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 12:13'
updated_date: '2026-10-28 13:24'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui-art/src/parts.rs
  - crates/rb-ui/src/about.rs
priority: low
type: enhancement
ordinal: 136000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that Help > About show the same logo as the main window's top-right corner, instead of the app icon it shows now (task-102, crates/rb-ui/src/about.rs, the app_icon part).

Today the wordmark is drawn inside the bg_transport part (crates/rb-ui-art/src/parts.rs, transport_bg, the group at (458, 1)):
- 'OPENRB-' in INK and '338' in ORANGE, cap 14;
- an orange underline 118 x 1.5;
- 'BASS LINE AND DRUM MACHINE' at cap 3.4.

What:
- Move the wordmark out of bg_transport into its own part, for example `logo`, about 134 x 48 like the logo click area (layout::on_logo). Lay it out on the panel at the same place, so the main window looks exactly as before (compare screenshots before and after).
- The About window draws that part, scaled up crisply: it is SVG, so render it at the window's pixel density.
- Regenerate assets/ui (`cargo run --release -p rb-ui-art -- gen`) and update assets/ui/README.md.
- Keep the part simple to split later: task-114 animates its pieces ('OPEN', 'RB-338', the underline, the subtitle) in the panel and in About. If cheap, emit the pieces as separate parts now (logo_open, logo_rb338, logo_line, logo_sub), with the composite or a placement table, so task-114 can position them.
- The app icon (app_icon) stays the window, Dock and package icon.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The About window shows the wordmark logo (the same art as the panel's top-right), crisp at any scale; the main window looks identical to before (screenshot compare)
- [x] #2 The wordmark is its own part (or pieces) laid out on the panel; assets/ui regenerated and README updated; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rb-ui-art parts.rs: remove the wordmark from transport_bg; add parts logo_open, logo_rb338, logo_line, logo_sub (each on the full 134x48 canvas, all placed at (0,0) of the logo: no clipping, no placement math) and the composite logo (the four bodies in order), placed at panel (474, 1) = the on_logo rectangle. Add it to the section backgrounds in rb-ui-art layout.rs and rb-ui layout::backgrounds + part_size.
2. About window draws part logo at the window pixel density (fit_to_exact_size at 2x native, SVG re-rasterized by the loader).
3. Regenerate assets/ui, update README; tests: logo body equals the pieces, bg_transport has no logo, logo box equals the on_logo rect, About draws the logo part.
4. Screenshots before/after, pixel compare of the panel; About headless render.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-adb71ac9cbb017b1f. Screenshot before: /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task116/before.png

First attempt split 'OPENRB-' into 'OPEN' and 'RB-' with a recomputed x: not pixel-identical (R sat 0.07 px off, because the font prints its scale rounded to 2 decimals: 8 + 21.2*2.33 vs 8 + 21.2*2.3333; AA delta up to 43/255). Fixed with font::text_range, which draws a character range of a run laid out as the whole run: the composite of old bg_transport + new logo then renders byte-identical to the old bg_transport at 2x (0 differing bytes, checked with a temporary test, removed); a committed test (crates/rb-ui-art/tests/logo.rs) renders the old single-run drawing against the logo part at 1x/2x/3x and requires equality.
Screenshots (OpenRB-338 --demo --size 624x510 --screenshot, 1248x1064 px): /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task116/before.png (main 7ceaaa6) and /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task116/after.png. Compared with /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task116/cmp.py (own PNG decoder): 1203 differing pixels, all inside the logo box (px x 962..1199, y 64..123), max channel delta 2 of 255; no pixel outside the logo differs. The residue is the rounding of blending a separate texture over the background instead of rasterizing both in one pass (inherent to a separate part; the art itself is byte-identical).
About window: /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task116/about_2x.png (temporary local hook to open About at frame 1, not committed; the window is mid open-fade in the capture). It shows the wordmark at 2x native (268x96 pt), re-rasterized by the SVG loader at the window's density.
Pre-existing, not touched: --ppp 3 crashes before my change too (wgpu texture 11730 > 8192) with or without About; not investigated.
Tests: cargo test --release -q: 539 passed, 0 failed (RB_MOD_DIR set); cargo clippy --all-targets -D warnings clean; cargo fmt; rb-ui-art check: 167 parts x 3 scales, 0 problems. New tests: rb-ui-art/tests/logo.rs (2), rb-ui about::tests (1), the_logo in rb-ui layout.rs extended (logo box == on_logo rect). The existing backgrounds_match_the_art_generator test covers the logo placement.

Review round 1: split the 114-column raw string in tests/logo.rs with concat!; commit message says 'renders identical at 1x/2x/3x' (only the render was compared). The about.rs LOGO_W import was skipped: rb-ui has rb-ui-art only as a dev-dependency.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The wordmark is its own part: parts.rs logo() emits logo (134x48, the whole) and the pieces logo_open, logo_rb338, logo_line, logo_sub (each on the same 134x48 canvas, placed at (474,1), so an animation only offsets or fades them; LOGO_PIECES, LOGO_X/Y/W/H exported), and transport_bg no longer holds any wordmark. font::text_range lays a character range out as the whole run, so the pieces sit pixel-exactly where the single run did. The logo is a section background drawn after bg_transport (rb-ui-art layout.rs, rb-ui layout::backgrounds, part_size; on_logo unchanged and tested to equal the part's box). Help > About draws the logo part at 2x its native size (about.rs), crisp at any density since it is SVG; app_icon stays the window, Dock and package icon. assets/ui regenerated (5 new SVGs, bg_transport, panel.svg, previews) and README updated. Panel screenshot before/after: art byte-identical, whole-screen differences only inside the logo box, max channel delta 2/255 (texture blending rounding). Tests: cargo test --release 539 pass, clippy clean. Branch worktree-agent-adb71ac9cbb017b1f, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

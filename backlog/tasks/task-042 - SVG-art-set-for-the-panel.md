---
id: TASK-042
title: SVG art set for the panel
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 16:23'
updated_date: '2026-10-27 13:02'
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
- [x] #1 assets/ui/ holds SVG parts for every panel element in docs/ui_spec.md, with a README of parts and anchors
- [x] #2 No original artwork or third-party marks are used; the license is stated
- [x] #3 A render check shows every part at 1x, 2x and 3x (screenshots in the notes)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. New dev crate crates/rb-ui-art (lib + bin ui-art): builds every part as SVG text in code (port of the ui-poc builder: flat palette art::pal, knob/button/fader/switch/LED styles, 7-segment digits), with a built-in stroke font so no <text>/system fonts are needed (usvg in egui_extras has no guaranteed fonts).
2. Native coordinates = docs/ui_spec.md pixels (panel 624x510, each part's viewBox = its spec size); section backgrounds, knob body+pointer layers, faders, switches, buttons (rest/pressed/lit), LEDs, step buttons, digits, meters, selection overlays; logo is an own OpenRB-338 wordmark; no TB-303/TR-808/TR-909/ReBirth marks in the art.
3. 'ui-art gen' writes assets/ui/*.svg, layout.tsv (every control: part, panel x y w h from the spec) and a composed panel.svg; 'ui-art check' renders every part at 1x/2x/3x with resvg (the renderer egui_extras uses) into contact sheets + panel PNGs under assets/ui/preview.
4. cargo test: committed files equal generator output, every SVG parses/renders at 1-3x at exact size, every layout part exists, no <text>.
5. assets/ui/README.md: parts, anchors, composition rules (knob angle, arc, fader travel, digit frames), license (own work, MIT like the repo).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-27, per task-040: the art is drawn by egui through egui_extras' SVG loader (usvg + resvg), loaded at the displayed physical pixel size; check the parts render correctly with resvg (the same renderer).

Supervisor 2026-10-27, per the user: the visual look of the user's experiment is the base for the art set. It is crates/rb-ui-poc on branch worktree-ui-poc (commit 1359e39; not on main): an egui/eframe 0.37 front panel redrawn in a flat style from SVG generated in code (src/art.rs, src/sprites.rs, src/widgets.rs; design size 1200 x 940; palette in art::pal). Read it with `git show worktree-ui-poc:crates/rb-ui-poc/src/art.rs` etc. or the worktree .claude/worktrees/ui-poc. Keep its look (palette, flat style, knob/button styles) and lay the controls out per docs/ui_spec.md.

Branch: worktree-agent-a97606bb3e6dd1746 (agent-worker).

Work notes (2026-10-27):
- Built crates/rb-ui-art (lib + bin ui-art): ports the ui-poc look (art::pal palette, flat knobs/buttons/faders/switches, red 7-segment) into native-pixel parts. Deviations from the poc: poc drew everything at design size 1200x940 with its own layout; here every part's viewBox = the control's spec size (624x510 panel), layout from docs/ui_spec.md. The poc's direct resvg use is reconciled per task-040: the art is plain SVG files for egui_extras' SVG loader (usvg+resvg, size hint = native size x pixels_per_point); rb-ui-art uses resvg only for the render check.
- No <text> in any SVG: labels are paths of a built-in stroke font (src/font.rs), because usvg has no guaranteed system fonts (the poc's text relied on load_system_fonts).
- Visible texts are generic (BASS SYNTH, DRUM MACHINE A/B, PCF, DELAY, DISTORTION, COMPRESSOR, wordmark OPENRB-338 from task-068): no ReBirth, Propellerhead, TB-303, TR-808, TR-909 or Roland marks in the art. The poc's 'ReBirth RB-338' logo was replaced.
- Outputs (179 parts + panel.svg + layout.tsv, 353 lines = 351 controls) in assets/ui/; README.md has parts, anchors, knob layers (body / app-painted arc / rotating pointer, angle -135+270v), fader travel, digit frames, license. Regenerate: cargo run --release -p rb-ui-art -- gen; check: ... -- check.
- Render check (resvg 0.49, the renderer of egui_extras): 'cargo run --release -p rb-ui-art -- check' -> '179 parts x 3 scales checked, 0 problems' (each part parses, renders at exactly w*s x h*s for s=1,2,3, has visible pixels). Screenshots (committed): assets/ui/preview/panel_{1x,2x,3x}.png (624x510, 1248x1020, 1872x1530: the composed panel) and assets/ui/preview/parts_{1x,2x,3x}.png (700x1245, 1400x2490, 2100x3735: sheets of every part with its name). I viewed panel_2x/3x against docs/ui_spec/panel.png: sections, rows, rack columns, transport, step strips and legends sit where the original's do.
- Layout correctness is tested, not eyeballed: tests/art.rs parses every control row of docs/ui_spec.md and requires a layout control at the same rectangle (x/W,y/H,w/W,h/H fractions x 624/510, 0.06 px tolerance); only grids, step strips, areas and the 63 px gain-reduction meter (9 LED parts) are exempt (composite controls). cargo test -p rb-ui-art: 4 pass (generator == committed files, render 1x-3x, spec rectangles, inside panel). clippy -D warnings and fmt clean.
- Not tried: running the SVGs through egui_extras itself (no egui dependency here; task-044). Meter scales and the 909 bank row remain [G] in the spec; the art draws 11 meter states and 4 equal bank cells.
- Cargo.lock gains resvg/usvg/tiny-skia and deps; workspace member added; crate name rb-ui-art follows the current prefix (task-068 renames).

Correction: layout.tsv has 352 controls (353 lines with the header), not 351.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added the SVG art set under assets/ui (179 parts, composed panel.svg, layout.tsv with 352 control rectangles, README with parts, anchors, composition rules and the license: own work, MIT, no Propellerhead/Roland marks or bitmaps) and the generator crates/rb-ui-art (ui-art gen/check; the parts are drawn in code from the ui-poc look at the spec's native 624x510 geometry, labels as stroke-font paths so no fonts are needed). Render check with resvg at 1x/2x/3x: 179 parts x 3 scales, 0 problems; screenshots assets/ui/preview/*.png. cargo test -p rb-ui-art 4 pass (including a test that every single-part control of docs/ui_spec.md sits at its specified rectangle), clippy and fmt clean. Branch worktree-agent-a97606bb3e6dd1746, one commit. Not run through egui itself (task-044).
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-179
title: 'Panel redesign: apply the approved mockup (mockup/179) to the art and the app'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-11-13T11:21:00Z'
updated_date: '2026-11-13T13:35:00Z'
labels:
  - ui
  - docs
milestone: m-5
dependencies: []
references:
  - mockup/179/mock.py
  - mockup/179/anim.py
  - mockup/179/reference/panel_2x.png
  - mockup/179/reference/panel_wide_window_2x.jpg
  - mockup/179/reference/logo_animation_main_frames.jpg
  - mockup/179/reference/logo_animation_about_frames.jpg
  - mockup/179/fonts/LICENSE.txt
  - crates/orb-ui-art/src/parts.rs
  - crates/orb-ui-art/src/layout.rs
  - crates/orb-ui/src/app.rs
  - crates/orb-ui/src/logo_anim.rs
  - crates/orb-ui/src/about.rs
documentation:
  - mockup/179/README.md
  - assets/ui/README.md
  - docs/ui_spec.md
priority: medium
type: enhancement
ordinal: 195000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-11-13) for the panel to be redesigned to match the original ReBirth RB-338 2.01 panel as closely as possible, as scalable SVG. The current art (task-042, task-085, task-115) is a flat look of orb's own with its own control placement. A mockup was drawn and reviewed with the user in many rounds on 2026-11-13 and approved; it is committed, with everything needed to apply it, in the temporary folder mockup/179/. This task ports that mockup into the art generator and the app and removes the folder.

Start with mockup/179/README.md. It is the detailed instruction: the user's decisions, the design section by section, what to change in crates/orb-ui-art and crates/orb-ui, the documents to update, the checks, and where each file of the folder goes. mockup/179/mock.py is authoritative for every position, size, color, gradient and filter; mockup/179/reference/ holds the approved renders. The README's commands reproduce reference/panel_2x.png byte for byte.

The user's decisions (2026-11-13). They are settled; do not reopen them:
- SVG stays the foundation; no pre-rendered PNG art.
- Layout: the original's sections and control places (docs/ui_spec.md sections 3 to 9), not the placement of task-085 and task-115. Where mock.py differs from the spec's tables, mock.py wins (README section 4 lists the differences).
- Kept from the current design: the mod's splash and name where the original has the 808's selector wheel, and the animated OPENRB-338 wordmark at the top right (same seven pieces, same animation, restyled).
- Knobs: flat-topped, seen from above, static body and a turning pointer, short cast shadows, no value arc.
- Pressed push buttons: "variant F, dimmed": the bevel and the cast shadow go, the face turns flat and a little lighter with a faint gradient.
- Routing keys (DIST, PCF, COMP, the master's COMP): never drawn pressed while on; only the lamp lights.
- Font: Inter 4.1 (SIL OFL 1.1; Regular, Medium, Bold), converted to outlines when the art is generated, so the shipped SVGs hold no text and the app needs no font.
- Help > About: only the wordmark sits on a gray speckled plate; the rest of the window keeps the app's dark colors and the flat icon.
- Wide (and tall) windows: a dark desk surface with the unit's cast shadow around the panel, in place of the flat letterbox color.
- The Sync and MIDI LEDs and labels are drawn, dark, as background art.
- Texts stay generic (PATTERN, MIX, MASTER, PCF, DELAY, DIST, COMP; no product name, no Roland or Propellerhead mark).

What the mockup does not settle, for this task to work out and report:
- Control states the mockup does not show (README section 3 lists them: transport keys pressed, Record lit, pattern mode, and others) are derived in the same style and shown to the user on the parts sheet.
- Parts now draw outside their control rectangles (shadows, halos, glows), which the app's "a part is exactly its control's rectangle" rule does not allow; README section 5 item 4.
- The header meters have four segments, the engine's meter has 11 states; the mapping is this task's to choose and document.
- Cost: the new art uses blurs and noise filters and the app rasterizes every part again at each new pixel size. Measure start and resize before and after. If only baked bitmaps would keep the app responsive, do not switch: file a Decide task for the user (they chose SVG) and make this task depend on it.

Moving the files (README section 9): the three Inter TTFs and their LICENSE.txt go to crates/orb-ui-art/fonts/; reference/panel_2x.png goes to docs/ui_spec/redesign_mockup_2x.png and is linked from docs/ui_spec.md section 19; the README's lasting content goes into assets/ui/README.md and docs/ui_spec.md section 19; everything else in mockup/179/ is deleted. After the task's commit there is no mockup/ folder.

Out of scope: the app icon (assets/ui/app_icon.svg, the .ico and the icon set stay), the menus and the other windows (File > Info, Mods > Info, the file browser), the engine, and any new feature for the space beside the panel (docks for songs or mods were discussed and set aside).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 crates/orb-ui-art generates the redesigned art: `ui-art gen` then `ui-art check` report no problems, the committed assets/ui equals the generator's output, and no file of assets/ui contains a <text> element (labels are Inter outlines made at generation time from the fonts in crates/orb-ui-art/fonts, with no system font involved)
- [x] #2 The generated panel matches the approved mockup: assets/ui/preview/panel_2x.png next to the mockup's reference/panel_2x.png shows every control, label and section at the same place within half a panel pixel and in the same look; the differences that remain are listed in the notes with the reason
- [x] #3 The app draws the new panel at 1x and 2x with every control usable as before: hit areas follow the drawn controls, behavior is that of docs/ui_spec.md, shadows, halos and glows that reach outside a control's rectangle are not clipped, pressed push buttons use variant F, and the routing keys show only their lamp when on; screenshots in the notes
- [x] #4 The wordmark's startup animation plays as before on the panel and in Help > About with the new pieces, and About shows the wordmark on the gray plate with the rest of the window unchanged; ORB_LOGO_T screenshots at the times of the reference strips are in the notes
- [x] #5 In a window wider or taller than the panel's shape the desk backdrop with the unit's cast shadow is drawn around the panel; a resize drag does not rasterize it again per size
- [x] #6 Cold start to the first complete frame and a resize drag are measured before and after in a release build at 1x and 2x, the numbers are in the notes, and neither is clearly worse; if that cannot be reached with SVG, a Decide task exists and this task depends on it
- [x] #7 The states the mockup does not show (README section 3) are drawn in the same style, named in the notes, and visible on assets/ui/preview/parts_2x.png for the user's review; the header meters' mapping from the engine's 11 states to four segments is documented in docs/ui_spec.md
- [x] #8 assets/ui/README.md and docs/ui_spec.md section 19 describe the new art (parts, sizes, bleed, knob layers, font and its license, a true origin statement); docs/ui_app/panel_*.png, assets/readme/screenshot.png and assets/readme/logo.gif are made again from the app
- [x] #9 The files of mockup/179 are moved as README section 9 says (fonts and license to crates/orb-ui-art/fonts, reference/panel_2x.png to docs/ui_spec/redesign_mockup_2x.png) and the mockup/ folder no longer exists
- [x] #10 cargo test, cargo clippy --all-targets -- -D warnings and cargo +nightly fmt --all --check pass; the tests with positions, part sizes or part counts in them are updated, not removed
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Measure before (main 080f0a8, release, this Mac): headless first frame and a 40-frame resize drag at ppp 1 and 2 (new ignored test crates/orb-ui/tests/startup_resize_cost.rs), and the real app's cold start to its --screenshot (8 frames) at --ppp 1 and 2.
2. orb-ui-art: port mock.py. svg.rs gets a defs table (every gradient and filter of mock.py; a part's SVG carries only the defs its body refers to) and a symmetric bleed per part (viewBox -b -b w+2b h+2b; the control rectangle stays (0,0,w,h)). New text.rs turns each label into Inter outlines with usvg (only the three bundled TTFs in the font database), so no part has <text>. parts.rs is rewritten section by section from mock.py; the grain of each background is laid in panel coordinates (one part per instance where the noise differs: bg_pattern_1..4, bg_mixer_1..4, wood_left/right), so the composed panel reproduces the mockup's grain. layout.rs moves every control to the mockup's places; compose() uses the demo state of README section 3.
3. Derived states (README section 3) drawn in the same style and shown on preview/parts_2x.png; header meter mapping 11 -> 5 documented.
4. orb-ui: build.rs reads each part's bleed from its viewBox; paint_part draws the art rectangle (control rectangle grown by the bleed), hit tests stay on the control rectangle. Knobs without arc, new display cell placement (centered), drum strips, selection overlays (808 chip ring, 909 lit chip label and LEDs), mod window, rest LED, header meter mapping, routing keys lamp-only, variant F pressed parts. Backdrop: app-painted gradient and vignette meshes, a 256-pt stitched grain tile drawn repeated, and shadow strips as vertex-colored meshes (nothing rasterized per window size). About: gray plate part under the wordmark, icon at y 28, rows 6 lower.
5. Cost: rasterize the parts missing in a frame in parallel at the end of the frame (placeholder handles set before the frame is drawn), and during a resize draw the textures of the old size until the size settles (0.3 s). Measure after; if still clearly worse, file the Decide task (no baked bitmaps).
6. Docs: assets/ui/README.md, docs/ui_spec.md section 19 (+ link to docs/ui_spec/redesign_mockup_2x.png), README.md / docs/building.md where they describe the art; screenshots docs/ui_app/panel_*.png, assets/readme/screenshot.png, assets/readme/logo.gif from the app.
7. Move fonts and the reference render (git mv), delete mockup/; tests updated; cargo test, clippy, nightly fmt; rebase; one commit.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: task-179-panel-redesign

AC6 before (main 080f0a8, release build, this Mac M-series, scratch measure.sh):
- headless (startup_resize_cost, Panel::demo, window 624x534 pt): ppp 1: first frame 15.5 ms, resize drag 40 frames (+3x2 pt each) 283.7 ms (mean 7.1, max 9.1); ppp 2: first frame 171.0 ms, drag 611.1 ms (mean 15.3, max 18.6).
- app cold start to the --screenshot exit (OpenRB-338 --demo --size 624x510 --ppp N --screenshot, 8 frames; runs 2-5, run 1 had a cold disk cache 2.92 s): ppp 1: 1.54, 1.32, 1.30, 1.26 s; ppp 2: 0.45, 0.49, 0.43, 0.47 s (0.48 first).

Progress (art): orb-ui-art ported from mock.py. svg.rs: defs() holds every gradient/filter of the mockup (unused variants of the rounds dropped: gSilverP, gPillLit, gPillFlatSoft, gPillP, gShade*, sphere knob gradients, gLogo*), a part's file carries only the defs its body names (defs_for), Part has a symmetric bleed (viewBox -b -b w+2b h+2b). text.rs: labels laid out by usvg 0.49 with only the three bundled Inter TTFs in the fontdb, flattened to path data (2 decimals), cached. Grain per instance in panel coordinates (translate(-ox -oy) inside the clip), so bg_pattern_1..4, bg_mixer_1..4, wood_left/right reproduce the mockup's speckle exactly.
Reference render check (scratch diffpanel.sh: ui-art gen into scratch, ui-art check, PIL diff of preview/panel_2x.png against docs/ui_spec/redesign_mockup_2x.png, which is mockup/179/reference/panel_2x.png moved): first pass 2.00 % of pixels > 8 (mean 2.40), almost all the mod window's splash (panel.svg has no picture: the app paints it). Excluding the splash and the mod name: 0.28 % > 8, 0.08 % > 32, mean 0.09.
Fixes found by the diff: (1) the 808's tone/tune knob shadows lay over the LC/MC/HC chips (the mockup draws the chips after the knobs) and the PCF's FREQ handle shadow over its caption: new foreground parts fg_tr808 (the 5 upper chips) and fg_pcf (the 4 captions), drawn after the controls; (2) the 909's lower knob labels at y 50.4 (not knob y - 3.8).

AC6 after, and what it took (release, this Mac; scratch scripts measure.sh, coldstart.sh, firstframe.sh):
- First after-measurement was far worse: headless ppp 2 first frame 6122 ms, app at --ppp 1 13 to 25 s to the screenshot. Cause (traced with temporary prints): eframe's and the headless context's FIRST frame has a bogus content area (20000 x 20000 points in the app on macOS, 5000 x 5000 headless at ppp 2), so every part was rasterized at the 8192 texture cap; with the flat art that was cheap, with the filters seconds. Fix: the window's first frame rasterizes nothing (track_resize, first_frame_id).
- Per-part cost at 2x, one core (orb-ui-art test part_cost, ignored): total 1050 ms; desk_tile 194, bg_transport 99, bg_tr808 90, about_plate 78, bg_tr909 64, then the gray backgrounds 25 to 35 each. The parallel flush is bound by the biggest part. desk_tile now rasterized once at 1x and only when the desk shows.
- Fine speckle (grD, grL) as a stitched 64-px pattern tile (pgrD, pgrL): about 20 % off the backgrounds (bg_transport 99 -> 80 ms, total 1050 -> 902 ms). Visually the same kind of speckle; diff vs the mockup grows to 18.4 % > 8 (only specks).
- Failed idea: rasterizing big parts in horizontal stripes in parallel. resvg 0.45 sizes a filtered group's layer by fit_to_rect(bbox, 4x canvas) (render.rs), so a stripe still computes the noise over up to 5 stripe heights: no gain. Not done.
- Maximized window still +0.5 s to the first frame with full-size rasters -> progressive start: when a frame misses more than 8 parts, parts denser than 1.25 px per panel px are rasterized first at 1 px per panel px (synchronously, in parallel) and at full size on a background thread that sets the handles and asks for a repaint (PREVIEW_DENSITY, PREVIEW_MIN_PARTS; off for --screenshot and where there are no threads).
- New developer variable ORB_UI_START_STATS=1 (docs/building.md); main's build got the same print as scratch-only instrumentation for the comparison.
Numbers, before (main 080f0a8) vs after, interleaved runs:
- first complete frame from process start (ORB_UI_START_STATS, 5 runs each): maximized 391/437/450/419/372 ms vs 426/459/485/505/438 ms (all parts sharp at 735 to 993 ms); --size 624x510 --ppp 2: 529/417/369/566/366 vs 555/510/418/623/399; --ppp 1 (on this Retina display the window is 1248 px, so the panel is at 2x): 1324/912/1111/1185/879 vs 614/433/509/500/484.
- start to the --screenshot exit (8 frames, full size, no preview; 6 runs): --ppp 1 1.43-2.26 s vs 0.64-1.12 s; --ppp 2 0.48-0.58 vs 0.58-0.65; maximized 0.54-0.60 vs 0.83-0.94 (the screenshot waits for the full sizes).
- headless (startup_resize_cost, 624x534 pt): ppp 1: first complete frame 15.5 vs 50.0 ms, resize drag of 40 frames 283.7 ms (mean 7.1, max 9.1) vs 52.4 ms (mean 1.3, max 40.1: the desk tile appears once), one rasterizing when the size settles 63 ms; ppp 2: 171.0 vs 37.8 ms (preview; full size on a thread), drag 611.1 ms (mean 15.3, max 18.6) vs 85.5 ms (mean 2.1, max 67.0), settle 128 ms.
Verdict: the first complete frame is within about 0.05 to 0.1 s of before (better at --ppp 1); the full-sharp panel comes 0.3 to 0.5 s later than main's first frame in a maximized window; a resize drag is much cheaper than before. Not clearly worse, so no Decide task.

AC7: states the mockup does not show, derived in the same style (all on assets/ui/preview/parts_2x.png under their part names):
- transport keys pressed: btn_play_pressed, btn_stop_pressed, btn_rewind_pressed, btn_ff_pressed, btn_record_pressed (variant F on the key face: gPillFlatLit, faint inner line, no highlight, no lower shade);
- Play at rest btn_play_rest (no halo, darker green #23891c); Record lit btn_record_lit (red #ff3b2c with haloR, like Play's green);
- arrows pressed arrow_inc_pressed, arrow_dec_pressed (variant F);
- every 303 button pressed: btn_pill_15x8_pressed (pitch), btn_pill_17x10_pressed (clear, note/pause, back), btn_step_pressed, key_white_pressed, key_black_pressed, btn_flag_pressed (variant F);
- loop switch off toggle_loop_off (lever up, LED dark); pattern mode switch_mode_0 (lever up, green Pattern LED lit, yellow Song LED dark); in pattern mode the song displays show seg_blank (model returns None);
- slide switches in both positions: switch_808_0/1, switch_fx_0/1, switch_wave_0/1;
- master meters' 11 states meter_15x56_0..10; header meters meter_14x14_0..4, mapping [0,1,1,1,2,2,2,3,3,4,4] in docs/ui_spec.md section 19 (model::header_meter_segments);
- gain-reduction LEDs lit gr_led_green/amber/red, unlit in their place's color gr_led_off_g/y/r (GR_OFF: r r y y g y y r r);
- 909 running light: no new part; the model shows a set step as step909_0 and an empty one as step909_2 (unchanged rule);
- the flam button off toggle_flam_off, every LED off/on (led_303_*, led_303s_*), every selection mark (sel808_* for the 12 legend and 5 upper chips, sel909_* for the 12 instruments, led909_on, led909ac_on).
New control: tb303_N.rest_led (the original's Rest LED, docs/ui_spec.md section 5) beside note_led.

AC2: generated preview/panel_2x.png against the approved render (docs/ui_spec/redesign_mockup_2x.png, which is mockup/179/reference/panel_2x.png moved), PIL diff (scratch imgdiff.py, mod window and its name left out): before the speckle tile 0.28 % of pixels > 8 of 255, 0.08 % > 32, mean 0.09; final (fine speckle as a tile) 18.42 % > 8, 1.94 % > 32, mean 4.36, the difference being specks only (crop comparisons c1..c3 in scratch). Differences that remain, with the reason:
1. The mod window's picture and name: panel.svg has none (the app paints the kit's splash and name; the app screenshots show them).
2. The fine speckle is a 64-px repeating tile (start cost, AC6), so its specks lie elsewhere.
3. Gain-reduction LEDs: the mockup lit LEDs 1-5 in fixed colors by place (G G G Y Y); the app lights them in the link's colors (section 16: center green, amber then red to its left), so the demo shows R R Y Y G; unlit LEDs show their place's color.
4. 808 SD's second knob labeled TONE (mockup: TUNE); the control is SD tone (docs/ui_spec.md section 6).
5. The PCF and delay lamp halos tint the top of the display frame below them slightly (the mockup draws the frame after the lamp; here the frame is background, the lamp a control).
6. Label antialiasing differs by a few levels (outlines vs the mockup's text rendering).
7. The 808's tone knobs' shadows and the PCF's lowest handle: fixed with foreground parts (fg_tr808, fg_pcf), so no difference left there.

Screenshots (AC3, AC4, AC5), from the release app (scratch shots.sh, anim_shots.sh, anim_compose.py):
- committed: docs/ui_app/panel_312x255.png (--size 312x255 --ppp 1: the panel at 1x), docs/ui_app/panel_624x510.png (2x), docs/ui_app/panel_1000x600.png (2x, wide window: the desk and the unit's shadow), assets/readme/screenshot.png (1100x700 at 2x), assets/readme/logo.gif (56 About frames, ORB_LOGO_T = i/30 s, 740x216, played once);
- scratch: /private/tmp/claude-501/-Users-pingw33n-dev-openrb-338/6e5ae53b-ec99-4222-b796-08716b3bbf16/scratchpad/task-179-panel-redesign/final_tall_2x.png (tall window: the desk above and below), /private/tmp/claude-501/-Users-pingw33n-dev-openrb-338/6e5ae53b-ec99-4222-b796-08716b3bbf16/scratchpad/task-179-panel-redesign/app_wide_2x.png, /private/tmp/claude-501/-Users-pingw33n-dev-openrb-338/6e5ae53b-ec99-4222-b796-08716b3bbf16/scratchpad/task-179-panel-redesign/final_about_2x.png (About: the wordmark on the gray plate, the rest of the window unchanged), /private/tmp/claude-501/-Users-pingw33n-dev-openrb-338/6e5ae53b-ec99-4222-b796-08716b3bbf16/scratchpad/task-179-panel-redesign/anim/strip_panel.png and /private/tmp/claude-501/-Users-pingw33n-dev-openrb-338/6e5ae53b-ec99-4222-b796-08716b3bbf16/scratchpad/task-179-panel-redesign/anim/strip_about.png (the panel's and About's wordmark at 0, 0.15, 0.35, 0.6, 0.9, 1.2, 1.5, 1.85 s: RB-338 in place, OPEN sliding out from behind it, the bars fading in left to right, the subtitle last, the icon fading in About), /private/tmp/claude-501/-Users-pingw33n-dev-openrb-338/6e5ae53b-ec99-4222-b796-08716b3bbf16/scratchpad/task-179-panel-redesign/gen/preview/parts_2x.png (the parts sheet).
Hit areas: the control rectangles of layout.tsv (the parts' bleed is drawn only); drum strips y 90..110 (808) and 85..103 (909), the 909 name row selects in y 74..84; the interaction, keyboard, wired, switch_values and menu tests pass on the new layout.
Tests: cargo test (workspace) 688 passed, 0 failed, 9 ignored (after fixing part_textures for the desk tile: the first run had 687 passed, 1 failed); cargo clippy --all-targets -D warnings clean; cargo +nightly fmt --all --check clean. Lines over 100 columns in the new code were split by hand (stroke_path, sk helpers in parts.rs; concat! in svg.rs).

Review round 1 (send back, one defect): crates/orb-ui-art/src/parts.rs labeled the 808's SD tone knob TONE where mock.py has TUNE. The user (@pingw33n) settled the knob labels on 2026-11-13: on the 808 every tune/tone knob reads TUNE except BD and CY (TONE); on the 909 every one reads TUNE except SD's second-row knob left of SNAP (TONE). Checked every label: 808 row 2 now BD TONE, SD TUNE, LT/MT/HT TUNE, CY TONE; 909 row 1 TUNE on BD..HT, row 2 SD TONE, CC/RC TUNE (ATT, DEC, SNAP, LEVEL are not tune/tone). Only the 808 SD label changed (parts.rs row2 list). docs/ui_spec.md: section 6 table row 'SD tone knob (labeled TUNE, section 19)', section 19 gets the user's rule as a paragraph and the AC2 difference item about the SD label is removed (AC2 difference 4 of the earlier note is withdrawn: the label now matches the mockup, checked by a crop of preview/panel_2x.png against the mockup render). Regenerated: ui-art gen and check (202 parts, no problems; bg_tr808.svg, panel.svg, preview/*), docs/ui_app/panel_*.png, assets/readme/screenshot.png and logo.gif (scratch shots.sh).

Review round 1 checks: cargo +nightly fmt --all --check clean, cargo clippy --all-targets -- -D warnings clean, cargo test 688 passed, 0 failed, 9 ignored; ui-art check 202 parts x 3 scales, 0 problems.

Review round 2 (the user tried the .dmg of 19b53f8, 2026-11-13), two requests:
1. The effects' on/off lamps (PCF, DELAY, DIST, COMP) as big as the mixer's: onoff() in crates/orb-ui-art/src/parts.rs now takes the lamp size; power_mix and power_fx are 9.6 square (power_fx was 16 - 4.4 = 11.6), power_pat stays 8.6. The control stays 16 x 16 (hit area unchanged), the lamp centered in it; the halo scales with the lamp as the mixer's does.
2. The routing keys (mixer DIST, PCF, COMP, the master's COMP) show pressed only while the mouse button (or a held computer key) is down on them. Reused BACK/CLEAR's mechanism: app.rs already passes 'down' (pointer down on the control or key held) to Panel::part_name for every clickable kind; part_name now appends _pressed to the on/off name when the family is mixsw_ and pressed, so the key never stays down and the lamp alone shows the state. New parts mixsw_off_pressed and mixsw_on_pressed (variant F pill, lamp off/on); they appear on preview/parts_*.png with the other derived states. Unit test model::routing_keys_show_pressed_only_while_held (rest, held, after a click; mute and effect lamps have no pressed look).
Docs: docs/ui_spec.md section 19 (the routing keys' and the lamps' paragraph, the derived states, a difference from the mockup), assets/ui/README.md (mixsw_*_pressed, the lamp sizes).
Regenerated: ui-art gen and check (204 parts x 3 scales, 0 problems), docs/ui_app/panel_*.png, assets/readme/screenshot.png and logo.gif.
Checks: cargo +nightly fmt --all --check clean, cargo clippy --all-targets -- -D warnings clean, cargo test 689 passed, 0 failed, 9 ignored.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The panel redesign approved as mockup/179 is in the art generator and the app (branch task-179-panel-redesign, one commit).
- orb-ui-art: mock.py ported. Shared gradients and filters in svg.rs (a part carries only the defs it uses), a symmetric bleed per part (viewBox -b -b w+2b h+2b), labels as Inter 4.1 outlines made with usvg from the three bundled TTFs (crates/orb-ui-art/fonts, OFL; no <text> in assets/ui). The speckle is laid in panel coordinates (one background per pattern row, mixer channel and cheek), the fine speckle as a 64-px stitched tile. New parts: foregrounds fg_tr808 and fg_pcf, selection marks sel808_*/sel909_*/led909*, about_plate, desk_tile, mod_view_glass, the 303's rest LED, the routing keys pressed (mixsw_*_pressed). Knob labels follow the user's rule of 2026-11-13 (808: TUNE except BD and CY TONE; 909: TUNE except SD's second-row TONE). 204 parts, ui-art check: no problems.
- orb-ui: build.rs reads the bleed and the control size from the viewBox; parts are drawn over the control rectangle grown by the bleed, hit tests stay on the control rectangle; knobs have no arc; centered display cells; new strips, selection marks, mod window; header meters map the 11 states to 4 segments [0,1,1,1,2,2,2,3,3,4,4]; routing keys show pressed only while held, their lamp shows the state; variant F pressed parts; the desk backdrop (crates/orb-ui/src/backdrop.rs: meshes and a 1x tile, nothing rasterized per size); About with the wordmark on a gray plate.
- Cost: missing parts are rasterized in parallel at the end of the frame, the first (bogus-size) frame rasterizes nothing, a big miss shows a 1x preview first and the full size from a background thread, a resize draws the old textures until the size settles for 0.3 s. First complete frame within about 0.05 to 0.1 s of main (better at --ppp 1), sharp 0.3 to 0.5 s later in a maximized window; resize drag 284/611 ms -> 52/86 ms (headless, 1x/2x, 40 frames). No Decide task.
- Against the mockup: every control, label and section in place; differences listed in the notes and docs/ui_spec.md section 19 (app-painted splash, tiled speckle, GR LED colors by the link's rule, lamp halo on the PCF/delay frames, antialiasing, the user's later requests: effect lamps as big as the mixer's, routing keys pressed while held).
- Review round 1: the 808 SD knob label fixed to TUNE by the user's label rule. Review round 2 (the user's requests after trying the .dmg): the effects' on/off lamps 9.6 square like the mixer's; the routing keys pressed while held, through BACK/CLEAR's pressed mechanism.
- Docs: assets/ui/README.md, docs/ui_spec.md sections 6 and 19 (+ redesign_mockup_2x.png), README.md, assets/README.md, docs/building.md (ORB_UI_START_STATS); docs/ui_app/panel_*.png, assets/readme/screenshot.png and logo.gif remade from the app. mockup/ removed.
- Tests (round 2): cargo test 689 passed, 0 failed, 9 ignored; clippy -D warnings and nightly fmt --check clean.
<!-- SECTION:FINAL_SUMMARY:END -->

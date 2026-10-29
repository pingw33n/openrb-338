---
id: TASK-115
title: 'rb-ui: UI clean-up pass with the Fable model (interactive with the user)'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 11:57'
updated_date: '2026-10-29 10:08'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - assets/ui/README.md
  - docs/ui_spec.md
  - crates/orb-ui-art/src/parts.rs
  - crates/orb-ui-art/src/layout.rs
priority: low
type: enhancement
ordinal: 135000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user wants to try cleaning up the UI's look with the Fable model (claude-fable-5-1), in an interactive design session the user drives (2026-10-28). Agents do not start it on their own.

Scope, to be set in the session: the panel art and layout (crates/orb-ui-art: parts.rs, layout.rs, font.rs; regenerated assets/ui with `cargo run --release -p orb-ui-art -- gen`), and the egui windows and menus (the file browser, Song Information, Mods > Info, About, Keyboard Shortcuts, prompts). Examples: spacing and alignment, label sizes and colors, consistency between sections, window styling.

Constraints:
- The panel's controls keep their ids and behavior: assets/ui/layout.tsv drives the app, and the wired and interaction tests must still pass.
- Each change is reviewed by the user in the running app, with screenshots via OpenRB-338 --screenshot.
- Write the design decisions into assets/ui/README.md and docs/ui_spec.md.
- For the session, run Claude Code on Fable (/model, or claude --model claude-fable-5-1). Compare its suggestions against the current look before applying them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The user ran the session with the Fable model and accepted the changes in the running app
- [x] #2 The art is regenerated, the wired/interaction tests pass, and assets/ui/README.md and docs/ui_spec.md record what changed
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Plan (spec: Panel art of mockups/HANDOFF.md, approved 2026-10-29):
1. orb-ui-art/parts.rs: lift led_3d, keycap, key_3d, mixsw_body, header3d, shade from the mockup generator; redraw power_13x13/14x14, switch_wave (40x14), mixsw (28x13), pat/bank keys (14x15), shuffle keycap, btn_pill (renamed 22x10), white/flag keys with 3D, headers 3D (pattern rust, mixer sea), tb303/pattern/mixer/master/fx backgrounds, key band consts, wordmark with four underline segments (logo_line_0..3 replace logo_line; LOGO_PIECES = open, rb338, line 0..3, sub), new app_icon.
2. layout.rs: the moved controls (303 knobs/keys/flags/edit step, pattern keys, mixer routing keys, master COMP).
3. Regenerate assets/ui (gen), assets/app_icon.ico (ui-art ico), update tests that assert positions/sizes (art tests, logo test), About window layout in orb-ui/about.rs (icon, wordmark, version, link; no name line).
4. docs/ui_spec.md and assets/ui/README.md updated.
5. Screenshot orb-ui --demo vs mockups/panel_after.png, list differences. Run fmt, clippy, tests, gen no diff, xwin clippy.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The user's list for the session (2026-10-28):
- Rewrite the app icon (the app_icon part in crates/orb-ui-art/src/parts.rs; it feeds the window icon, the .icns of task-110, the AppImage of task-112 and the installer of task-111).
- What the user does not like:
  - TB-303 layout: there is empty space; the 'EDIT STEP' placement doesn't feel right; the leftmost section (pitch mode, clear) has too much padding.
  - Pattern number and bank buttons: they look too bare without any style, and are maybe too big and too dense.
  - The on/off buttons in the panel headers: too simplistic, and the 'on' color is too bright. It is also perceived differently depending on the header color (on the red header vs the dirty-green one).
  - The channel header color: could be better (too dirty).
  - The pattern header color: too bright, though the user likes that color on the 808's knobs and buttons.
  - In the channel (mixer) panels: the placement of the DIST, PCF and COMP buttons feels off.

User, 2026-10-29: the UI change set below was approved in the mockup session and is to be implemented by an agent. Reference renders are in the main checkout's mockups/, untracked; the user will delete it after tasks 114 and 115 merge, so this text is the record. The generator mockups/gen/src/main.rs links crates/orb-ui-art and has every shape as Rust.

# UI changes approved in the mockup session (2026-10-29)

The user reviewed six rounds of mockups and approved the set below. It is the content of
task-115 (the clean-up pass; everything under "Panel art") and task-114 (the startup animation;
"Animation"), plus the new app icon that task-115 lists. All numbers are panel pixels unless
said otherwise. The mockups are in `mockups/` (untracked): `panel_after.png` is the whole panel
as approved, `round6.png`, `waveform.png`, `icons.png`, `wordmark.png`, `about.png` and the
`anim_*.gif` files are the per-area reviews, and `mockups/gen` is the generator that drew them
(it links `crates/orb-ui-art`, so every shape below exists as Rust in `gen/src/main.rs` and can
be lifted into `parts.rs` and `layout.rs` nearly as is: `led_3d`, `keycap`, `key_3d`,
`mixsw_body`, `header3d`, `tb303_bg`, `pattern_bg`, `mixer_bg`, `master_fx_bgs`, `mark_body`,
`relayout`, `anim_at`, `wordmark_anim`).

## Panel art (task-115)

### 303 (both)

- Waveform: "WAVEFORM" (cap 3.6, ink) centered at (35, 5.5) like a knob label. One switch
  `switch_wave_0/1` of 40 x 14 at (15, 20): black frame (radius 3) with a `#26282b` inset
  (0.6), the saw and square glyphs printed in INK on the frame (6 px tall, saw at x 2..9.5,
  square at 30.5..38, centered at y 7), an 18 x 12 track `#0e0f10` at x 11, the thumb 8 x 11
  steel over a `#8c929b` lip (1.5 short) at x 12 (saw) or 20 (square), with a 4 px dark line.
  The bare glyphs on the paper go away.
- Knobs centered: pitch 40, first knob at x 95 (centers 110 + 40 k), labels at the centers.
- Key band: three blocks of one height, y 46..112, radius 4: pitch/clear black at
  (12, 46, 34, 66); keys gray `#7d8087` at (52, 46, 184, 66); flags black at (242, 46, 112, 66).
- Pitch/clear: "PITCH" at (29, 51.5), LED 8 x 8 at (25, 56), pitch pill at (18, 66); "CLEAR" at
  (29, 88.5), clear pill at (18, 95). The pill is 22 x 10 (was 17 x 10).
- Keys: white keys at x 66 + 21 i (y 89), their LEDs at x + 0.5 (y 78); black keys 11 px
  right of the white key to their left (y 60), LEDs at the same x (y 49).
- Edit step: the display 19 x 16 at (296, 50) inside the flags block, its arrows at (316, 49)
  and (316, 58), "EDIT STEP" (cap 3.2, INK) centered at (279.5, 58).
- Flags at x 250.5 + 22 j: LED y 78, key y 89, label y 109.
- 3D on the pills, white keys and flag keys: a darker base rect, the face inset 0.5 at the
  sides and 1.5 short at the bottom (white key base `#c4c7cb` under WHITE; flag key and pill
  base `#a4aab2` under STEEL). The black keys are unchanged.

### Pattern section

- Header rust `#bf4a2b`, 3D (see Headers).
- Pattern keys 14 x 15 at x 4 + 16 i, rows y 25 and 43; bank keys the same shape at y 64. A
  keycap is a face over a 1.8 px darker lip with a 0.5 px `#00000055` outline, radius 2.2,
  digit/letter cap 5.5 at (7, 6.8). Pattern: `#dde1e6` on `#a7afba`, black digit; on `#e8503f`
  on `#9c2b25`, white digit. Bank: `#4e5a6b` on `#2c3542`, INK_DIM letter; on ORANGE on
  `#a85d14`, black letter.
- SHUFFLE 30 x 12 is the same keycap (`#dde1e6`/`#a7afba`, label cap 3.2 at (15, 5.3); on
  `#e8503f`/`#9c2b25`, white label). Its position and the steps display are unchanged.

### Mixer channels

- Header sea `#3b8a8f`, 3D.
- Pan knob at (8, 26), delay send at (40, 26), labels "PAN" (19, 52.5) and "DELAY" (51, 52.5).
  Fader at (11.5, 58).
- Routing keys `mixsw_dist/pcf/comp` 28 x 13 at x 38, y 62 + 16 i, no well: base `#2a323d`
  (shows as a 0.6 rim and a 1.8 lip), face `#4b5767` with a `#ffffff0c` top light over the
  upper 4.6 px, the LED (below, r 1.8, amber) at (5.4, 6.2), the label cap 3.8 left-aligned
  at x 10.4, INK when on and INK_DIM when off.
- Master COMP: the same key at section (27, 82) (panel 553, 132).

### Headers (pattern, mix, master, PCF, delay, dist, comp)

`header3d`: a lip in the header color darkened to 0.62 at the full height, the face 1.3 px
shorter over it, a vertical gradient on the face (white 0.17 at the top, white 0.02 at 55 %,
black 0.16 at the bottom), and a 0.6 px white line at 0.18 opacity 0.5 px below the top, inset
2 px. Sizes and label positions unchanged.

### On/off LED (`power_13x13`, `power_14x14`) and the routing keys' LED

`led_3d(cx, cy, r, on, hue)`, no square bezel; r 2.6 for the headers, 1.8 on the routing keys:
- socket: a white ring at 0.17 (r + 1.3, 0.7 wide, shifted down 0.45) whose lower edge peeks
  out as the recess rim, a black circle at 0.38 (r + 1.4), an inner black ring at 0.31
  (r + 0.9, 0.8 wide, shifted up 0.15);
- halo when on: the lit color at 0.14, r + 2.6;
- lens: a radial gradient (cx 0.38, cy 0.32, r 0.75). Green on: `#c8f8cc` 0, `#6fe07e` 0.3,
  `#38b84d` 0.75, `#1f7a30` 1. Green off: `#4f6a55` 0, `#2f4536` 0.5, `#1c2a20` 1. Amber on:
  `#fff0b8`, `#ffd04a`, `#f0a300`, `#9a6400`. Amber off: `#5d5340`, `#3b3526`, `#231f14`;
- gloss: a white ellipse (rx 0.45 r, ry 0.28 r) at (cx - 0.25 r, cy - 0.45 r), opacity 0.55 on
  and 0.18 off.

### Wordmark (transport, `logo` / `logo_line`)

The orange underline becomes four segments in the 808 step colors: at (8 + 29.9 i, 28),
28.4 x 1.5, radius 0.75, RED, ORANGE, YELLOW, WHITE. Nothing else changes and no mark is
added to the bar.

### App icon (`app_icon`, 64 x 64; feeds the .icns/.ico/AppImage)

The same tile as today (52 x 52, radius 12, inset 6, SLATE_DARK, top light `#ffffff0c` over
the upper half, rim `#ffffff1a`). On it the saw-into-square wave in ORANGE, 4 px stroke, round
caps and joins, path `M13 32 L24 16 V32 H30 V16 H41 V32 H51`, with a `#00000050` 5 px shade
1 px lower; under it four step keys 8.5 x 9, radius 2, at x 12 + 10.75 i, y 38, in RED,
ORANGE, YELLOW, WHITE, each with a `#00000050` shade 1 px lower.

### About window

360 pt wide, 190 tall: the icon at 64 pt at (18, 52); the wordmark pieces at (92, 48) scaled
2.0 with the colored underline; the version and the link left-aligned at x 108 under the
subtitle (y 150 and 172); the "OpenRB-338" name line is dropped. The link needs about 20 pt
more width at that alignment, or one size smaller.

Branch: worktree-agent-aa03b4b320a6f00cd (rebased on main c97ba15).

Implementation (2026-10-29, @agent-worker). Lifted from mockups/gen/src/main.rs into crates/orb-ui-art: led_3d, keycap, key_3d, mixsw_body, shade, header (3D), wave_glyphs (parts.rs); band constants BAND_303/WHITE_303/FLAGS_303/EDIT_303 plus new KNOB_X0_303 and LEFT_X_303; layout.rs moves the 303 knobs, switch, pitch/clear, keys, flags, edit step, the pattern and bank keys (14 x 15 at 4 + 16 i, rows 25/43, bank 64), the mixer (pan 8,26; delay 40,26; routing keys 38, 62 + 16 i) and the master COMP (27, 82).
Renamed parts: btn_pill_17x10_* -> btn_pill_22x10_* (the part is 22 wide now), logo_line -> logo_line_0..3 (LOGO_PIECES is now 7 pieces: open, rb338, four segments, sub; logo = their concatenation, tests/logo.rs updated and a test for the four colored segments added). New sizes: switch_wave 40 x 14, mixsw 28 x 13, pat/bank 14 x 15.
About window (crates/orb-ui/src/about.rs): fixed 380 x 156 content, no inner margin: icon 64 pt at (18,18), wordmark at (92,14) x2, version at (108,104), link at (108,126); name line dropped. Width is 380 (spec 360 plus the 20 the link needs); title bar height is egui's own, not the mockup's 34. tests/menu.rs about test: icon + logo = 2 images (was 1). Added an OpenRB-338 --about flag (opens About at start, for --screenshot).
Regenerated: cargo run --release -p orb-ui-art -- gen (172 files, 3 removed, 6 new) and -- ico (assets/app_icon.ico, 20539 bytes, 8 sizes). A second gen + ico leaves the sha1 of every file under assets/ unchanged.
Static compare: ui-art check preview panel_2x.png vs mockups/panel_after.png: same 1248 x 1020, 545 pixels differ, all in x 482..600, y 29..30 = the wordmark underline (the mockup panel_after still had the old one-piece orange underline; wordmark.png shows the four segments, as implemented).
Screenshots: /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task115/panel.png (OpenRB-338 --demo --size 624x510 --ppp 2) and /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task115/about.png (--demo --about, same size). Differences from mockups/panel_after.png: the live demo state differs (selected patterns/banks, knob values, loop button, meters, step lights, the kit's splash in the 808 view, menu bar on top), the About screenshot is taken mid fade-in so the window is translucent, and the About title bar is egui's. All panel art (headers, LEDs, keycaps, 303 band, mixer keys, master COMP) matches in position and look.
Checks: cargo +nightly fmt --all --check clean; cargo clippy --all-targets -D warnings clean; cargo test -p orb-ui -p orb-ui-art (debug) 205 pass; cargo test --release workspace 587 pass, 0 fail; cargo xwin clippy --release -p orb-ui --target x86_64-pc-windows-msvc -D warnings clean (CL=-Wno-incompatible-pointer-types, XWIN_ACCEPT_LICENSE=1).
Not done: AC #1 (the user accepting the changes in the running app) is the user's; the animation (task-114) is not part of this.

User accepted the changes in the running app on 2026-10-29 ("ui looks fine, merge"), including the 380 pt About width and the --about option.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The approved panel art of the mockup session (2026-10-29) is in crates/orb-ui-art (parts.rs, layout.rs), assets/ui is regenerated (gen leaves no diff) and assets/app_icon.ico is regenerated with ui-art ico. What changed: the 303 (waveform switch with its label and printed glyphs, centered knobs, three-block key band, 22 x 10 pills, 3D keys, edit step in the flags block), the pattern section (rust header, 14 x 15 keycaps, shuffle keycap), the mixer (sea header, pan and delay side by side, 28 x 13 routing keys with amber LEDs, master COMP), 3D headers, round 3D on/off LEDs, the four-segment wordmark underline split into logo_line_0..3 pieces (LOGO_PIECES = 7) for task-114, the new app icon and the About window layout (--about flag for screenshots). docs/ui_spec.md (section 19, About) and assets/ui/README.md record it. Tests: cargo test --release 587 pass, debug orb-ui + orb-ui-art 205 pass, fmt, clippy and the Windows xwin clippy clean. The rendered static panel equals mockups/panel_after.png except the wordmark underline. The user accepted the result in the running app on 2026-10-29. Branch worktree-agent-aa03b4b320a6f00cd, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

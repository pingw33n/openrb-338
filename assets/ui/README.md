# assets/ui: the panel's SVG art set

The art of the bundled UI (milestone m-3): the song window ("the panel") of
`docs/ui_spec.md`, 624 x 510 px, drawn as scalable SVG parts. The app (task-044) draws them
through egui's SVG loader (`egui_extras`, usvg + resvg) and places them with `layout.tsv`.

**Everything here is generated.** `crates/rb-ui-art` builds every file from code (the flat look
of the ui-poc experiment: its palette, flat shapes, red seven-segment displays, slate and paper
sections). Do not edit an SVG by hand; change `crates/rb-ui-art/src/parts.rs` (art) or
`layout.rs` (positions) and run

```
cargo run --release -p rb-ui-art -- gen      # writes assets/ui/*.svg, panel.svg, layout.tsv
cargo run --release -p rb-ui-art -- check    # renders everything at 1x/2x/3x into preview/
cargo test -p rb-ui-art                      # committed files = generator; render + spec checks
```

## License and origin

All art is our own work, drawn in code, released under the repository's license (MIT, see
`Cargo.toml`). It contains no Propellerhead or Roland artwork, logo or mark and no copy of the
original's bitmaps: the original's screenshots were used for layout only (`docs/ui_spec.md`
places the controls). Visible texts are generic: the sections carry no product names, the
effect blocks are PCF, DELAY, DIST and COMP, and the logo is the wordmark "OPENRB-338" (the
project's name, task-068). File names keep the spec's short names (`tb303`, `tr808`, `tr909`) because they are
code identifiers, not art. The labels are vector paths of a built-in stroke font
(`src/font.rs`), so the SVGs hold no `<text>` and need no font file.

## Coordinates and scaling

- Every part's `width`/`height` and `viewBox` are its **native size in panel pixels** (a
  30 x 30 knob is 30 x 30 units). The sections are the original's (`docs/ui_spec.md`); the
  placement of the controls inside them is rb's own design (task-085), so `layout.tsv`, not the
  spec, has the control rectangles. The anchor of a part
  is its top-left corner at the control's top-left; there are no margins to subtract.
- To draw a part at scale `s` (window scale, `pixels_per_point`) load it with a size hint of
  `(w * s, h * s)` physical pixels and reload on a scale change (decision task-040). All art is
  vector; nothing is a bitmap.
- `layout.tsv` has one line per control (`id section kind part x y w h`): the control's
  rectangle in **panel pixels** (origin top-left of the 624 x 510 panel; divide by 624 and 510
  for the fractions of `docs/ui_spec.md`). `kind` is the spec's class. `part` is the part to draw
  (the default state), or the family name for a composed control (see below).
- `panel.svg` is the whole panel composed from the parts in a demo state (song mode, playing,
  pattern 1/3/2/6 selected).
  `preview/panel_{1,2,3}x.png` are its renders and `preview/parts_{1,2,3}x.png` sheets of every
  part with its name (resvg at 1x, 2x, 3x: the render check of task-042).
- A background is drawn first, controls over it. The dark seam of 1 px around a section is part
  of its background.

## Sections (backgrounds)

Backgrounds hold the face, the titles, the printed labels and the display wells; no control.
Place each at its section origin (panel pixels).

| Part | Size | Origin in the panel | Used for |
|---|---|---|---|
| `wood` | 16 x 510 | (0, 0) and (608, 0) | left and right side wood |
| `bg_transport` | 592 x 50 | (16, 0) | transport, with the logo at (458, 1) |
| `bg_pattern` | 70 x 115 | (16, 50 + 115 r) | pattern section of row r = 0..3 |
| `focus_off`, `focus_on` | 4 x 115 | (86, 50 + 115 r) | the row's focus bar (lit orange) |
| `bg_tb303` | 366 x 115 | (90, 50) and (90, 165) | the two bass synths |
| `bg_tr808` | 366 x 115 | (90, 280) | drum machine A |
| `bg_tr909` | 366 x 115 | (90, 395) | drum machine B |
| `bg_mixer` | 70 x 115 | (456, 50 + 115 c) | mixer channel c = 0..3 |
| `bg_master` | 82 x 98 | (526, 50) | master |
| `bg_pcf` | 82 x 111 | (526, 148) | PCF |
| `bg_delay` | 82 x 93 | (526, 259) | delay |
| `bg_dist` | 82 x 65 | (526, 352) | distortion |
| `bg_comp` | 82 x 93 | (526, 417) | compressor |

Control positions in `layout.tsv` are already absolute. The drum legend rows are in the
backgrounds (808: 12 cells of 22 px at (10, 71); 909: the 12 instrument chips of the top row at
(4, 4) with the edges of `EDGES_909` in `parts.rs`, and the name row at (26, 74)). The 303's key
band (`BAND_303`) is three sections: pitch mode and clear, the keys on gray, and one row of the
flags and note/rest under the edit step display.

`app_icon` (64 x 64) is not part of the panel: the app renders it as the window icon.

## Controls

States: push buttons `rest` / `pressed`; toggles `off` / `on`; transport `lit` while playing or
recording. Sizes are `w x h` in panel pixels.

### Knobs (rotation-friendly layers)

A knob is three layers, all of the control's size, drawn at the same anchor:

1. `<knob>_body`: the faint full-range ring track and the body (static);
2. the value arc, **painted by the app** with the painter (not a file): an arc of radius
   `ring_r`, width `ring_w`, color `arc`, centered on `(cx, cy)`, from the start angle to the
   pointer angle (a pan knob starts at 0 degrees, the middle);
3. `<knob>_pointer`: the pointer drawn straight up; rotate it around `(cx, cy)`.

The angle is `-135 + 270 * v` degrees (0 = up, clockwise positive), `v = p / 1000` for the
knob position p of `docs/ui_spec.md` section 2. `ring_r`, `ring_w`, `arc` and `cx, cy` per knob
are in `KNOBS` (`src/parts.rs`) and below:

| Knob | Size | center (cx, cy) | ring r / width | arc color | Used for |
|---|---|---|---|---|---|
| `knob_303` | 30 x 30 | (15, 15) | 13.6 / 1.6 | `#f08a24` | the bass synth's six knobs |
| `knob_808_red` | 13 x 13 | (6.5, 6.5) | 5.9 / 0.9 | `#f2c531` | drum machine A level knobs |
| `knob_808_white` | 13 x 13 | (6.5, 6.5) | 5.9 / 0.9 | `#f08a24` | drum machine A tone, tuning, decay |
| `knob_909` | 12 x 12 | (6, 6) | 5.4 / 0.9 | `#e2413b` | drum machine B knobs and flam |
| `knob_22` | 22 x 22 | (11, 11) | 10.1 / 1.4 | `#ffb000` | shuffle, pan, delay send, effect knobs |

### Faders and switches

- `fader_track` 15 x 54 and `fader_handle` 15 x 11. The handle's top is at
  `(1 - v) * 43` px from the track's top (`v` = position / 999; travel 43 px); the track holds the
  scale ticks and slot. Used by the mixer levels, the master and the four PCF faders.
- Two-position switches, `_0` / `_1` (0 = left or top): `switch_wave_` 16 x 12 (303 waveform:
  0 saw, 1 square), `switch_808_` 11 x 17 (shared slots: 0 upper label, 1 lower), `switch_fx_`
  9 x 18 (PCF BP/LP; delay triplet/16th), `switch_mode_` 26 x 19 (transport: 0 pattern, 1 song).
  A click anywhere on a two-position switch flips it.
- `mixsw_dist_`, `mixsw_pcf_`, `mixsw_comp_` (`off` / `on`) 20 x 11: the routing buttons of the
  mixer channels; `mixsw_comp_` also on the master.
- `power_13x13_`, `power_14x14_` (`off` / `on`): flat green on/off lamps in the headers: pattern
  section; mixer mute and effects.
- `toggle_loop_` 18 x 18 (icon only), `toggle_shuffle_` 30 x 12, `toggle_flam_` 14 x 8 (`off` /
  `on`; the flam button, shaped like a 909 step's light).

### Buttons

| Part | Size | States | Used for |
|---|---|---|---|
| `btn_play_`, `btn_record_` | 32 x 25 | `rest`, `pressed`, `lit` | Play, Record |
| `btn_stop_`, `btn_rewind_`, `btn_ff_` | 32 x 25 | `rest`, `pressed` | Stop, Rewind, Fast forward |
| `arrow_inc_`, `arrow_dec_` | 11 x 9 | `rest`, `pressed` | every value display's up / down arrows |
| `pat_btn_1_` .. `pat_btn_8_` | 17 x 20 | `off`, `on` | pattern grid, 4 x 2 cells at (1 + 17 col, 19 + 20 row) of the section |
| `bank_btn_A_` .. `bank_btn_D_` | 17 x 20 | `off`, `on` | bank row, 4 cells at (1 + 17 i, 66) |
| `arrow303_inc_`, `arrow303_dec_` | 11 x 9 | `rest`, `pressed` | 303 Step (up) and Back (down) |
| `btn_pill_17x10_` | 17 x 10 | `rest`, `pressed` | 303 Pitch mode, Clear |
| `key_white_`, `key_black_` | 9 x 16, 8 x 15 | `rest`, `pressed` | 303 note keys (white at y 89, black at y 60) |
| `btn_flag_` | 9 x 16 | `rest`, `pressed` | 303 Down, Up, Accent, Slide, Note/Rest |

### LEDs and steps

- `led_8_off/on` 8 x 8 (303 key, flag, note/rest, pitch mode LEDs; the note/rest LED is lit
  for a note); `gr_led_off/green/amber/red` 7 x 10: the compressor's 9 gain-reduction LEDs at
  (9.5 + 7 i, 25); the app picks the colors.
- `step808_<red|orange|yellow|white>_<off|on>` 16 x 17 at (14 + 16 i, 91), the color by `i / 4`;
  the LED is the lit state (the app inverts it for the running light).
- `step909_<a|b>_<0|1|2|3>` 21 x 16 at (26 + 21 i, 94), under the name row's cells: `a` for
  steps in groups 0 and 2, `b` for groups 1 and 3; value 0 off, 1 note (orange), 2 accent (red),
  3 flam (green).
- `sel_22x14`, `sel_42x14`, `sel_21x14`: translucent frames to mark the selected instrument over
  a legend cell (22 and 42 px: the 808's cell and the 909's AC chip, the 909's BD..HT chip; 21:
  the 909's RS..RC chip). The 909's AC is selected from its chip.

### Displays and meters

- `seg_0` .. `seg_9`, `seg_blank`: one seven-segment digit frame, 9 x 16 (the 11 frames of
  `CLEDNumberView`). A display of n digits is n frames right-aligned in its rectangle (the
  rectangle is 9n wide, or 9n + 1 for the 19 px displays); leading zeros are `seg_blank`. A
  disabled display (pattern mode) is all `seg_blank`: the unlit segments stay faintly visible.
  The dark well behind is in the background.
- `meter_8x14_`, `meter_15x56_` + `0` .. `10`: level meters (the mixer channels' and effects'
  header meters, bare segments; master left and right, in a dark well) with 10 segments and 11
  states; the app maps the engine's meter byte to the state.
- `mod_view`: 84 x 105, the dark well right of drum machine A's legend (control
  `tr808.mod_view`, kind `modview`). The app paints the kit's `splash.jpg` in the 76 x 76
  square at the top and the mod's name under it.

## How the app composes them

`rb_ui_art::layout::compose` (`crates/rb-ui-art/src/layout.rs`) is the reference: background,
then per control the parts at the control's rectangle (knob: body, arc, rotated pointer; fader:
track and handle; display: digit frames). `docs/ui_spec.md` has the behavior (which values the
controls show and send). The test `controls_are_inside_the_panel` checks that every control of
`layout.tsv` lies inside the panel.

## Not done

- The 8 pattern buttons and the bank buttons carry their number or letter in the part; a pattern
  above 8 is the bank's business of the app.
- The Sync and MIDI indicators (MIDI is out of scope) are not drawn.
- The art was rendered with resvg directly (`ui-art check`), the same usvg + resvg that
  `egui_extras` uses. task-044 then ran every part through egui itself (`rb-ui`,
  `crates/rb-ui/tests/interaction.rs`: all parts load and rasterize at 1x/2x/3x; screenshots
  in `docs/ui_app/`).
- `layout.tsv` ids are not unique: `tb303_N.accent` is both the accent knob and the accent flag
  button. The app keys controls by kind and id.

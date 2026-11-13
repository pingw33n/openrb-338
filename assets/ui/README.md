# assets/ui: the panel's SVG art set

The art of the bundled UI (milestone m-3): the song window ("the panel") of
`docs/ui_spec.md`, 624 x 510 px, drawn as scalable SVG parts. The app (task-044) rasterizes them
with resvg (the renderer of egui_extras' SVG loader) and places them with `layout.tsv`.

**Everything here is generated.** `crates/orb-ui-art` builds every file from code. Do not edit an
SVG by hand; change `crates/orb-ui-art/src/parts.rs` (art), `svg.rs` (the shared gradients and
filters), `text.rs` (labels) or `layout.rs` (positions) and run

```
cargo run --release -p orb-ui-art -- gen      # writes assets/ui/*.svg, panel.svg, layout.tsv
cargo run --release -p orb-ui-art -- check    # renders everything at 1x/2x/3x into preview/
cargo test -p orb-ui-art                      # committed files = generator; render checks
```

## The look (task-179)

The redesign of task-179 follows the original ReBirth RB-338 2.01 panel as closely as possible
(the user's decision of 2026-11-13): the original's sections and control places
(docs/ui_spec.md sections 3 to 9), gray speckled plates with a bevel, glossy headers (red for
PATTERN, olive for MIX and MASTER, blue for the effects), silver push buttons, round LEDs with
halos, seven-segment displays with a glow, a light 303 with a silver key plate, a dark olive 808
with cream chips and glossy step keys, a pale 909 with dark chips, and wooden cheeks. It was
drawn and reviewed with the user in many rounds as a mockup (a Python script that drew the
whole panel as one SVG), approved on 2026-11-13, and ported here; the approved render is
`docs/ui_spec/redesign_mockup_2x.png` and docs/ui_spec.md section 19 lists where the art leaves
the spec's tables. Kept from before: the mod's splash and name where the original has the 808's
instrument selector, and the animated wordmark `OPENRB-338` at the top right.

The user's choices for the details: knobs are flat-topped and seen from above (a static body, a
turning pointer, short cast shadows toward the lower right, no value arc); a pressed push button
is "variant F, dimmed" (the bevel and the cast shadow go, the face turns flat and a little
lighter with a faint gradient, `gPillFlatLit`); the routing keys (DIST, PCF, COMP and the
master's COMP) are drawn pressed only while the mouse is held on them, never after, and only
their lamp shows the state; the effects' on/off lamps are as big as the mixer's; the Sync and MIDI LEDs are drawn,
dark (MIDI is out of scope).

Numbers in panel pixels. The generator functions are named after what they draw (`plate`,
`header`, `onoff`, `meter4`, `display`, `pill`, `keycap`, `led_round`, `lever`, `slide_v`,
`knobs`, `fader`, `chip808`, ...) and hold every size, color, gradient and filter.

## License and origin

All art is our own work, drawn from scratch in code, released under the repository's license
(GPL-3.0-only, see `Cargo.toml`). It contains no copy of the original's bitmaps and no
Propellerhead or Roland artwork, logo or mark: the original's screenshots and the iPad version
of ReBirth were looked at for the layout and the look, which since task-179 the art follows
closely by the user's decision. Visible texts are generic: the sections carry no product names,
the effect blocks are PCF, DELAY, DIST and COMP, and the logo is the wordmark "OPENRB-338" (the
project's name, task-068). File names keep the spec's short names (`tb303`, `tr808`, `tr909`)
because they are code identifiers, not art.

**The font.** The labels are set in Inter 4.1 (Regular, Medium and Bold; copyright 2016 The
Inter Project Authors, https://github.com/rsms/inter; SIL Open Font License 1.1,
`crates/orb-ui-art/fonts/LICENSE.txt`).
The three static TTFs are committed with the generator (`crates/orb-ui-art/fonts/`), and the
generator turns every label into path data when it writes the art (`text.rs`: usvg lays the text
out with only these three fonts in its font database, no system font, so the outlines are the
same on every machine). The SVGs of this folder hold no `<text>`, and the app needs no font
(orb-ui builds resvg with no text support). The OFL allows embedding the outlines in the art;
the font files themselves are not shipped with the app. The wordmark keeps its own stroke glyphs
(`crates/orb-ui-art/src/font.rs`); the digits of the displays are shapes.

## Coordinates, sizes and bleed

- A part's rectangle `w x h` is its **control's rectangle in panel pixels** (the hit area of
  `layout.tsv`), or its section's for a background.
- Many parts draw beyond that rectangle: cast shadows of knobs and buttons, the halos of lit
  LEDs and lamps, the glow of lit keys and segments, the selected 808 chip's ring. Each part has
  a **bleed** `b`: the file's canvas is `(w + 2b) x (h + 2b)` and its `viewBox` is
  `-b -b w+2b h+2b`, so the control's rectangle is at (0, 0) of the viewBox and the art may
  reach `b` beyond it on every side. The app reads `b` from the viewBox (`crates/orb-ui/build.rs`),
  draws the part over the control's rectangle grown by `b`, and hit-tests the control's
  rectangle only. A part with `b = 0` has `viewBox="0 0 w h"`.
- Draw a part at scale `s` (window scale times pixels per point) by rasterizing it at
  `((w + 2b) s, (h + 2b) s)` pixels (decision task-040). All art is vector; the one bitmap the
  panel shows is the mod's splash, which the app paints.
- `layout.tsv` has one line per control (`id section kind part x y w h`): the control's
  rectangle in **panel pixels** (origin top-left of the 624 x 510 panel). `kind` is the spec's
  class. `part` is the part to draw (the default state), or the family name for a composed
  control (see below).
- `panel.svg` is the whole panel composed from the parts in the mockup's demo state (song mode,
  playing, loop on, tempo 140, bar 9; row 1 focused; pattern row 2 off with SHUFFLE lit; 303
  number 1 in pitch mode with a square wave and a rest, number 2 with BACK and STEP held; BD
  selected on both drum machines; 909 flam lit; channel 2 routed to DIST; the master's COMP
  on). `preview/panel_{1,2,3}x.png` are its renders and `preview/parts_{1,2,3}x.png` sheets of
  every part in every state with its name (gray: the control's rectangle; magenta outline: the
  canvas with the bleed). `panel.svg` has no splash (the app paints it).
- A background is drawn first, controls over it, then the foregrounds (below), then the
  selection marks. The dark seam of 1 px around a section is part of its background.

## Sections (backgrounds)

Backgrounds hold the plate, its speckle, the headers and their titles, the printed labels, the
display wells and the frames of the arrow keys; no control. Place each at its section origin.

| Part | Size | Origin in the panel | Used for |
|---|---|---|---|
| `wood_left`, `wood_right` | 16 x 510 | (0, 0), (608, 0) | the side cheeks |
| `bg_transport` | 592 x 50 | (16, 0) | transport (with the dark Sync and MIDI LEDs; no logo) |
| `logo` | 134 x 48 | (474, 1) | the wordmark, over `bg_transport`; the click area of the logo |
| `logo_open`, `logo_rb338`, `logo_line_0` .. `logo_line_3`, `logo_sub` | 134 x 48 | (474, 1) | the pieces of `logo` |
| `bg_pattern_1` .. `bg_pattern_4` | 70 x 115 | (16, 50 + 115 r) | the pattern section of row r = 0..3 |
| `focus_off`, `focus_on` | 4 x 115, bleed 2 | (86, 50 + 115 r) | the row's focus bar (a red line, lit with a glow) |
| `bg_tb303` | 366 x 115 | (90, 50), (90, 165) | the two bass synths |
| `bg_tr808` | 366 x 115 | (90, 280) | drum machine A |
| `bg_tr909` | 366 x 115 | (90, 395) | drum machine B |
| `bg_mixer_1` .. `bg_mixer_4` | 70 x 115 | (456, 50 + 115 c) | mixer channel c = 0..3 |
| `bg_master` | 82 x 98 | (526, 50) | master (with the meters' dark wells) |
| `bg_pcf` | 82 x 111 | (526, 148) | PCF |
| `bg_delay` | 82 x 93 | (526, 259) | delay |
| `bg_dist` | 82 x 65 | (526, 352) | distortion |
| `bg_comp` | 82 x 93 | (526, 417) | compressor |

**The speckle.** The gray plates, the 808 and the 909 carry a speckle of thresholded fractal
noise (`feTurbulence`; the filters `grD`, `grL`, `blotD`, `blotL`, `marble`, `woodD`, `woodL` of
`svg.rs`). The noise is laid in **panel coordinates** (the noise rectangle is translated to the
part's place in the panel inside the plate's clip), so no two plates look alike, as on the
mockup, which laid one speckle over the whole panel: that is why the pattern sections, the mixer
channels and the two cheeks have one background part each. The two 303s have no speckle and
share `bg_tb303`. The fine speckle (`grD`, `grL`) is a pattern: a stitched noise tile of
64 x 64 panel pixels (`pgrD`, `pgrL`, `GRAIN_TILE`) repeated in panel coordinates, so a part
rasterizes the noise of one tile, not of its whole plate; the coarse blotches and the 808's,
909's and wood's grain are noise over the plate.

**Foregrounds**, static art drawn after the controls, where a control would otherwise lie over
it: `fg_tr808` (107 x 10 at (168, 324): the 808's upper labels LC, MC, HC, CL, MA, over the
tone knobs' cast shadows) and `fg_pcf` (76 x 7 at (530, 251): the PCF faders' captions, over the
handles at the bottom of their travel).

**The wordmark.** `OPENRB-338` in the stroke glyphs of `font.rs` (cap 14: scale 2.33, the run at
x 8, `338` at x 92, so the startup animation's `OPEN_TRAVEL` is unchanged), embossed: a stroke of
1.02 glyph units with a dark copy below-right and a light copy above-left; ink `#f2f5f7`, `338`
orange `#ff9a2e`. The underline is four glossy bars of 28.4 x 2.6 at (8 + 29.9 i, 27.6) in the
808's step key gradients (red, orange, yellow, white) with a soft shadow, and the subtitle `BASS
LINE AND DRUM MACHINE` is Inter Medium 5.5 with letter spacing 1.02 in the panel's label style.
`logo` is exactly its seven pieces in the order of `LOGO_PIECES` (task-114: each piece is on the
whole canvas, so the animation only offsets or fades a piece).

**Help > About**: `about_plate` (268 x 92, bleed 8) is the gray plate with its bevel, speckle
and a soft shadow under the wordmark at 2x, at (92, 14) of the window's content.

**The desk**: `desk_tile` (256 x 256) is the speckle and a faint horizontal brushing of the
desk around the unit in a window of another shape, made with stitched noise so the tile repeats
without a seam; the app lays it over its own gradient (docs/ui_spec.md section 19).

`app_icon` (64 x 64) is the 303's saw running into a square wave (orange) over four step keys in
the 808's colors, on a slate tile (task-115; unchanged by task-179). It is not part of the panel:
the app renders it as the window icon and in Help > About, and `ui-art iconset` / `ui-art ico`
make the macOS `.icns` and the Windows `assets/app_icon.ico` from it.

## Controls

States: push buttons `rest` / `pressed`; toggles `off` / `on`; Play and Record also `lit`.
Sizes are the controls' rectangles; the bleed is in the part's viewBox.

### Knobs (two layers)

A knob is two layers of the control's size, drawn at the same anchor:

1. `<knob>_body`: the tick marks, the cast shadow, the wall or collar and the flat top (static);
2. `<knob>_pointer`: the pointer drawn straight up; rotate it around `(cx, cy)`, the middle of
   the rectangle.

The angle is `-135 + 270 * v` degrees (0 = up, clockwise positive), `v = p / 1000` for the knob
position p of `docs/ui_spec.md` section 2. There is no value arc (task-179).

| Knob | Size | center | bleed | Look | Used for |
|---|---|---|---|---|---|
| `knob_303` | 30 x 30 | (15, 15) | 1 | brushed aluminum top with light and dark sectors, dark knurled wall, black index, 11 heavy ticks | the bass synths' six knobs |
| `knob_808_red` | 13 x 13 | (6.5, 6.5) | 2.5 | flat red cap in a black collar | drum machine A levels |
| `knob_808_white` | 13 x 13 | (6.5, 6.5) | 2.5 | flat pale cap in a black collar | drum machine A tone, tuning, decay, snappy |
| `knob_909` | 12 x 12 | (6, 6) | 2 | dark cylinder, orange index, 11 fine ticks | drum machine B knobs and flam |
| `knob_gray` | 18 x 19 | (9, 9.5) | 3 | blue-gray flat cap in a black knurled collar, white index, 7 ticks | shuffle, pan, delay send, the effects' knobs (their scale and caption are in the background) |

### Faders and switches

- `fader_track` 15 x 54 (11 tick lines and the dark slot) and `fader_handle` 15 x 11 (dark,
  with a white line and a soft shadow, bleed 4.5). The handle's top is at `(1 - v) * 43` px
  from the track's top (`v` = position / 999). Used by the mixer levels, the master and the four
  PCF faders.
- Two-position switches, `_0` / `_1` (0 = left or top): `switch_wave_` 16 x 12 (303 waveform,
  a slide between the printed saw and square: 0 saw, 1 square), `switch_808_` 11 x 17 (the
  shared slots: 0 the upper label, 1 the legend's), `switch_fx_` 9 x 18 (PCF BP/LP; delay
  triplet/16th), `switch_mode_` 26 x 19 (transport: the green Pattern LED, a chrome bat toggle
  and the yellow Song LED; 0 pattern, 1 song). A click anywhere on a two-position switch flips
  it.
- `toggle_loop_` 15 x 18 (`off` / `on`): the loop's bat toggle and its yellow LED.
- `mixsw_off`, `mixsw_on` 24 x 11, and `mixsw_off_pressed`, `mixsw_on_pressed`: the routing
  keys DIST, PCF and COMP of the mixer channels and the master's COMP (a silver key and a
  yellow lamp at its right; the label is in the background). The key shows pressed (variant
  F) only while the mouse is down on it, like the 303's BACK and CLEAR; the lamp lights when
  the routing is on.
- `power_pat_`, `power_mix_`, `power_fx_` (`off` / `on`): the green on/off lamps of the headers,
  13 x 13.6, 17 x 14 and 16 x 16, each in its header's frame color; lit with a halo. The lamp
  is 8.6 square in the pattern sections' and 9.6 in the mixer's and the effects' (the same
  size, the user's request).
- `toggle_shuffle_` 15 x 15 (`off` / `on`): the pattern section's SHUFFLE keycap, lit red.
- `toggle_flam_` 14.8 x 14.8 (`off` / `on`): the 909's flam button, lit red with a glow.

### Buttons

| Part | Size | States | Used for |
|---|---|---|---|
| `btn_play_`, `btn_record_` | 32 x 25 | `rest`, `pressed`, `lit` | Play (lit: green with a halo; at rest a darker green), Record (lit: red with a halo) |
| `btn_stop_`, `btn_rewind_`, `btn_ff_` | 32 x 25 | `rest`, `pressed` | Stop, Rewind, Fast forward |
| `arrow_inc_`, `arrow_dec_` | 11 x 9 | `rest`, `pressed` | every value display's up / down keys (their frame is in the background) |
| `pat_btn_1_` .. `pat_btn_8_` | 14.6 x 14.6 | `off`, `on` | pattern keys: white keycaps at centers (11.6 + 15.6 col, 30.9 + 15.6 row) of the section; the selected one tinted red with a glow |
| `bank_btn_A_` .. `bank_btn_D_` | 14.6 x 14.6 | `off`, `on` | bank keys at centers (11.6 + 15.6 i, 72.9) |
| `btn_pill_15x8_` | 15 x 8.4 | `rest`, `pressed` | 303 Pitch mode |
| `btn_pill_17x10_` | 17 x 10 | `rest`, `pressed` | 303 Clear, Note/Pause, Back |
| `btn_step_` | 25.4 x 17 | `rest`, `pressed` | 303 Step (with two grip lines) |
| `key_white_`, `key_black_` | 9 x 16, 8.4 x 15.4 | `rest`, `pressed` | 303 note keys (the small silver buttons on the drawn keyboard) |
| `btn_flag_` | 9 x 16 | `rest`, `pressed` | 303 Down, Up, Accent, Slide |

Every `pressed` state is variant F (above). The push buttons' bleed is 3 (their shadow).

### LEDs, steps and selection marks

- `led_303_off/on` and `led_303s_off/on`, 6 x 6 around the lamp (radius 1.9 and 1.8): the 303's
  LEDs (keys, Note, Rest, the flags; Pitch mode and the black keys).
- `gr_led_green/amber/red` (lit) and `gr_led_off_g/off_y/off_r` (unlit, in their place's color:
  the center green, the two beside it amber, the outer ones red), 7 x 7: the compressor's nine
  gain-reduction LEDs at (8 + 7 i, 26.5) of the section.
- `step808_<red|orange|yellow|white>_<off|on>` 12.6 x 19 at (16 + 16 i, 90.2): glossy keys
  with a small round LED at the top; the LED is the lit state (the app inverts it for the
  running light).
- `step909_<0|1|2|3>` 16.4 x 16.8 at (29 + 21 i, 85.4): silver keys with a light bar, unlit or
  orange, red, green for the values 1, 2, 3 (the running light shows as 2 on an empty step and
  as 0 on a set one; the step numbers are in the background).
- `sel808_<AC..CH|LC..MA>`: the selected 808 instrument's chip, lighter with an orange ring,
  drawn over its chip (17.8 x 9.2 at (11.6 + 22 i, 72.6), or 17.8 x 8.8 at (78.6 + 22 k, 44.4)
  for an upper label).
- `sel909_<AC..RC>`: the selected 909 instrument's label lit on its chip (y 4.6, 9.2 high; a
  patch of the chip with the label glowing orange), and `led909_on` / `led909ac_on` (5 x 5) its
  lit selection LED in the name row or the AC box.

### Displays and meters

- `seg_0` .. `seg_9`, `seg_blank`: one seven-segment cell, 9 x 16, skewed 5 degrees about its
  middle row, the unlit segments faintly visible, the lit ones with a glow, under a faint glass
  sheen (bleed 2). A display of n digits is n cells centered in its rectangle (the rectangle is
  9n wide, or 9n + 1); leading zeros are `seg_blank`; a disabled display (pattern mode) is all
  `seg_blank`. The well behind is in the background.
- `meter_14x14_0` .. `_4`: the header meters of the mixer channels and the effects, four
  segments (green, yellow-green, amber, red from the bottom). The app maps the engine's 11 meter
  states onto them (docs/ui_spec.md section 19).
- `meter_15x56_0` .. `_10`: the master's two meters, 10 segments in an 11-state column (the
  dark well is in the background).
- `mod_view`: 84.4 x 106 at (277.2, 4.2) of the 808, the dark well where the original has the
  instrument selector (control `tr808.mod_view`, kind `modview`). The app paints the kit's
  `splash.jpg` in the 76 x 76 square at (4.2, 3.7) of it, lays `mod_view_glass` (76 x 76) over
  the picture, and writes the mod's name under the line at y 85.

## How the app composes them

`orb_ui_art::layout::compose` (`crates/orb-ui-art/src/layout.rs`) is the reference: backgrounds,
then per control the parts at the control's rectangle (knob: body, rotated pointer; fader:
track and handle; display: digit cells), then the foregrounds, then the selection marks.
`docs/ui_spec.md` has the behavior (which values the controls show and send). The test
`controls_are_inside_the_panel` checks that every control of `layout.tsv` lies inside the panel.

## Notes

- The 8 pattern keys and the bank keys carry their number or letter in the part.
- `layout.tsv` ids are not unique: `tb303_N.accent` is both the accent knob and the accent flag
  button. The app keys controls by kind and id.
- The filters (blurs and noise) make the parts slower to rasterize than flat art; the app
  rasterizes the missing parts of a frame in parallel and keeps the old size's textures during a
  resize (docs/ui_spec.md section 19 has the measurements).

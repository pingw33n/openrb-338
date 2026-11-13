# mockup/179: the panel redesign, approved as a mockup (task-179)

This folder is **temporary**. It holds the redesign of OpenRB-338's panel that the user approved
on 2026-11-13 as a mockup, and everything needed to apply it to the app. task-179 ports it into
`crates/orb-ui-art` and `crates/orb-ui`, moves the files that stay to their final places
(section 9) and deletes this folder in the same commit.

The mockup is not app code. `mock.py` draws the whole panel as one SVG in a demo state; the app
draws it from parts (`assets/ui/*.svg`) that `crates/orb-ui-art` generates. Port the look and the
numbers, not the script's structure.

## 1. What the user asked for and decided

The request (2026-11-13): redesign the panel to match the original ReBirth RB-338 2.01 panel as
closely as possible, as scalable SVG, using the desktop original for the control types and the
higher-resolution iPad version of ReBirth for the rendering detail. Two things of the current
design stay: the mod's splash where the original has the 808's instrument selector wheel, and the
animated OPENRB-338 wordmark at the top right.

Decisions taken during the mockup rounds (all by the user, 2026-11-13). Do not reopen them:

| Topic | Decision |
|---|---|
| Foundation | SVG stays. No pre-rendered PNG art. |
| Layout | The original's sections and control places (docs/ui_spec.md sections 3 to 9), not the placement of task-085/task-115. Where `mock.py` differs from the spec's tables, `mock.py` wins (section 4 lists the differences). |
| Knobs | Flat-topped, seen from above; no sphere-like radial shading. The body is static, only the pointer turns. Short cast shadows toward the lower right (the long ones "floated"). No value arc. |
| Pressed push buttons | "Variant F, dimmed": the bevel and the cast shadow go, the face turns flat and a little lighter with a faint gradient (`pill(..., pressed=True)`, style `flatlit`, gradient `gPillFlatLit`). Not darker, not a reversed gradient, no colored glow. |
| Routing keys (DIST, PCF, COMP) | Never drawn pressed while on: the key stays up and only the lamp beside it lights (`ROUTE_STYLE = "led"`). |
| Font | Inter 4.1 (SIL OFL 1.1), weights Regular, Medium and Bold, converted to outlines when the art is generated. The shipped SVGs hold no `<text>`. |
| Wordmark | Same glyph paths and the same seven pieces as today, so `logo_anim.rs` keeps working; heavier embossed stroke, glossy underline bars in the 808's key colors, the subtitle in the panel's label style. |
| Help > About | Only the wordmark sits on a gray speckled plate; the rest of the window keeps the app's dark window colors, text colors and the flat icon. |
| Wide windows | A dark matte desk surface left and right of the unit, with the unit's cast shadow (`backdrop()`), in place of the flat letterbox color. |
| Sync and MIDI | Their two LEDs and labels are drawn, dark (MIDI is out of scope). They are background art, not controls. |

Texts stay generic, as today: the section titles are PATTERN, MIX, MASTER, PCF, DELAY, DIST and
COMP; no product name, no Roland or Propellerhead logo or mark, and the wordmark is OPENRB-338.

## 2. What is in this folder

| Path | What |
|---|---|
| `mock.py` | The mockup generator: the whole panel as one SVG (624 x 510 panel units). Authoritative for every position, size, color, gradient and filter. |
| `anim.py`, `anim_template.html` | The wordmark's startup animation with the new art, in the main window and in About: an HTML page and two film strips. Also draws the About window's content (`about_plate()`). |
| `render/` | A small resvg renderer (SVG to PNG with the fonts of a folder). Not a workspace member. |
| `fonts/` | `Inter-Regular.ttf`, `Inter-Medium.ttf`, `Inter-Bold.ttf` (the static TTFs of Inter 4.1) and Inter's `LICENSE.txt`. |
| `reference/panel_2x.png` | The approved panel, rendered at 2x (1248 x 1020). |
| `reference/panel_wide_window_2x.jpg` | The panel in a 900-unit-wide window with the backdrop. |
| `reference/logo_animation_main_frames.jpg`, `reference/logo_animation_about_frames.jpg` | Key frames of the animation at 0, 0.15, 0.35, 0.6, 0.9, 1.2, 1.5 and 1.85 s. |

To render the mockup (from the repository root; the outputs go to any scratch folder `OUT`):

```
cargo build --release --manifest-path mockup/179/render/Cargo.toml --target-dir target/mockup-render
cargo run --release -p orb-cli -- mod unpack assets/Standard.rbm -o OUT/standard
python3 mockup/179/mock.py OUT/panel.svg OUT/standard/splash.jpg
target/mockup-render/release/render OUT/panel.svg OUT/panel_3x.png 3 mockup/179/fonts
python3 mockup/179/mock.py OUT/wide.svg OUT/standard/splash.jpg 900
target/mockup-render/release/render OUT/wide.svg OUT/wide_2x.png 2 mockup/179/fonts
python3 mockup/179/anim.py target/mockup-render/release/render OUT/standard/splash.jpg OUT/anim
```

The render at scale 2 is byte-identical to `reference/panel_2x.png` (checked on 2026-11-13 with
resvg 0.49.0; the renderer loads no system fonts). Render at 3 or higher and crop to study a
detail; the user reviewed the mockup at 3x.

## 3. The demo state the mockup shows

Song mode, playing, loop on, tempo 140, bar 9. Row 1 has the focus. Each control state appears at
least once so that it could be reviewed:

- Pattern row 2: header LED off and SHUFFLE lit; the other rows LED on, SHUFFLE off.
- 303 number 1: PITCH MODE lit, waveform square. 303 number 2: BACK and STEP held down.
- 808: BD selected (orange ring on its chip), steps 1, 5, 9, 13 lit.
- 909: BD selected, FLAM button lit, steps with the values 1 (orange), 2 (red) and 3 (green).
- Mixer channel 2: DIST on (lamp only). Master: COMP on (lamp only).
- Header meters at 0, 1, 2, 3 and 4 of 4 segments (PCF idle, DELAY full).
- PCF faders at 0, about half, 1 and about a third, to show the travel's two ends.

States the mockup does **not** show, to be derived in the same style and listed in the task's
notes for the user's review: the transport keys pressed (use the variant F treatment on the key
face) and Record lit (red, with a halo like Play's green one); Play at rest (the triangle without
the halo, darker green); pattern mode (the three song displays all unlit, the mode switch and its
LEDs the other way); the loop switch off; the white and black 303 keys and the flag keys pressed;
the arrows pressed; the two-position slide switches in their other position; the master meters'
11 states and the gain reduction LEDs in each color; a 909 step that is the running light.

## 4. The design, section by section

All numbers are panel pixels. A section's art is drawn relative to the section's origin
(`g("translate(...)")` at the top of each function). Read the function for the exact numbers.

**Shared.**
- Gray plates (`gray_base`, `plate`, `plate_edges`, `GRAY = #5f7079`): a 1 px dark seam, a face
  with a light top and left edge and a dark bottom and right edge, and a speckle laid over it
  (`grain_over` with the filters `grD`, `grL`, `blotD`, `blotL`: `feTurbulence` thresholded to
  black or white specks and clipped to the plates).
- Printed labels on gray (`stext`): Inter Medium, near white, with a dark copy offset 0.45 px
  down and right. Labels on the light faces (303, 909) are plain dark text (`text`).
- Headers (`header`, `fx_header`): a glossy bar with a soft shadow, the title in Inter Bold. Red
  for PATTERN, olive for MIX and MASTER, blue for the four effects (`gHdrPat`, `gHdrMix`,
  `gHdrFx`). The on/off button (`onoff`) is a square green lamp in a dark frame, lit with a halo.
  The header meter (`meter4`) is 14 x 14 with four segments (green, yellow-green, amber, red from
  the bottom), the same size on MIX and on the effects.
- Displays (`display`, `seg_paths`): seven-segment digits in a 9 x 16 cell, skewed 5 degrees, the
  unlit segments faintly visible, the lit ones with a small glow.
- Arrows (`arrows`): two silver keys of 11 x 9 in one dark frame.
- Knobs: `knob_gray` (18 x 19, the gray sections; blue-gray flat cap in a black knurled collar,
  white index, 7 tick marks), `knob_303` (30 x 30; brushed metal top with light and dark sectors,
  dark knurled wall, black index, 11 heavy ticks), `knob_808` (13 x 13; flat red or pale cap in a
  black collar), `knob_909` (12 x 12; dark cylinder, orange index, 11 fine ticks).
  `knob_scale` places a gray knob's scale marks and caption at fixed offsets from the knob.
- Faders (`fader`): 11 tick lines, a dark slot, a dark handle of 15 x 11 with a white line.
- Push buttons (`pill`): a silver face with a light bevel in a dark rim, with a soft shadow.
- LEDs (`led_round`): round, with a dark ring; lit ones have a highlight and a halo.
- Wood (`wood`): the two 16 px side cheeks, a gradient with a grain filter.

**Transport** (`transport`). Shuffle knob, the dark Sync and MIDI LEDs, Tempo display and arrows,
the Pattern / Song mode switch (`lever`: a chrome bat toggle between a green and a yellow LED),
five keys of 32 x 25 in one dark well (Play lit green), Bar, the Loop switch (lever and LED) with
the "Loop" bracket, Start and Length, and the wordmark (`logo`) at (458, 1) of the section.

**Pattern section** (`pattern_section`). Header with the on/off lamp; the eight pattern keys and
the four bank keys are white keycaps (`keycap`, 14.6 square on a 15.6 pitch, centers at
x 11.6 + 15.6 i, y 30.9 and 46.5 for the patterns, 72.9 for the banks) in thin black wells; the
selected one is tinted red with a red glow. SHUFFLE is one keycap of 15, lit the same way. The
steps display and its arrows are at the original's place. The focus bar right of the section is
a thin red line, bright for the focused row.

**303** (`tb303`). A light upper plate with WAVEFORM (the slide switch centered between the saw
and square icons, the square with a quarter-period tail), the six knobs with their names, and
the EDIT STEP display. Below it a silver bezel around a black frame that holds: PITCH MODE (white
on black, LED, button) over CLEAR; eight white keys with the five black keys, each with its LED
and its silver button; the note and rest cells with their LEDs and the note/pause button; DOWN,
UP, ACCENT, SLIDE (white on black labels over white cells with LED and button); BACK and STEP.

**808** (`tr808`). A dark olive speckled face with 13 thin column lines; LEVEL, TONE, TUNE, DECAY
and SNAPPY knobs; cream chips (`chip808`) for the upper labels LC, MC, HC, CL, MA and for the
legend row AC to CH, the selected one with an orange ring; five slide switches (`slide_v`); 16
glossy step keys in red, orange, yellow and white, each with a small round LED at its top. Right
of the legend: the mod window, a dark well with the kit's splash (76 x 76) and the mod's name.

**909** (`tr909`). A pale face with a faint marbling; dark chips with orange instrument names at
the top (the selected one glows); knobs with their labels; the AC box; the FLAM box with its knob
and button (lit: red with a glow); outlined name labels with a selection LED each; 16 silver step
keys with a light bar (orange, red or green for the values 1, 2 and 3) and the step numbers 1 to
16 in a dark strip under them.

**Mixer channel** (`mixer`, `route_switch`). Header with the mute lamp and the meter; PAN knob
over the level fader on the left; on the right the three routing keys, each a silver key with a
yellow lamp at its right and its name under it (DIST, PCF, COMP, left-aligned at one x); the
DELAY send knob at the bottom right.

**Master** (`master`). Two meters of 10 segments with the scale Clip, -6, -12, -24, -36 outside
them, the fader between them, and the COMP routing key with its label to its left, centered on
the key.

**Effects** (`pcf`, `pcf_labels`, `delay`, `dist`, `comp`). PCF: pattern display and arrows, the
BP / LP switch, four faders with their captions under them. Delay: steps display and arrows, the
triplet / sixteenth switch with the "3" bracket and a note, PAN and F.BACK knobs. Dist: AMOUNT
and SHAPE knobs. Comp: nine round gain reduction LEDs over the scale -40 to +40, RATIO and THRES
knobs.

**Where `mock.py` leaves the spec's tables** (docs/ui_spec.md sections 3 to 9); `mock.py` wins:
- 303 waveform switch: 16 x 12 at (24.65, 18.5), not (24, 10), centered between the plate's left
  edge and the divider, and vertically in the plate.
- Pattern and bank keys: the keycap grid above, not the 17 x 20 cells of the original's grid
  (hit rectangles follow the drawn keys).
- PCF faders at y 50, not 55 (their captions under them need the room).
- Comp knobs at y 57, not 56 (level with the mixer's DELAY knob beside them).
- Effect header meters 14 x 14 at (63, header y + 0.5), not 13 x 16 at (64, 2 or 3).
- No 808 instrument selector knob (unchanged from today): the mod window is in its place.

## 5. Applying it: the art generator (`crates/orb-ui-art`)

Everything in `assets/ui` stays generated; no SVG is edited by hand. The generator draws the same
parts as today in the new look and writes `layout.tsv` with the new rectangles.

1. **Read first:** `assets/ui/README.md` (the part list, sizes, states and how the app composes
   them), docs/ui_spec.md sections 2 to 9 and 19, `crates/orb-ui-art/src/{parts,layout,svg,font}.rs`
   and `crates/orb-ui/src/{art,layout,app,model,behavior,logo_anim,about}.rs`.
2. **Keep the part names and states** wherever the control still exists, so that
   `Panel::part_name` (model.rs) and the app's composition change as little as possible. Sizes
   change (for example the knobs: `knob_22` becomes the 18 x 19 gray knob, a rename is fine).
   Rename or add parts where the new design needs them, and update the app's `KNOBS`, `part_size`
   and tests to match.
3. **Self-contained parts.** Each part is rendered on its own by the app, so each SVG carries its
   own `<defs>` (gradients, filters, clip paths). `svg.rs` needs builders for them. Emit only the
   definitions a part uses.
4. **Bleed.** Today a part is exactly its control's rectangle. Several new parts draw outside it:
   knob cast shadows, LED and lamp halos, the glow of lit pattern keys and of FLAM, the display
   glow, the selected 808 chip's ring, the push buttons' shadows. Give such parts a margin around
   the hit rectangle (the art rectangle larger than the control's), carry it in `layout.tsv` or in
   a table the app shares, and keep hit testing on the control rectangle. Static shadows and
   halos that never change may go into the section background if that is simpler.
5. **Knobs.** Two layers as today: `<knob>_body` (ticks, cast shadow, wall, top; static) and
   `<knob>_pointer` (drawn straight up, rotated by the app about the center). No value arc: set
   the designs' `ring_r` and `ring_w` to 0 (the app already takes 0 as "no arc") or remove the arc
   code if nothing uses it.
6. **Grain.** The mockup lays one noise filter over all gray plates, so no two plates look alike.
   As parts, `bg_pattern` and `bg_mixer` are drawn four times each. Check whether the repeated
   speckle and blotches are visible; if they are, vary them per instance (a seed or an offset per
   row).
7. **Labels in Inter, as outlines.** The fonts are committed with the generator (section 9). The
   generator turns every label into path data at generation time; the written SVGs contain no
   `<text>` and need no font at run time (orb-ui builds resvg with `default-features = false`:
   no text support). usvg can do the conversion: `usvg::Tree::to_string` with
   `WriteOptions { preserve_text: false, .. }` writes text as paths (resvg 0.49, the version
   orb-ui-art already depends on). Load only the bundled fonts into the font database (no system
   fonts) so the output is the same on every machine and the "committed files equal the
   generator's output" test keeps holding. The weights used are 400, 500 and 700; letter spacing
   is used on most labels. The wordmark keeps the stroke glyphs of `font.rs`; the seven-segment
   digits are shapes. `font.rs`'s stroke font may go for everything but the wordmark.
8. **The wordmark.** `LOGO_PIECES` keeps its seven names; each piece stays on the whole 134 x 48
   canvas at (474, 1). The glyph layout is unchanged (the run starts at x 8 with scale 2.33, 338
   at x 92), so `OPEN_TRAVEL` and the mask at `RB_X` in `logo_anim.rs` stay right. `logo_piece()`
   has each piece's drawing: the stroke is 1.02 wide (was 0.84) with a dark copy below-right and
   a light copy above-left; the underline bars are 28.4 x 2.6 at (8 + 29.9 i, 27.6).
9. **The mod window.** `mod_view` keeps its control; the app paints the splash and the name. The
   well is 84.4 x 106 at (277.2, 4.2) of the 808 section with the 76 x 76 picture at (281.4, 7.9).
10. **`panel.svg` and the previews.** `layout::compose` keeps composing the demo panel from the
    parts; make its demo state the one of section 3 so that `ui-art check`'s `preview/panel_2x.png`
    can be compared with `reference/panel_2x.png`.

## 6. Applying it: the app (`crates/orb-ui`)

1. **Layout.** `layout.tsv` moves every control. Update what is hard-coded beside it: the
   seven-segment cell (9 x 16) and the display code in `App::control`, the drum strips and the
   instrument selection (`drum_strips`, `instrument_selection`: step cells, legend cells, the
   `sel_*` frames), `mod_view`'s inner rectangle, the focus bars, `layout::backgrounds`, and the
   tests with positions in them (`crates/orb-ui/tests/*.rs`, `layout.rs`'s own tests).
2. **Hit areas.** Keep the behavior of docs/ui_spec.md (what a click does); the rectangles follow
   the drawn art. The 808 legend and the 909 legend and name rows select as today.
3. **Header meters.** The art has four segments, the engine's meter byte has 11 states today
   (`meter_8x14_0..10`). Map the 11 states onto five (0 to 4 segments) and say how in
   docs/ui_spec.md. The master meters keep 10 segments and 11 states.
4. **Routing keys.** `mixsw_*_on` is the key at rest with its lamp lit; there is no pressed look.
5. **Pressed buttons.** The `pressed` state of every push button uses variant F (section 1).
6. **About** (`about.rs`; `about_plate()` in `anim.py` has the numbers). The wordmark at 2x sits
   on a gray plate of 268 x 92 at (92, 14) of the content (a new part, with the plate's bevel and
   a soft shadow). The icon moves to y 28 (centered on the plate); the text rows move 6 points
   down so the version line clears the plate (grow `CONTENT_H` to match). Window background, text
   and link colors and the icon stay as they are. The animation is unchanged.
7. **The backdrop** (`App::show` fills the window with `palette::BG` today). In a window whose
   shape is not the panel's, draw the desk surface around the unit: a vertical gradient from
   `#1d2023` to `#131517`, a fine grain, a faint horizontal brushing, darker toward the window's
   edges, and the unit's soft cast shadow (stronger on the right and below). It must work at any
   window size, also when the free space is above and below, and must not be rasterized again at
   every size during a resize: do not make it one SVG of the window's size. A gradient painted by
   the app, a small tiling grain and shadow strips beside the unit are enough.
8. **Cost.** The new parts use blurs and `feTurbulence`; the mockup's whole panel takes seconds
   to rasterize at 3x. The app rasterizes each part once per pixel size (`paint_part_with`,
   `PartTextures`) and a window resize asks for new sizes on every step. Measure the cold start
   to the first complete frame and a resize drag before and after (release build, at 1x and 2x)
   and write the numbers into the task's notes. Keep `crates/orb-ui/tests/frame_cost.rs`,
   `part_textures.rs` and `texture_size.rs` meaningful. If the filters make the start or the
   resize clearly worse, fix it inside the SVG foundation first (cheaper filters, the grain only
   in the backgrounds, rasterizing during a resize at the old size until it settles). If only
   baked bitmaps would do, stop and file a `Decide:` task for the user: they chose SVG.
9. **The web build** (m-6, wasm) rasterizes the same parts in the browser. Nothing to do here,
   but do not add anything that only works natively.

## 7. Documents and pictures to update in the same commit

- `assets/ui/README.md`: the look, the part table (sizes, states, bleed), the knob layers without
  the arc, the font and its license, and the origin statement. That statement must stay true:
  the art is drawn from scratch in code and holds no copy of the original's bitmaps and no
  Propellerhead or Roland mark, and since task-179 it follows the original's layout and look
  closely by the user's decision. The label outlines come from Inter (SIL OFL 1.1).
- docs/ui_spec.md section 19: replace the task-115 placement with the new one (what differs from
  sections 3 to 9, the header meter mapping, the About layout, the backdrop).
- `docs/ui_app/panel_*.png`, `assets/readme/screenshot.png` and `assets/readme/logo.gif`: made
  again from the real app (`--screenshot`, `ORB_LOGO_T`; docs/building.md has both).
- `README.md` and `docs/building.md` where they describe the look or list the art's inputs.
- `assets/app_icon.ico` and the icon set are unchanged: the app icon is not part of this task.

## 8. Checks

- `cargo test` (orb-ui-art's "committed files equal the generator" and render checks, orb-ui's
  interaction, layout, texture and frame-cost tests), `cargo clippy --all-targets -- -D warnings`
  and `cargo +nightly fmt --all --check`.
- `cargo run --release -p orb-ui-art -- gen` then `check`: no problems, no `<text>` in
  `assets/ui/*.svg`, and `preview/panel_2x.png` next to `reference/panel_2x.png` shows the same
  panel (positions within half a panel pixel; the grain may differ).
- The app itself, with `--screenshot`, at 1x and 2x and in a wide and a tall window; the logo
  animation on the panel and in About (`ORB_LOGO_T` at the times of the reference strips).
- A sheet of every part in every state (`preview/parts_*.png`) for the user, with the derived
  states of section 3 named in the task's notes.

## 9. Where the files of this folder go

| From | To |
|---|---|
| `fonts/Inter-Regular.ttf`, `fonts/Inter-Medium.ttf`, `fonts/Inter-Bold.ttf` | `crates/orb-ui-art/fonts/` (`git mv`), read by the generator |
| `fonts/LICENSE.txt` | `crates/orb-ui-art/fonts/LICENSE.txt` (`git mv`) |
| `reference/panel_2x.png` | `docs/ui_spec/redesign_mockup_2x.png` (`git mv`), linked from docs/ui_spec.md section 19 as the approved mockup |
| `README.md` (this file) | Its lasting content goes into `assets/ui/README.md` and docs/ui_spec.md section 19; the file is deleted |
| `mock.py`, `anim.py`, `anim_template.html`, `render/`, the other files of `reference/` | Deleted (`git rm`): the generator in `crates/orb-ui-art` replaces them, git keeps the history |

After the moves `mockup/179/` is gone, and `mockup/` with it if it is empty. If a path filter or a
script names `mockup/`, remove that too.

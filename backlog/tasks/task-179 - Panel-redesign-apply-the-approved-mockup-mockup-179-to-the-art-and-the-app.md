---
id: TASK-179
title: 'Panel redesign: apply the approved mockup (mockup/179) to the art and the app'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-11-13T11:21:00Z'
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
- [ ] #1 crates/orb-ui-art generates the redesigned art: `ui-art gen` then `ui-art check` report no problems, the committed assets/ui equals the generator's output, and no file of assets/ui contains a <text> element (labels are Inter outlines made at generation time from the fonts in crates/orb-ui-art/fonts, with no system font involved)
- [ ] #2 The generated panel matches the approved mockup: assets/ui/preview/panel_2x.png next to the mockup's reference/panel_2x.png shows every control, label and section at the same place within half a panel pixel and in the same look; the differences that remain are listed in the notes with the reason
- [ ] #3 The app draws the new panel at 1x and 2x with every control usable as before: hit areas follow the drawn controls, behavior is that of docs/ui_spec.md, shadows, halos and glows that reach outside a control's rectangle are not clipped, pressed push buttons use variant F, and the routing keys show only their lamp when on; screenshots in the notes
- [ ] #4 The wordmark's startup animation plays as before on the panel and in Help > About with the new pieces, and About shows the wordmark on the gray plate with the rest of the window unchanged; ORB_LOGO_T screenshots at the times of the reference strips are in the notes
- [ ] #5 In a window wider or taller than the panel's shape the desk backdrop with the unit's cast shadow is drawn around the panel; a resize drag does not rasterize it again per size
- [ ] #6 Cold start to the first complete frame and a resize drag are measured before and after in a release build at 1x and 2x, the numbers are in the notes, and neither is clearly worse; if that cannot be reached with SVG, a Decide task exists and this task depends on it
- [ ] #7 The states the mockup does not show (README section 3) are drawn in the same style, named in the notes, and visible on assets/ui/preview/parts_2x.png for the user's review; the header meters' mapping from the engine's 11 states to four segments is documented in docs/ui_spec.md
- [ ] #8 assets/ui/README.md and docs/ui_spec.md section 19 describe the new art (parts, sizes, bleed, knob layers, font and its license, a true origin statement); docs/ui_app/panel_*.png, assets/readme/screenshot.png and assets/readme/logo.gif are made again from the app
- [ ] #9 The files of mockup/179 are moved as README section 9 says (fonts and license to crates/orb-ui-art/fonts, reference/panel_2x.png to docs/ui_spec/redesign_mockup_2x.png) and the mockup/ folder no longer exists
- [ ] #10 cargo test, cargo clippy --all-targets -- -D warnings and cargo +nightly fmt --all --check pass; the tests with positions, part sizes or part counts in them are updated, not removed
<!-- AC:END -->

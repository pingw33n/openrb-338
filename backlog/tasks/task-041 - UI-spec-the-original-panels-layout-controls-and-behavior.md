---
id: TASK-041
title: 'UI spec: the original panel''s layout, controls and behavior'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 16:23'
updated_date: '2026-10-27 07:10'
labels:
  - ui
  - re
  - docs
milestone: m-3
dependencies: []
references:
  - docs/song_load.md
  - docs/test_machine.md
priority: high
type: spike
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The UI must reproduce the original panel's layout and behavior (not its art). Write `docs/ui_spec.md`: every section of the panel (transport and global controls, the two TB-303s, TR-808, TR-909, mixer, the four effects, master, pattern/bank selectors, song position and loop displays), each control with its position and size relative to the panel (measured from screenshots of the original, docs/test_machine/ has some; take more on the test machine per doc-001), its value range and detents, its mouse and keyboard behavior (drag direction and sensitivity, fine adjust, click on switches, the 303 step editing modes and keys, the 808/909 step buttons and instrument selector, pattern copy/paste/clear if needed), what it displays, and which engine parameter it drives (the parameter numbers of docs/song_load.md). Sources: the manual (/Volumes/downloads/rebirth_2.01_manual.pdf, or the copy on the CD), screenshots, and Rebirth.exe's panel code in IDA (headless copy, read-only) where behavior is unclear. Scope: what the menu functions below need and the panel itself; from the original menu only load/save song, export, choose RBM and the Vintage flag. Mark each control's behavior [C] (observed or read in the code) or [G].
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/ui_spec.md lists every panel section and control with relative geometry, range, interaction, display and engine parameter
- [x] #2 The 303 and drum step-editing behaviors and the pattern/bank/song-mode controls are specified completely enough to implement
- [x] #3 Screenshots used are committed under docs/ui_spec/ and each behavior is marked [C] or [G]
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Read the manual (pages 16-87, 143-180, 221-226) for the documented behavior.
2. Extract the panel art from Rebirth.exe's JPEG resources (stdlib PE reader in the scratchpad) to get every section's and sprite's native size; never commit the art.
3. In a headless copy of ida/Rebirth.exe.i64: CReBirthBitmaps (0x4287A0) for the bitmap ids, the window layout 0x42A8FE, each view's layout method (vtable slot 22), constructor (child class + bitmap) and poll method (vtable slot 17: which child drives which parameter), the widget classes (knob 0x4A60D0, fader 0x469240, toggle/trigger/repeat/wind buttons, CBitmapSwitch, CABCPositionView), the 303/808/909 step editing, the keyboard translation table (0x492312) and the key handlers.
4. Short app session on the test machine (all redc-win-run permits): full-panel screenshot at native size, overlay of the computed rectangles to verify the geometry, a few experiments (knob drag sensitivity and Shift fine mode measured through a saved .rbs, 909 click/modifier values, 808 drag paint, BACK wrap, pitch-mode reset), then quit and release.
5. Write docs/ui_spec.md (sections, controls with pixel and fractional geometry, ranges, interaction, display, engine parameter, [C]/[G] with addresses) and commit the screenshots used under docs/ui_spec/.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a26d97444af667f04

Evidence so far (2026-10-27):
- Panel art: Rebirth.exe has 64 JPEG resources (type 1053), e.g. 12057 transport 592x50, 12500 303 366x115, 12520 808, 12564 909, 12058 pattern section 70x115, 12060 mixer 70x115, 12061/12064/12063/12065/12569 master/PCF/delay/dist/comp. Ids and frame counts are set in CReBirthBitmaps 0x4287A0; sprite frame width = (W+1)/n - 1 (sub_46DC43).
- Window layout sub_42A8FE: panel 624x510, wood 16 px each side, transport (16,0,592,50), rack rows of 115 px: pattern 70, focus bar 4, device 366, mixer 70; effects column x=526: master 98, PCF 111, delay 93, dist 65, comp 93 high.
- Layout methods (slot 22): transport 0x4298B9, 303 0x43FB06, pattern 0x41A9A0, 808 0x442CDA, 909 0x446E4E, mixer 0x413E78, master 0x412D6B, PCF 0x41BC80, delay 0x407BBC, dist 0x408A5F, comp 0x4040CE. Polls (slot 17): transport 0x42A058, 303 0x43ED76, pattern 0x41AB67, 808 0x441E79, 909 0x445FB0, mixer 0x413F47, master 0x412DC7, PCF 0x41B914, delay 0x407F7A, dist 0x408AF5, comp 0x404396.
- Some coordinates are copy-protection expressions (song_load.md 9); resolved by template-matching the sprites on the background art (pattern LED (5,4), mixer meter (51,4), dist/delay meter x 63-64) and by the column pattern.
- Knob 0x4A60D0/0x4A6437/0x4A64E5: position 0..1000, drag up = increase, full scale = a4 px (303 62, 808 36, 808 selector 40, 909 41, others 64), Shift (GetKeyState 16) x5; value = (127p+500)/1000.
- App session 08:47-08:55 (PID 18424, all 6 permits, released): CUTOFF dragged to 0 then 31 px up saved as 64; RESO Shift+31 px saved as 13; 909 BD: click 0->1, 2 clicks ->2, Ctrl+Alt ->3, Ctrl ->1, Alt on a 2 ->0; 808 drag over 4 empty steps painted 4 notes on BD (default selected instrument BD on both drum machines); BACK from step 1 with length 8 went to 16; pitch mode on reset the edit step to 1 and a key press advanced to 2. Saved file read with tools/rbs.py json, then deleted from C:\re\rb.
- Failed attempts: the first BACK clicks at the right edge of the button did nothing (the hit rect is the 17 px frame, not the 35 px bitmap); rcs screenshot --window cannot capture the MDI child, the full screen was cropped at the transport art's position.

Writing (2026-10-27):
- docs/ui_spec.md written: 15 sections (window and sections, widget classes, transport, pattern sections, TB-303 and step editor, TR-808, TR-909, mixers, master and effects, keyboard, pattern/song mode, menu items in scope, manual vs program, screenshots, open items). Every control table has the section-relative px rectangle and the panel fractions (W=624, H=510); the tables were generated from one geometry list (scratchpad geom.py) that also drew the overlay docs/ui_spec/panel_controls.png, which matches the art at every control.
- Keyboard: sub_492312 translates virtual keys to the framework's codes; the window handler sub_42D1D3, the pattern keys sub_42CE3D, the 303 keys sub_43F673, the 808 tap keys sub_44311B. Two differences from the manual: numpad 4 = Rewind and 5 = Fast forward (manual has them swapped); Note/Pause and the drum AC key are VK_OEM_PLUS ('=' on US), '-' has no code -> OB-13 in docs/original_bugs.md and Decide task-054 (recommendation C: physical US positions plus '=' alias).
- docs/test_machine.md section 10: MDI child cannot be captured with --window, mouse drags work in screen coordinates, Alt modifier unreliable via rcs, Ctrl+S saving, the Save-changes box on taskkill.
- Screenshots committed: docs/ui_spec/panel.png (624x510, 256 colors, 214 KB), panel_controls.png (2x overlay, 306 KB), transport_song_mode.png (26 KB). The extracted JPEG art is not committed.
- Not confirmed ([G], listed in ui_spec.md section 15): repeat-button first step at press, the pitch-mode LED flash counter +54, the bank row hit function, the master right meter x and the dist meter x, the meter scales, the 909 tap keys and drum Shift+Tab, Clear's effect on length and shuffle.

Review round 1 (2026-10-27): send back for four defects, fixed in the amended commit:
1. ui_spec.md logo rect: each number's frame stated (code rect l458 t1 r608 b49 in transport coordinates, clipped to the transport's right edge 592; panel x 474..607).
2. 'Swedish and German layouts' for VK_OEM_PLUS: German dropped, Swedish marked [G], in ui_spec.md section 13 and OB-13.
3. Section 15: added the tempo divisor, wind-button timing, transport flag +76, PgUp/PgDn scroll codes; every item now says what would settle it (code with address, or a test-machine check).
4. task-054's AC line '#1 #1' fixed with the CLI.
Nits: quantized, center; the 808/909 ctrl 2 rows renamed 'Accent (AC) level knob' to match song_load.md.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
docs/ui_spec.md specifies the original panel for rb's UI, read from Rebirth.exe's panel code (headless IDA copy) and checked in a short app session on the test machine (2026-10-27, all 6 redc-win-run permits, released).

- Geometry: panel 624x510 (sub_42A8FE); every section and every control with its pixel rectangle relative to its section and as fractions of the panel. The rectangles come from each view's layout method; the copy-protected coordinates were resolved by template-matching the sprites on the background art. An overlay of all rectangles on the screenshot (docs/ui_spec/panel_controls.png) lines up with the art.
- Interaction: exact widget behavior from the code (knob span per section with Shift x5 fine mode, fader jump/drag, toggle, trigger, repeat-button timing, two-position switches, position grids). Knob drag and fine mode were confirmed by saving a song after drags (CUTOFF 64, RESO 13).
- Step editing: the 303 editor (keys, pitch mode resetting to step 1, Back/Step wrapping over 16 steps whatever the length, flip of Up/Down/Accent/Slide/rest, Clear, tap), the 808 and 909 step buttons (click, drag paint, 909 values with Ctrl/Alt/Ctrl+Alt/Flam, hi-hat exclusion, running light) and the instrument selection areas, confirmed in the app.
- Pattern/bank selection and its bar-line timing, transport (tempo, stop logic, wind, bar and loop arithmetic), pattern and song mode, the keyboard (translation table and all handlers), and the in-scope menu items with their ids.
- Every engine parameter is given as the controller or global-message id of docs/song_load.md.
- New facts elsewhere: OB-13 (Note/Pause and AC on VK_OEM_PLUS, not '-'), test_machine.md section 10. Decision raised: task-054 (keyboard mapping; task-045 should depend on it).
- Open [G] items are listed in ui_spec.md section 15.

Branch worktree-agent-a26d97444af667f04, one commit (docs). Testing: none (docs only).
<!-- SECTION:FINAL_SUMMARY:END -->

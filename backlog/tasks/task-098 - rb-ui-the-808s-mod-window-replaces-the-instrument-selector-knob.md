---
id: TASK-098
title: 'rb-ui: the 808''s mod window replaces the instrument selector knob'
status: Done
assignee:
  - '@supervisor'
created_date: '2026-10-28 08:34'
updated_date: '2026-10-28 08:34'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/app.rs
  - crates/rb-ui-art/src/parts.rs
  - docs/ui_spec.md
priority: low
type: enhancement
ordinal: 118000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) to show the mod's splash.jpg and the mod name inside the TR-808 section where the drum selector knob is, and to remove the knob and its art. The legend row still selects the instrument.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The 808 selector knob, its ring of labels and its knob_sel art are gone; the legend selects the instrument
- [x] #2 A dark well right of the legend shows the playing kit's splash.jpg (the standard mod's for the built-in kit) and the song's mod name, and follows a song open and a mod change
- [x] #3 The user reviewed it in the running app; cargo test and clippy pass
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb-ui-art: the selector knob, its 12-position ring in bg_tr808 and the knob_sel design are removed; a new part mod_view (84 x 105 dark well, MOD_VIEW_808 at section (278, 5)) is laid out as control tr808.mod_view, kind modview. rb-ui: App::mod_view paints the kit's splash.jpg (session::mod_file, new; decoded with the image crate's jpeg feature, cached per Kit) in a 76 x 76 square and the song's mod name under it; the standard mod's splash for the built-in kit. The model's selector knob handling (behavior::selector_instrument/value, knob span 40) and its tests are removed; instrument selection is the legend's. docs/ui_spec.md and assets/ui/README.md updated. Screenshots checked for the standard kit and PBE2; the user reviewed the change and asked to drop the knob art, commit and push (2026-10-28). Full suite 482 pass, 0 fail; clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->

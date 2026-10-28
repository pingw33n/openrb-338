---
id: TASK-086
title: 'Decide: where rb-ui switches ''Play Mod Startup Sound'''
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-27 19:58'
updated_date: '2026-10-28 05:41'
labels:
  - decision
  - ui
milestone: m-3
dependencies: []
references:
  - docs/ui_spec.md
  - crates/rb-ui/src/settings.rs
  - crates/rb-ui/src/menu.rs
priority: low
type: spike
ordinal: 106000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-074 made rb-ui play the new mod's startup sound after a mod change when the preference 'Play Mod Startup Sound' is on, as the original (sub_426FB3 -> sub_426C67, which tests app byte +0x533 through sub_4098C3 at 0x426C85). The preference is on by default in the original (the preferences' constructor sub_41CAA4 sets prefs +536 = 1 at 0x41CB13; the app constructor sub_40D1D2 sets +0x533 = 1 at 0x40D338), and the original switches it in its Preferences dialog (Edit > Preferences..., the checkbox 'Play Mod Startup Sound: Enable'; stored at byte 0x20A of the prefs file, docs/test_machine.md section 5). rb-ui now keeps it in its settings file (Settings::startup_sound, key startup_sound=0|1, default 1) but has no control for it: the menu is the user's reduced menu of 2026-10-26 (docs/ui_spec.md section 18: song files, mods, the Vintage flag, the keyboard mode), so adding an item is the user's call.

Options:
1. An Options menu checkbox 'Play Mod Startup Sound' next to 'Vintage ReBirth Synth Sound' (a toggle that writes the settings file at once).
2. A checkbox in the Mods menu, below the mod list.
3. No control: the settings file only (edit startup_sound=0 by hand).

Recommendation: option 1. It is one line in menu.rs (Cmd::ToggleStartupSound: flip Settings::startup_sound and Settings::save, the error to the notice line), it sits with the other sound option, and the logo click (which plays the sound whatever the preference, as sub_42A799 does) stays as it is.
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Decided by the user on 2026-10-28: option 1, an Options menu checkbox 'Play Mod Startup Sound' next to 'Vintage ReBirth Synth Sound', a toggle that flips Settings::startup_sound and saves the settings file at once (a save error goes to the notice line). The logo click keeps playing the sound whatever the preference. Implemented by task-089.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-089
title: 'rb-ui: Options menu checkbox ''Play Mod Startup Sound'''
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:41'
updated_date: '2026-10-28 05:44'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-086
references:
  - docs/ui_spec.md
  - crates/rb-ui/src/menu.rs
  - crates/rb-ui/src/settings.rs
priority: low
type: feature
ordinal: 109000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user decided in task-086 (2026-10-28) that rb-ui switches the preference 'Play Mod Startup Sound' with an Options menu checkbox. task-074 and task-087 made rb-ui play the mod's startup sound on a mod change, on a song open and at start when Settings::startup_sound is on (settings file key startup_sound=0|1, default 1), but there is no control for it: it can only be changed by editing the settings file. The original switches it in Edit > Preferences... ('Play Mod Startup Sound: Enable', app byte +0x533, prefs file byte 0x20A; docs/test_machine.md section 5).

What: add a checkbox 'Play Mod Startup Sound' to the Options menu next to 'Vintage ReBirth Synth Sound' (menu.rs). Toggling it flips Settings::startup_sound and calls Settings::save at once; a save error goes to the notice line, as the other settings writes do. The logo click keeps playing the sound whatever the preference (sub_42A799). Update docs/ui_spec.md section 18 (the reduced menu) to list the new item.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Options menu shows a 'Play Mod Startup Sound' checkbox next to 'Vintage ReBirth Synth Sound', checked when Settings::startup_sound is on
- [x] #2 Toggling it flips Settings::startup_sound and saves the settings file at once; a save error is shown on the notice line
- [x] #3 A test covers the toggle (the setting flips and the saved file has startup_sound=0 then 1)
- [x] #4 docs/ui_spec.md section 18 lists the new item; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Cmd::ToggleStartupSound in menu.rs: flips Settings::startup_sound, Settings::save, error to notice. 2. Options checkbox after Vintage. 3. Test in tests/menu.rs with settings.file in a temp dir. 4. ui_spec section 18 table + drop 'no switch yet' sentence.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-abec9a1623d2a3cef

Cmd::ToggleStartupSound in menu.rs; checkbox after Vintage; test the_startup_sound_checkbox_flips_the_preference_and_saves_it (tests/menu.rs) checks startup_sound=0 then 1 in the file and a notice on an unwritable file. fmt, clippy -D warnings and cargo test --release -p rb-ui pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Options > Play Mod Startup Sound checkbox added (menu.rs), flips Settings::startup_sound, saves at once, save error to notice. Test added; ui_spec section 18 updated. Branch worktree-agent-abec9a1623d2a3cef, one commit. Tests: rb-ui all pass, clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-099
title: 'rb-ui: the mod''s startup sound plays only when the mod changes'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:38'
updated_date: '2026-10-28 08:53'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/menu.rs
  - docs/ui_spec.md
priority: medium
type: enhancement
ordinal: 119000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that the mod's startup sound not play when the mod did not actually change. Example: Mods > Demo Song opens the current mod's demo, so the mod stays the same and its sound should not play. Today rb plays the sound after every mod change, every song open and at start when Settings::startup_sound is on (App::mod_opened, via settle; docs/ui_spec.md 18.1 records the original's rule, which rb now departs from).

What: play the startup sound only when the kit that plays after the change differs from the kit that played before it: compare the resolved Kit (doc.kit: Standard or Mod(path)) before and after. This covers a song open, File > New, Demo Song and a Mods menu change (choosing the mod that is already current plays nothing). At start the previous kit is none, so the first song's mod plays as today. The preference still gates it. The logo click keeps playing the sound whatever the preference and whatever changed (sub_42A799). Record the rule in docs/ui_spec.md 18.1 as rb's deviation from the original, at the user's request, with the date.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Opening a song, File > New, Demo Song or a Mods menu choice plays the startup sound only when the resolved kit changes; at start it plays as before; the logo click still always plays
- [x] #2 Tests cover: same-mod song open (no sound), Demo Song (no sound), a mod change (sound), File > New from a mod song to Standard (sound) and from Standard (no sound)
- [x] #3 docs/ui_spec.md 18.1 records the rule as rb's deviation; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. App gets a private flag sound_known (no song settled yet). 2. mod_opened takes changed: settle compares the kit before (doc.kit) with the new one, or first settle; use_kit plays only if kit != doc.kit. 3. Tests in crates/rb-ui/tests/menu.rs for the five cases. 4. docs/ui_spec.md 18.1 deviation. 5. fmt, clippy, tests.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a9ef4b3dd9fb0b979

Implemented as App::settled (pub(crate) bool, not sound_known as the plan says): settle plays only if !settled || doc.kit != kit; use_kit plays only if kit != doc.kit. Existing test file_new_plays_the_startup_sound... changed to start from a PBE2 song (Standard to Standard now plays nothing). New test the_startup_sound_plays_only_when_the_kit_changes covers first open (sound), same-mod reopen, New from Standard, mod change, same mod again, Demo Song, New from a mod, preference off. Mod-dependent tests ran, not skipped (RB_STANDARD_MOD and RB_MOD_DIR set; PBE2.rbm used). cargo test --release -q: 483 pass, 0 fail; clippy clean; fmt run.

Review round 1: fixed the 18.1 paragraph that said every open and File New plays; reworded the deviation (original's same-mod case not verified); updated settle and fresh comments; a refused start song now sets App::settled; tests added for a failed mod change (no sound, kit kept) and a song that resolves to the current mod (no sound). The adopt_as fallback to Standard was not tested: the test harness resolves mods through the env mod folder, so a mod that fails to load is hard to arrange. Rerun with env: cargo test --release -q 483 pass (mod tests not skipped), clippy clean.

Review round 2: the open after the mod change only raised the unsaved prompt, so the asserts checked nothing. Now the test answers the prompt (DontSave) and asserts doc.path == song, not modified, kit. The song resolves to Standard (it names the standard kit), so from a mod song the open is a kit change and plays: this is the resolved-kit case. Env tests: 483 pass, clippy clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The startup sound plays only when the resolved kit changes (App::settle, App::use_kit in crates/rb-ui/src/menu.rs; App::settled in app.rs, also set by a refused start song; logo click unchanged). docs/ui_spec.md 18.1 records the deviation (2026-10-28) and the older paragraph is brought in line. Tests: new the_startup_sound_plays_only_when_the_kit_changes (incl. failed mod change), adapted file_new test; cargo test --release -q 483 pass with mod env set, clippy -D warnings clean. Branch worktree-agent-a9ef4b3dd9fb0b979, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-099
title: 'rb-ui: the mod''s startup sound plays only when the mod changes'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:38'
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
- [ ] #1 Opening a song, File > New, Demo Song or a Mods menu choice plays the startup sound only when the resolved kit changes; at start it plays as before; the logo click still always plays
- [ ] #2 Tests cover: same-mod song open (no sound), Demo Song (no sound), a mod change (sound), File > New from a mod song to Standard (sound) and from Standard (no sound)
- [ ] #3 docs/ui_spec.md 18.1 records the rule as rb's deviation; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

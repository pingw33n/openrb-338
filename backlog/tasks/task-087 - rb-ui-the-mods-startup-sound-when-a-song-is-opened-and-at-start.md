---
id: TASK-087
title: 'rb-ui: the mod''s startup sound when a song is opened and at start'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-27 19:58'
updated_date: '2026-10-27 20:05'
labels:
  - ui
  - re
milestone: m-3
dependencies:
  - TASK-074
references:
  - docs/export_state.md
  - docs/ui_spec.md
  - crates/rb-ui/src/menu.rs
priority: low
type: feature
ordinal: 107000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the original plays a mod's startup sound not only after a Mods-menu change (task-074) but also from the song load: sub_426C67 (msg 58 when the mod record's word +1088, its user count, is 1 and the preference 'Play Mod Startup Sound' is on) is called from sub_425C4D at 0x425FAB and 0x426400 and from sub_426CA8 at 0x426D48 (docs/export_state.md section 8). So opening a song whose mod is not already open by another song, and probably the app's start with its Untitled song on 'Standard ReBirth', play the startup sound. rb-ui does not do this yet (task-074 covered the mod change and the logo).

Do: read sub_425C4D, sub_426CA8 and their callers in the disassembly (which paths: open, new song, the start), note which of them call sub_426C67 and when the user count is 1 (one song in rb-ui: the new song's mod is opened before the old one is released? check the order against sub_416300 / sub_4165BE, docs/rbm.md section 14.1). Then call Link::startup_sound after Link::load in App::adopt (and at start, if the original does) when Settings::startup_sound is on, with a test in crates/rb-ui/tests/menu.rs like a_mod_change_plays_the_startup_sound_when_the_preference_is_on. Document the paths in docs/ui_spec.md section 18.

Acceptance: the paths are documented with addresses; rb-ui plays the startup sound where the original does; a test covers it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the load paths that send msg 58 are documented with addresses in docs/ui_spec.md
- [ ] #2 rb-ui plays the startup sound on those paths when the preference is on, with a test
<!-- AC:END -->

---
id: TASK-056
title: 'RE: how the original applies a mod to an open song'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-27 07:34'
updated_date: '2026-10-27 07:49'
labels:
  - re
  - rbm
  - ui
milestone: m-3
dependencies:
  - TASK-043
references:
  - docs/rbm.md
  - docs/ui_spec.md
  - crates/rb-player/src/lib.rs
priority: medium
type: task
ordinal: 51000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the Mods menu (docs/ui_spec.md section 12: "Select Mod...", one item per mod) applies a mod to the open song. rb knows only that a song is created with its sample set (msg 52, crates/rb-engine/src/song.rs Song::mod_name; docs/rbm.md). The real-time player (task-043, crates/rb-player) builds a song with a sample set (Player::build / Player::load); it cannot change the mod of a song that plays, because how the original does it is not known: a new song object (msg 52 + reload, with the activation rules of docs/export_state.md section 3), a message that swaps the drum machines' samples in place, or something else, and what happens to the playing voices, the position and the process globals.

Do: find in Rebirth.exe the Mods menu handler (command 403 and the per-mod items) and what it sends to the DLL; read the DLL side; write the facts with addresses into docs/rbm.md; then make rb-player apply a mod as the original does (a Command, or Player::load of the rebuilt song, whichever the original's behavior is), with a test; the callback must stay allocation-free (crates/rb-player/tests/no_alloc.rs).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/rbm.md says, with addresses, what the Mods menu sends and what the DLL does with a playing song
- [ ] #2 rb-player applies a mod as the original does, with a test; the callback stays allocation-free
<!-- AC:END -->

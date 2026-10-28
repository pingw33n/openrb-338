---
id: TASK-094
title: 'rb-ui: Mods > Demo Song opens the current mod''s Demo.rbs'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:51'
updated_date: '2026-10-28 05:51'
labels:
  - ui
  - rbm
milestone: m-3
dependencies:
  - TASK-090
references:
  - crates/rb-ui/src/menu.rs
  - docs/rbm.md
  - crates/rbm
priority: low
type: feature
ordinal: 114000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for Mods > Demo Song, which loads the demo song stored in the mod, if any. Every shipped mod carries an embedded Demo.rbs; the standard mod does not (docs/rbm.md, the Demo.rbs paragraph: the original only enables a button of its mod info dialog, sub_41A0C5 at 0x41A3E0, that saves it, sub_419E92 -> sub_417022; sub_417000 tests sub_416710("Demo.rbs") != 0).

What: add a Mods menu item 'Demo Song' (below the mod list or Select Mod...), enabled only when the current song's mod has a Demo.rbs. Choosing it opens the demo as a new, never-saved song (Untitled or the name 'Demo', so Save goes to Save As and never writes into the mod) through the same path as File > Open: the unsaved-changes prompt, the stop of playback (the task for opening while playing), the pool refusal (task-088). The song's mod is the mod it came from. Read the file from the recrypted mod with the rbm crate's file lookup.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Mods > Demo Song is enabled only when the current mod has Demo.rbs and disabled for the standard mod
- [ ] #2 Choosing it loads the demo as an unsaved song with that mod, through the open path (prompt, stop, refusal)
- [ ] #3 A test loads a mod's demo (from RB_MOD_DIR, skipped when absent, as the other mod tests do) and checks the disabled case
- [ ] #4 docs/ui_spec.md section 18 lists the item; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

---
id: TASK-094
title: 'rb-ui: Mods > Demo Song opens the current mod''s Demo.rbs'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:51'
updated_date: '2026-10-28 06:53'
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
- [x] #1 Mods > Demo Song is enabled only when the current mod has Demo.rbs and disabled for the standard mod
- [x] #2 Choosing it loads the demo as an unsaved song with that mod, through the open path (prompt, stop, refusal)
- [x] #3 A test loads a mod's demo (from RB_MOD_DIR, skipped when absent, as the other mod tests do) and checks the disabled case
- [x] #4 docs/ui_spec.md section 18 lists the item; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. session.rs: open_bytes (shared with open_song), mod_demo(path) reads Demo.rbs from a recrypted mod with rbm file_by_name + embedded key table; mod_has_demo(path).
2. menu.rs: Then::Demo(mod path), Cmd::Demo, App::demo_song (prompt when modified, like open_song), open_demo_now (read, adopt with forced kit/mod name from the mod, no path = Untitled, so Save goes to Save As), refusal/stop via the same Link::load as adopt. Cache of has_demo keyed by mod path. Menu item 'Demo Song' below Choose file..., disabled for the standard kit and mods without Demo.rbs.
3. Tests in tests/menu.rs (RB_MOD_DIR, skip when absent): disabled for standard, enabled for PBE2, loads demo unsaved with the mod, prompt, stop, no write to the mod on Save.
4. docs/ui_spec.md section 18 table + notes.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a0844f22a3548b285

Evidence: cargo test --release -q (workspace) 457 passed, 0 failed, with RB_STANDARD_MOD and RB_MOD_DIR set (recrypted-mods: Metallicon, MSM2, Orbit2, PBE2); the mod tests ran, no 'skipped' line printed. New test the_demo_song_of_the_current_mod_opens_as_an_unsaved_song (tests/menu.rs): standard kit and the standard mod file have no demo, PBE2 has; prompt (Cancel keeps, Don't Save opens), playing stops, Untitled, unmodified, kit/mod name = the mod, Save goes to the browser, mod file bytes unchanged. Pool refusal: the demo goes through App::adopt_as, the same Link::load as File > Open (tested by a_song_the_node_pool_refuses_is_not_opened); not separately tested for a demo (no mod carries a huge demo). Menu enabled state is has_demo (tested), the item is drawn with add_enabled(has_demo) and not exercised by a pixel/click test. cargo clippy --all-targets -D warnings clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Mods > Demo Song added below Choose file...: enabled only when doc.kit is a mod file with an embedded Demo.rbs (App::has_demo, cached per mod file), so disabled for the standard kit. It goes through the open path: demo_song asks the unsaved prompt (Then::Demo), open_demo_now reads Demo.rbs from the recrypted mod (session::mod_demo, rbm file_by_name with the embedded key table) and calls App::adopt_as (the refactored tail of adopt: Link::load stops the playing song, node-pool refusal alert). The song has no file (Untitled, unmodified), so Save goes to Save As and never writes into the mod; its kit and mod name/URLs are the mod's. session::open_bytes now backs open_song. docs/ui_spec.md section 18 has the table row and a Demo Song bullet. Tests: workspace cargo test --release 457 pass, clippy clean. Branch worktree-agent-a0844f22a3548b285, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

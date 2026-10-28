---
id: TASK-101
title: >-
  rb-ui: the Mods menu caches the mod list, loads it at start, and has Refresh
  List
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
  - crates/rb-ui/src/app.rs
priority: medium
type: enhancement
ordinal: 121000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that the Mods menu's list of mods be cached, loaded at start, and reloaded only with a new Mods > Refresh List item. Today App::mod_list is filled lazily (list_mods when the menu is drawn and mod_list is None) and read from the mods folder.

What: load the mod list once in App::start (and in App::fresh if that is where start state lives), keep it in App::mod_list, and add Mods > Refresh List, which reloads it and clears the per-mod caches that depend on the folder's files (demo_cache; the splash cache of App::mod_view if it is keyed by file). Also reload it when the mods folder setting changes (the browser's 'Use as mods folder'). A load error stays shown in the menu as today. The menu draw never reads the folder. Update docs/ui_spec.md section 18.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The mod list is read at start and on Refresh List or a mods-folder change only; drawing the menu reads no files
- [ ] #2 Mods > Refresh List picks up a mod added to and one removed from the folder (test with a temp folder copy of RB_MOD_DIR mods, skipped when absent and recorded as run)
- [ ] #3 docs/ui_spec.md section 18 lists Refresh List; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

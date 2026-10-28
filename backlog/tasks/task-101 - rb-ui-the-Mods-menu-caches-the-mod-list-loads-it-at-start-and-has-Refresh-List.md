---
id: TASK-101
title: >-
  rb-ui: the Mods menu caches the mod list, loads it at start, and has Refresh
  List
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:38'
updated_date: '2026-10-28 09:26'
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
- [x] #1 Mods > Refresh List picks up a mod added to and one removed from the folder (test with a temp folder copy of RB_MOD_DIR mods, skipped when absent and recorded as run)
- [x] #2 docs/ui_spec.md section 18 lists Refresh List; cargo test and cargo clippy --all-targets -- -D warnings pass
- [x] #3 The mod list is read at start and on Refresh List or a mods-folder change only; drawing the menu reads no folder (has_demo reads the current mod file once per file)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. menu.rs: App::refresh_mods() = mod_list=Some(list_mods()), demo_cache=None, splash=None; called in start() and when a browser answer changed settings.mods_dir (App::browse). Remove the lazy load, the close-resets (mods_open) and the settle reset of mod_list. 2. Cmd::RefreshMods + Mods > Refresh List item. 3. Menu draw reads no files except has_demo (cached; cleared by refresh). 4. Tests in tests/menu.rs: add/remove a mod in a temp folder + Refresh List; menu draw does not re-read. 5. ui_spec section 18.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-af983e8688ef27276

Implemented App::refresh_mods (menu.rs): list, demo_cache and splash cleared/reloaded; called from App::start, Cmd::RefreshMods and App::browse when settings.mods_dir changed. Removed the lazy load, the menu-close reset and the settle reset. Test refresh_list_reads_the_mods_folder_again (tests/menu.rs) adds/removes mods in a temp folder; its demo part proves the demo cache is kept until the refresh. The splash cache is cleared by the same fn but is private to the app, so it is not asserted. Caveat: has_demo still reads the current mod file once per mod file when Demo Song is first drawn (cached per file); the list itself is never read in a draw. Full run: RB_STANDARD_MOD and RB_MOD_DIR set, no 'skipped' output from the menu tests (nocapture), cargo test --release: 485 pass, 0 fail, 2 ignored; clippy -D warnings clean; fmt run.

Review round 1: (1) ui_spec wording fixed: the menu draw reads no folder, a song open still resolves its mod via mods::find_in_dir. (2) commit Result reworded. (3) the test now opens the Mods menu in frames (click along the bar until the shapes contain 'Refresh List') over a changed folder and asserts mods() unchanged; shown to fail with a temporary 'self.mod_list = Some(self.list_mods())' in the Mods closure (left [MSM2, PBE2], right [PBE2]), then reverted. (4) AC1 reworded. Not tested: the browser path (app.rs browse, mods_dir change). With no start, mod_list is None (the headless state) and the menu shows no entries.

Review round 2: rewrapped the ui_spec paragraph to at most 100 columns; reworded the test's doc comment (drawing the menu reads no folder, a song open does not fill the list). Docs and comments only; tests not re-run.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Mods menu list is cached in App::mod_list, read by App::refresh_mods at App::start, on Mods > Refresh List and on a mods-folder setting change from the browser (not tested); refresh also clears demo_cache and splash. Drawing the menu reads no folder (has_demo reads the current mod file once per file); a song open still resolves its mod in the folder. Test refresh_list_reads_the_mods_folder_again opens the Mods menu over a changed folder (fails with a read reinstated in the menu closure). docs/ui_spec.md section 18 updated. cargo test --release: 485 pass; clippy clean. Branch worktree-agent-af983e8688ef27276.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-144
title: 'File dialogs remember the last folder used, separately for songs and mods'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-29 14:29'
updated_date: '2026-10-30 08:15'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/orb-ui/src/browser.rs
  - crates/orb-ui/src/settings.rs
  - crates/orb-ui/src/menu.rs
  - docs/ui_spec.md
priority: medium
type: enhancement
ordinal: 163000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked on 2026-10-29 that the in-app file dialogs (crates/orb-ui/src/browser.rs, task-046) remember the last folder used, with the song dialogs and the mod dialog keeping separate folders. The task was created unassigned at the user's request.

Today (docs/ui_spec.md section 17, "Defaults and settings"): a dialog starts in the settings' songs folder (songs and export) or mods folder (mods), else the documents folder. "Use as songs/mods folder" sets those. The last 8 folders a file was chosen in are kept as recent folders. Some opens start in the open song's folder (Browser starting in dir, browser.rs:224). Nothing remembers where the user last was if they navigate elsewhere and choose a file.

What:
- Two remembered folders, stored in ui.toml (settings.rs, the same atomic rewrite):
  - `last_song_dir`: the folder of the last song file chosen in Open, Save As or Export (decide and document whether Export shares it with the song dialogs or has its own; recommended: shared);
  - `last_mod_dir`: the folder of the last mod chosen in the mod dialog.
  A path is remembered only when a file is actually chosen (Open, Save, Export or choosing a mod), not on Cancel.
- Where a dialog starts: the remembered folder of its kind if it still exists, else today's rule (the songs or mods folder setting, else the documents folder). Decide and document how the existing "start in the open song's folder" case fits: recommended, the remembered folder wins, and Save of a song that has a file keeps its folder as now.
- The songs and mods folder settings keep their meaning (the default place and the Mods menu's folder); the remembered folders don't change them.
- A non-UTF-8 path is left out on save, like songs_dir. A missing remembered folder falls back silently.
- Tests: settings round-trip of both keys; each dialog kind starts in its own remembered folder; Cancel remembers nothing; a deleted folder falls back. Update docs/ui_spec.md section 17 and the settings.rs crate docs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The song dialogs and the mod dialog each start in the folder of the last file chosen in their kind (kept in ui.toml as separate keys), else today's default; Cancel remembers nothing; a missing folder falls back
- [x] #2 Tests cover both kinds, Cancel, the fallback and the settings round-trip; docs/ui_spec.md 17 updated; fmt, clippy, debug cargo test -p orb-ui pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. settings.rs: new fields last_song_dir, last_mod_dir (own load/save lines; non-UTF-8 left out).
2. browser.rs: Browser::new starts in the remembered folder of its kind if it is a folder, else the old rule; finish() records the chosen file's folder (songs and export share last_song_dir; mods use last_mod_dir), saved with the recent note. Cancel and errors remember nothing.
3. Decisions: Export shares last_song_dir; the remembered folder wins over the songs_dir setting. with_dir stays an explicit override (no app caller today; Save of a song with a file saves in place, no dialog).
4. Tests in tests/browser.rs; docs/ui_spec.md 17 and settings.rs docs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ad135bd6bea0ecc33 (rebased on main 94e5371).

Evidence: tests each_dialog_kind_remembers_its_own_last_folder, cancel_and_failed_choices_remember_nothing, a_deleted_remembered_folder_falls_back_silently plus the settings round-trip with both keys; fmt, clippy, debug cargo test -p orb-ui and release cargo test --no-fail-fast all pass. The mod test copies assets/Standard.rbm (a recrypted mod). First draft of the test failed because Enter needs a selected row; fixed with select(0).

Review round 1 (verdict merge, with small fixes): with_dir doc reworded as an explicit override used by tests; settings.rs test covers last_song_dir and last_mod_dir non-UTF-8; ui_spec 17 paragraph rewrapped to 100 columns; added that the folder is remembered at the choice, before the open or write. fmt, clippy, tests browser and settings pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Branch worktree-agent-ad135bd6bea0ecc33, one commit. settings.rs: last_song_dir and last_mod_dir (own load/save lines, non-UTF-8 omitted). browser.rs: Browser::new starts in the remembered folder of its kind while it is a folder, else songs_dir/mods_dir, else documents; finish() records the chosen file's folder with the recent note (Cancel and refused choices record nothing). Decisions: Export shares last_song_dir with Open and Save As; the remembered folder wins over the configured folder; the 'open song's folder' case does not exist in the app (with_dir has no caller; Save of a song with a file writes in place), with_dir stays an explicit override. docs/ui_spec.md 17 and settings.rs docs updated. Tests: 3 new in crates/orb-ui/tests/browser.rs plus the round-trip extended; fmt, clippy, debug cargo test -p orb-ui, release cargo test all pass.
<!-- SECTION:FINAL_SUMMARY:END -->

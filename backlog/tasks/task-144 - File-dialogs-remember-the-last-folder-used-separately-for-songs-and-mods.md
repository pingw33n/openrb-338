---
id: TASK-144
title: 'File dialogs remember the last folder used, separately for songs and mods'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-29 14:29'
updated_date: '2026-10-30 08:03'
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
- [ ] #1 The song dialogs and the mod dialog each start in the folder of the last file chosen in their kind (kept in ui.toml as separate keys), else today's default; Cancel remembers nothing; a missing folder falls back
- [ ] #2 Tests cover both kinds, Cancel, the fallback and the settings round-trip; docs/ui_spec.md 17 updated; fmt, clippy, debug cargo test -p orb-ui pass
<!-- AC:END -->

---
id: TASK-151
title: >-
  Mods sharing an INFO name: a submenu of variants, a remembered preference,
  byte-order default
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-30 07:43'
updated_date: '2026-10-30 08:19'
labels:
  - ui
  - cli
  - rbm
milestone: m-5
dependencies:
  - TASK-146
references:
  - crates/orb-cli/src/mods.rs
  - crates/orb-ui/src/menu.rs
  - crates/orb-ui/src/settings.rs
  - docs/rbm.md
  - docs/ui_spec.md
  - crates/orb-cli/tests/thirdparty.rs
priority: medium
type: feature
ordinal: 169000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user decided task-146 on 2026-10-30. Several mod files can carry the same INFO name. The user's collection, ~/rb-mods/mods (171 recrypted mods, read only), has 4 such pairs: AXIOM (AXIOM.rbm, axiom(b3).rbm), dejenerate (dejenerate.rbm, dejenerate(b3).rbm), Alpha 2.0 (Alpha2.rbm, alpha3.rbm) and AlpJunUV (AlpJunUV.rbm, UVJ.rbm). Names match ignoring ASCII case (task-145, task-150), so names differing only in case count as the same name.

Default choice (CLI and UI, when no preference is set): of the files sharing a name, the one whose file name sorts first by byte values, a plain strcmp over the file name's bytes. In Rust that is the Ord of the name's bytes (OsStr::as_encoded_bytes, or the UTF-8 str): no Unicode folding, no locale, no NTFS model. This replaces mods.rs's NTFS model (folder_order) for choosing among duplicates. It is a deliberate difference from the original, where the volume's listing order decides (docs/rbm.md 11.3: on NTFS axiom(b3).rbm wins; by bytes 'A' 0x41 < 'a' 0x61, so AXIOM.rbm wins). Decide whether the plain Mods menu order (the non-duplicate entries) also moves to byte order or keeps its current order, and say why. The recommendation is one order everywhere, by bytes, for simplicity.

CLI (orb render, compare, play, info with --mod-dir / ORB_MOD_DIR): the default above. --mod FILE still picks any file. The CLI does not read the UI's settings or preferences, so a render is deterministic.

UI (crates/orb-ui/src/menu.rs, the Mods menu, docs/ui_spec.md 18):
- Mods sharing a name show as one top-level item with that name, opening a submenu. Each submenu entry is a file, shown as its file name, followed by the mod's INFO version when non-blank (for example "axiom(b3).rbm  1.2"). Entries are in byte order.
- The top-level item is checked when the song's active mod is any of the variants (as a single mod's item is today), and the submenu marks the active variant.
- Clicking a variant that is not active plays it for the current song. The song's saved mod name stays the INFO name, which the variants share; the song is marked modified only if the name changes, as today.
- Clicking the active variant makes it the preferred file for that name. Store it in the settings file (crates/orb-ui/src/settings.rs, ui.toml), for example a table preferred_mods = { "<INFO name>" = "<file name>" }, written through the existing atomic save. Show the preference in the submenu (for example a "preferred" mark), and say how it is cleared: clicking the active preferred variant again, or a preference whose file is gone falls back silently.
- Song open, Mods > Refresh List and a mod choice resolve a shared name to: the preference if its file is still in the folder, else the byte-order default.
- Non-UTF-8 names follow settings.rs's rule (left out on save).

Tests:
- mods.rs unit tests: the byte-order default (AXIOM.rbm over axiom(b3).rbm), ASCII-case-insensitive grouping, and that --mod overrides;
- settings round-trip of the preference;
- menu tests: the submenu, the checks, choosing a variant, setting and using the preference, a missing preferred file falling back.

The task-129 gated songs test must stay true to rbhost's renders. rbhost (the original on NTFS) used axiom(b3).rbm for songs naming AXIOM, and the new default picks the other file (AXIOM.rbm); for dejenerate both orders pick dejenerate(b3).rbm (by bytes "(" 0x28 sorts before "." 0x2E). The manifest records the mod file each song used (its "mod" key), so make the test render with that file explicitly (--mod), and check that every manifest song still equals rbhost's.

Update docs/rbm.md 11.3 (the rule and the deliberate difference) and docs/ui_spec.md 18.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CLI: a shared INFO name resolves to the file whose name sorts first by bytes; --mod overrides; the CLI ignores UI preferences
- [x] #2 UI: shared names are one Mods item with a submenu of variants (file name + non-blank version); top-level and submenu checks show the active one; clicking a variant plays it; clicking the active variant sets a preference saved in ui.toml and used on open; a missing preferred file falls back
- [x] #3 Tests cover the default, grouping, override, preference round-trip and menu behavior; the task-129 gated test passes (songs rendered with the manifest's mod file); docs/rbm.md 11.3 and docs/ui_spec.md 18 updated; fmt, clippy, cargo test --release and the corpus (175/175) pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. mods.rs: byte_order (file-name bytes) replaces folder_order; find_preferring(dir,name,pref); find_in_dir = find_preferring(None). Tests: default, grouping, override.
2. settings.rs: preferred_mods table (INFO name -> file name), ASCII-case-insensitive lookup; round-trip test.
3. menu.rs: ModEntry gets variants (file + version); shared names = one SubMenuButton, selected when active, submenu marks active, clicking a non-active variant plays it, clicking the active toggles the preference ((preferred) mark, status line); kit_for uses the preference; use_kit marks modified only when the INFO name changes. Menu tests.
4. thirdparty.rs songs test: --mod with the manifest's mod file.
5. docs/rbm.md 11.3 + 10, docs/ui_spec.md 18. Run fmt, clippy, tests, corpus, gated tests.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a2502490a08da3ab6 (rebased on main 453421a).

Choices (simple options, for the user to confirm): (1) the Mods menu order everywhere is byte order of file names (one order, as recommended); (2) clearing the preference = clicking the active preferred variant again (status line says so); the preferred variant shows '(preferred)' after its label; a gone preferred file is ignored silently and stays in ui.toml; (3) the top-level item is a SubMenuButton shown selected when active; (4) use_kit now marks the song modified only when the INFO name changes (task text). (5) The song-info window / CLI-style resolution outside kit_for ignore the preference (only song open does). Gated thirdparty test (release, 2 tests) passed in 98.8 s with --mod <manifest mod file> for songs with a recorded mod (AXIOMpeace/Alpha2/axiom(b3)). Corpus: 175 of 175 equal. cargo test --release --no-fail-fast: 65 suites 625 passed 0 failed; debug cargo test -p orb-ui: 223 passed. No screenshot: the hidden options cannot open a menu (no flag for it); menu tests assert the model instead (mods_that_share_a_name_are_one_entry_with_variants, the_mods_menu_draws_a_submenu_for_a_shared_name).

Review round 1 (rebased on 94e5371): (1) set_song_mod marks the song modified when the name, mod_ftp or mod_www changes; test a_variant_with_other_urls_marks_the_song_modified. (2) docs/rbm.md fixed: only AXIOM differs from NTFS (dejenerate(b3).rbm first in both); task description corrected through the CLI. (3) CD-free tests with INFO-only mods: shared_names_group_and_resolve_by_preference_then_bytes (list_mods grouping, checks, choose_variant/toggle, ui.toml, stale preference, Refresh List); the draws test is dropped (headless egui cannot open the menu here; the submenu drawing is not covered by tests). (4) App::resolve_mod_file is the one rule (song open + ModEntry::resolve for list entries, Refresh List and preference changes); the top-level shared item is only a submenu opener (stated in ui_spec 18). Debug cargo test -p orb-ui without ORB_CD_ISO: 224 pass; release --no-fail-fast with ORB_CD_ISO: 626 pass; fmt, clippy clean.

Review round 2: commit message only (Change and Testing updated for round 1; gated thirdparty and corpus ran on round-1 code, mods.rs and thirdparty.rs unchanged since; supervisor reruns both at the merge).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Mods that share an INFO name (ASCII case ignored) now resolve to the file whose name sorts first by bytes (mods.rs byte_order replaces the NTFS folder_order; find_preferring added; find_in_dir = default; --mod overrides; the CLI reads no UI settings). The UI lists them as one Mods item with a submenu (file name + non-blank version, byte order), checks the entry and marks the active variant, plays a clicked variant, and makes the clicked active variant the preferred file (ui.toml table preferred_mods, '(preferred)' mark, click again to clear, a missing file falls back). Song open uses the preference. The task-129 gated songs test renders with --mod of the manifest's mod file and still equals rbhost for every listed song. docs/rbm.md 10, 11.3 and docs/ui_spec.md 18 updated. Tests: release gated thirdparty 2 pass; corpus 175/175; cargo test --release 625 pass; debug orb-ui 223 pass; fmt and clippy clean. Branch worktree-agent-a2502490a08da3ab6, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

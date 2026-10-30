---
id: TASK-151
title: >-
  Mods sharing an INFO name: a submenu of variants, a remembered preference,
  byte-order default
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-30 07:43'
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

The task-129 gated songs test must stay true to rbhost's renders. rbhost (the original on NTFS) used axiom(b3).rbm and dejenerate(b3).rbm for songs naming AXIOM or dejenerate, and the new default picks the other file. The manifest records the mod file each song used (its "mod" key), so make the test render with that file explicitly (--mod), and check that every manifest song still equals rbhost's.

Update docs/rbm.md 11.3 (the rule and the deliberate difference) and docs/ui_spec.md 18.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CLI: a shared INFO name resolves to the file whose name sorts first by bytes; --mod overrides; the CLI ignores UI preferences
- [ ] #2 UI: shared names are one Mods item with a submenu of variants (file name + non-blank version); top-level and submenu checks show the active one; clicking a variant plays it; clicking the active variant sets a preference saved in ui.toml and used on open; a missing preferred file falls back
- [ ] #3 Tests cover the default, grouping, override, preference round-trip and menu behavior; the task-129 gated test passes (songs rendered with the manifest's mod file); docs/rbm.md 11.3 and docs/ui_spec.md 18 updated; fmt, clippy, cargo test --release and the corpus (175/175) pass
<!-- AC:END -->

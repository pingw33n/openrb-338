---
id: TASK-146
title: 'Decide: what orb does with mods that share an INFO name'
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-29 14:49'
updated_date: '2026-10-30 07:43'
labels:
  - decision
  - rbm
dependencies: []
references:
  - docs/rbm.md
  - crates/orb-cli/src/mods.rs
  - crates/orb-ui/src/menu.rs
priority: medium
type: spike
ordinal: 164000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked on 2026-10-29 to decide this. A song names its mod by the mod's INFO name, and several mods can carry the same INFO name. The user's collection (~/rb-mods/mods, 172 mods) has 5 such pairs:
- 8tRack: 8tRack.rbm and DJ Turn's 8-Track.rbm (identical files);
- AXIOM: AXIOM.rbm and axiom(b3).rbm;
- dejenerate: dejenerate.rbm and dejenerate(b3).rbm;
- Alpha 2.0: Alpha2.rbm and alpha3.rbm;
- AlpJunUV: AlpJunUV.rbm and UVJ.rbm.
A song naming one of them can mean either file, and the files differ in sound except for 8tRack.

What the original does (docs/rbm.md 11.3, [C] from the disassembly): sub_4158A9 lists the Mods folder with FindFirstFile/FindNextFile. When sub_4155D1 finds a mod's INFO name already in the list (0x415A90), it drops the later file and shows "The MOD "X.rbm" was not added because another Mod with the same name is already installed." So the first file the volume lists wins. On NTFS that is $UpCase order: axiom(b3).rbm before AXIOM.rbm, confirmed on the test machine by an export. On FAT or a network share the order can differ [G].

What orb does today: crates/orb-cli/src/mods.rs (folder_order) models NTFS order: names upcased per character, then compared as UTF-16 units. The first match wins, as in the original. Whether orb warns about the duplicate, in the CLI or in the UI's Mods menu, is to be checked by the implementing task. task-145's decision (an INFO-name match only, case-insensitive ASCII) makes names that differ only in case count as duplicates too. The collection has none of those today.

Options:
A. Keep the original's rule silently: the first file in NTFS order wins, and nothing is shown.
B. The original's rule, plus a warning: the first in NTFS order wins. The CLI warns (orb render/compare/play/info: "mod name X matches A.rbm and B.rbm; using A.rbm, as ReBirth would on NTFS; pass --mod to choose"), and the UI shows it once on the status line when the Mods folder is read, like the original's box. The Mods menu lists only the winner, as the original does.
C. Prefer the file whose name equals the INFO name (AXIOM.rbm over axiom(b3).rbm), and fall back to NTFS order. This is more intuitive, but it differs from the original, so a render can differ from rbhost's.
D. Refuse: a duplicated name is an error that lists the files, and the user must pass --mod (CLI) or pick a file (UI).
E. Show every duplicate in the Mods menu, labeled with its file name, so the user can choose. Songs still resolve by rule A or B.

Recommendation: B. It renders exactly what the original renders, which keeps the reference checks of task-129 meaningful, and it tells the user that the choice was made and how to override it. The UI keeps the original's one-entry menu.

Acceptance: the user's choice is recorded here with the date. If it is not A, a follow-up task implements it (mods.rs, the UI's Mods menu, docs/rbm.md 11.3), with tests on a folder holding a duplicate pair.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The user's choice is recorded with the date, and a follow-up task exists when needed
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor, 2026-10-29: the user removed DJ Turn's 8-Track.rbm (the identical copy of 8tRack.rbm) from ~/rb-mods/mods, so 4 duplicate pairs remain: AXIOM, dejenerate, Alpha 2.0 and AlpJunUV. The manifest entry is dropped in task-149.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Decided by @pingw33n on 2026-10-30, a variant of options B and E:
- UI: mods that share an INFO name (ASCII case-insensitive, per task-145) are grouped in the Mods menu under one item with a submenu. The submenu lists the files by file name, plus the mod's version when it is non-blank.
- The active mod checks the top-level item, and the submenu marks the active variant.
- Clicking a variant that is not active plays it for the song. Clicking the active variant makes it the globally preferred file for that name, which is remembered in the settings file.
- Default, with no preference: the file whose name sorts first by byte values (strcmp on the file name's bytes). This replaces the NTFS-order model for choosing a duplicate, a deliberate difference from the original, where the volume's listing order decides (docs/rbm.md 11.3).
- CLI: the same default (first by file name bytes); --mod FILE still picks any file. The CLI does not read the UI's preferences.
Implemented in task-151.
<!-- SECTION:FINAL_SUMMARY:END -->

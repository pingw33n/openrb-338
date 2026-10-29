---
id: TASK-145
title: 'Decide: drop orb''s file-name fallback when resolving a song''s mod'
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-29 14:32'
updated_date: '2026-10-29 15:13'
labels:
  - decision
  - cli
dependencies: []
references:
  - docs/rbm.md
  - crates/orb-cli/src/mods.rs
type: spike
ordinal: 162000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-129 (docs/rbm.md 11.5) found that orb's --mod-dir lookup (crates/orb-cli/src/mods.rs find_in_dir) falls back to a file-name match when no mod's INFO name equals the song's GLOB mod name. The original matches only the INFO name, exactly (sub_4155D1, strcmp; docs/rbm.md 10). Two songs of the user's collection (CaveysRebirthStuff/Mod Demo Songs/omega.rbs and ReBirth 2.x Songs/Pack63/FoamBee-NegativeLegSucker.rbs) name 'omega'. No INFO name matches it: Omega.rbm is 'Omega'. The original asks (Download / Standard / Cancel) and, on Standard, plays the built-in kit. orb silently loads Omega.rbm, so orb render --mod-dir differs from the original for these songs. The task-129 test renders such songs without the mod folder, so it does not depend on this decision.

Options:
A. Keep the fallback: it is a documented convenience (docs/rbm.md 10) and finds a mod whose INFO name differs only in case or spelling.
B. Keep it, but warn when the file-name fallback is used: 'the original would not find this mod by name'.
C. Drop it: an INFO-name match only, as the original. --mod FILE stays the explicit override.

Recommendation: B. It keeps the convenience but tells the user that the render is not the original's; the gated test already pins the original's behavior.

Acceptance: the user's choice is recorded here; if B or C, a follow-up task implements it (crates/orb-cli/src/mods.rs, docs/rbm.md 10).
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Decided by @pingw33n on 2026-10-29: a variant of option C. orb finds a song's mod by its INFO name only, ignoring ASCII case, and drops the file-name fallback. This deliberately differs from the original, which compares exactly (sub_4155D1, strcmp): the two 'omega' songs (Mod Demo Songs/omega.rbs, Pack63/FoamBee-NegativeLegSucker.rbs) will load Omega.rbm (INFO 'Omega'), while the original plays the built-in kit. The supervisor's survey of the collection that day: 509 songs match an INFO name exactly, 2 (these two) only ignoring case, and none by file name only. Implemented in task-150.
<!-- SECTION:FINAL_SUMMARY:END -->

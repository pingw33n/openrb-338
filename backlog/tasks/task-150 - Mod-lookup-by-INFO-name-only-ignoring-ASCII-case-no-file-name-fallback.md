---
id: TASK-150
title: 'Mod lookup by INFO name only, ignoring ASCII case; no file-name fallback'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-29 15:13'
labels:
  - cli
  - rbm
milestone: m-4
dependencies:
  - TASK-145
references:
  - crates/orb-cli/src/mods.rs
  - docs/rbm.md
  - crates/orb-cli/tests/thirdparty.rs
priority: medium
type: enhancement
ordinal: 168000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user decided task-145 on 2026-10-29. A song's mod (its GLOB mod name) is found in a mod folder by the mod's INFO name only, comparing ASCII case-insensitively, and the file-name fallback goes. The original compares exactly (sub_4155D1, strcmp; docs/rbm.md 10 and 11.3), so this is a deliberate difference.

What:
- crates/orb-cli/src/mods.rs (find_in_dir and anything else that resolves a song's mod): match the INFO name with ASCII case folding only (eq_ignore_ascii_case; non-ASCII bytes compare exactly). Remove the file-name fallback ('PBE2' or 'pbe2.rbm'). --mod FILE stays the explicit override. Check every user: orb render/compare/play/info and the UI's Mods menu and song open.
- When several mods match (exact duplicates, or names differing only in case), the first in folder order wins, as today (folder_order models NTFS). What orb shows about duplicates is task-146's decision; don't change it here.
- The --mod warning ('the original would not load it for this song') compares the same way: no warning when the names differ only in ASCII case.
- docs/rbm.md 10 and 11: state the rule, the deliberate difference from the original (strcmp), and the collection facts: 509 exact, 2 'omega' songs matching 'Omega' only ignoring case, 0 needing the file name.
- Tests:
  - a mod is found when the song's name differs only in ASCII case;
  - a name that equals only the file name is not found;
  - a non-ASCII letter is compared exactly ('ÇoNaNbox 2.0' does not match 'çoNaNbox 2.0');
  - the first of two matching mods wins.
- The task-129 gated songs test renders the two 'omega' songs as rbhost did (built-in kit). Check how the test passes the mod folder for them, and keep the test true to the original's render; say how in the notes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A song's mod is found by its INFO name only, ASCII case-insensitive, by every orb command and the UI; the file-name fallback is gone; --mod still overrides
- [ ] #2 Tests cover case-only match, no file-name match, exact non-ASCII compare and first-of-two; docs/rbm.md states the rule and the deliberate difference; the task-129 songs test still passes; fmt, clippy, cargo test --release pass
<!-- AC:END -->

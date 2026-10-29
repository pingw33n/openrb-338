---
id: TASK-150
title: 'Mod lookup by INFO name only, ignoring ASCII case; no file-name fallback'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-29 15:13'
updated_date: '2026-10-29 16:09'
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
- [x] #1 A song's mod is found by its INFO name only, ASCII case-insensitive, by every orb command and the UI; the file-name fallback is gone; --mod still overrides
- [x] #2 Tests cover case-only match, no file-name match, exact non-ASCII compare and first-of-two; docs/rbm.md states the rule and the deliberate difference; the task-129 songs test still passes; fmt, clippy, cargo test --release pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. mods.rs: names_match (eq_ignore_ascii_case, non-ASCII exact); find_in_dir = first mod of scan() in folder order whose INFO name matches; drop the file-name fallback; the --mod warning uses names_match. All users (render/compare/info, orb-ui kit_for) go through find_in_dir. 2. Unit tests in mods.rs; update the orb-ui menu.rs test that relied on the fallback. 3. docs/rbm.md 10, 11.3, 11.5; the gated thirdparty.rs omega handling. 4. fmt, clippy, tests, gated thirdparty test, corpus compare 175/175.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a7e404f97958ceec9 (rebased on main 061860e)

Change: crates/orb-cli/src/mods.rs names_match (eq_ignore_ascii_case) + find_in_dir = first mod in folder order whose INFO name matches; file-name fallback removed; --mod warning uses names_match. Every user (render/compare/play/info via resolve/resolve_mod, orb-ui kit_for and Mods menu) goes through find_in_dir. The Standard ReBirth check stays exact. Tests: 4 new unit tests in mods.rs (case-only, no file name, non-ASCII exact, first of two wins), tests/mods.rs and orb-ui menu.rs updated. Found on the way: the unit-test helper wrote the INFO name as UTF-8 but INFO is Latin-1 (cstr_latin1); fixed in the helper. Omega: the gated songs test still renders the two omega songs without --mod-dir (as rbhost: built-in kit, hash equal to rbhost's) and additionally runs orb info --json --mod-dir, requiring Omega.rbm (case-only match): the deliberate difference. orb info on omega.rbs and FoamBee-NegativeLegSucker.rbs with --mod-dir now says: Mod: omega, resolution file ~/rb-mods/mods/Omega.rbm; orb render --mod-dir on omega.rbs loads it. Gated test (cargo test --release -q -p orb-cli --test thirdparty -- --test-threads 1): 2 passed in 149 s, 1239 renders equal rbhost's. Corpus: orb compare --manifest tests/refs/manifest.json --mod-dir (recrypted from the CD): 175 of 175 equal. The 509 exact / 0 file-name counts in docs are the task's, not recounted by me. fmt, clippy, debug orb-ui, release workspace tests pass.

Review round 1 (send back, text only): fixed the stale file-name wording in main.rs render/play --mod-dir help, mods.rs scan comment, orb-ui menu.rs mod_checks and ModEntry.name docs; rewrapped docs/ui_spec.md; thirdparty.rs info_names now uses orb_cli::mods::scan (same folder_order as orb). Grepped crates/docs/AGENTS.md for other file-name fallback mentions: none left (remaining hits are the sample lookup and recrypt wording). Reran fmt --check, clippy, cargo test -q -p orb-cli (incl. gated thirdparty, debug), orb-ui --test menu (50 pass), release thirdparty (2 pass).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
orb finds a song's mod by INFO name only, ignoring ASCII case; the file-name fallback is gone (mods.rs names_match/find_in_dir; the --mod warning compares the same way). docs/rbm.md 10 and 11.5 and docs/ui_spec.md updated. New unit and CLI tests; gated thirdparty test passes (1239 equal rbhost); corpus 175/175; fmt, clippy, cargo test --release pass. The omega songs load Omega.rbm with --mod-dir (deliberate difference); the gated test renders them without the folder to match rbhost and checks the resolution.
<!-- SECTION:FINAL_SUMMARY:END -->

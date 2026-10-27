---
id: TASK-063
title: 'Recrypt all mods; rb reads only recrypted mods, the DAT goes to recrypt only'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 09:40'
updated_date: '2026-10-27 10:26'
labels:
  - rbm
  - cli
  - testmachine
milestone: m-2
dependencies:
  - TASK-059
references:
  - docs/rbm.md
  - docs/test_machine.md
priority: medium
type: enhancement
ordinal: 33750
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested by the user on 2026-10-27, to run right after task-059 (rb recrypt with the embedded 2 KB key window). Once mods can be recrypted, rb stops reading original (non-recrypted) mods itself: the only rb command that takes RB20FUL.DAT is `rb recrypt`. Everything else uses recrypted mods and the embedded window only.

1. CLI: remove `--rb20ful` and the `RB_RB20FUL` lookup (and the default-location search, crates/rb-cli/src/mods.rs) from `rb render`, `rb compare` and `rb play`; keep them only on `rb recrypt`. A mod that is not recrypted fails with a clear error naming the file and the fix (`rb recrypt --rb20ful DAT IN.rbm -o OUT.rbm`). The mod folder lookup (--mod-dir, RB_MOD_DIR, the duplicate-name order of task-058) skips or reports non-recrypted mods consistently (decide which, and say so in the help: listing them as "needs rb recrypt" is suggested). crates/rbm keeps the general descrambler for recrypt's own use only. The future UI (m-3, task-047's mod chooser) gets the same rule. Update the CLI help, AGENTS.md and the docs that name --rb20ful (docs/rbm.md, the task-053 note).
2. Recrypt the user's third-party mods in ~/rb_stuff/Mods (172 files). Do not overwrite the originals: first move them to ~/rb_stuff/Mods.orig (check it does not exist; never delete it), then write the recrypted mods to ~/rb_stuff/Mods under the same file names. Verify each: every embedded file and sample equals the original's (tools/rbm.py or the rbm crate), and record the counts.
3. Recrypt the shipped mods on the test machine. The user asked for this explicitly, which overrides for this step only the standing rule against writing under C:\ReBirth RB-338: back up C:\ReBirth RB-338\Mods to C:\re\rb\mods_orig first (check it does not exist; never delete it), recrypt the four shipped mods locally from that copy, and replace the files in C:\ReBirth RB-338\Mods with the recrypted ones; check by hash. Only when Rebirth.exe is not running (the user sometimes uses it; wait, never kill it), holding all redc-win-run permits. Then one app export through tools/rb_export.py (--restore-prf) of a shipped mod song from the installation, showing it equals `rb compare` with the recrypted mod. Record in docs/test_machine.md that the installation's Mods are recrypted and where the originals are. Also recrypt the local copies the tests use (the shipped mods in RB_MOD_DIR, e.g. the scratchpad data/Mods folder the supervisor names) into a new folder, keeping the originals.
4. Tests: the env-gated mod tests (crates/rb-cli/tests/corpus_mods.rs, thirdparty.rs, mods.rs) take recrypted mods from RB_MOD_DIR / RB_THIRDPARTY_DIR and no RB20FUL.DAT; one gated test still runs `rb recrypt` from RB_RB20FUL and checks the result equals the committed expectations. `rb compare` over the manifest stays 164/164 and 175/175 with the mod env vars (pointing at recrypted mods); cargo test --release green.
Never commit a .rbm or key bytes beyond the embedded window.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rb render, compare and play no longer take --rb20ful or RB_RB20FUL; only rb recrypt does; a non-recrypted mod fails with an error naming rb recrypt
- [x] #2 The 172 mods in ~/rb_stuff/Mods are recrypted, the originals kept in ~/rb_stuff/Mods.orig, and every file and sample verified equal
- [x] #3 The shipped mods in C:\ReBirth RB-338\Mods are recrypted (originals in C:\re\rb\mods_orig), and an app export with one equals rb compare
- [x] #4 The mod tests run on recrypted mods without RB20FUL.DAT; compare stays 164/164 and 175/175; cargo test --release passes; docs and AGENTS.md updated
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. CLI: drop --rb20ful/RB_RB20FUL from render/compare/play; load_mod accepts only embedded-window mods, else an error naming rb recrypt; find_rb20ful moves to recrypt (searches the input's folder). Non-recrypted mods stay listed (found by name) and fail at load. 2. Recrypt ~/rb_stuff/Mods after moving it to Mods.orig; verify with thirdparty.rs mods against rbm.py check-dir on the originals. 3. Test machine: back up Mods to C:\re\rb\mods_orig, recrypt from the downloaded backup, replace under all permits with no Rebirth.exe, hash check, rb_export --restore-prf of Fonda Workout, scan N for rb compare. 4. Tests and docs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-aa904bcd5066f8dec

Evidence. CLI: crates/rb-cli/src/main.rs (load_mod, recrypt), mods.rs (find_rb20ful replaces ModOpts::rb20ful). Decision: non-recrypted mods are found by name like any other and fail when loaded (an error naming the file and 'rb recrypt --rb20ful RB20FUL.DAT IN -o OUT.rbm'); skipping would hide the fix as 'mod not found'. Stated in the --mod-dir help, mods.rs and docs/rbm.md 12.4.
User mods: ~/rb_stuff/Mods moved to ~/rb_stuff/Mods.orig (did not exist before), 172 recrypted into ~/rb_stuff/Mods, 0 failures. Verification: thirdparty.rs mods test (recrypted Mods through the embedded window vs tools/rbm.py check-dir of Mods.orig with the DAT): 172 checked, 172 load, 16680 embedded files and 7285 slots equal. Shipped mods for tests: recrypted copies of the scratchpad data/*.rbm in scratchpad/recrypted-mods (originals untouched). Sizes +1032 each.
Test machine: no Rebirth.exe running, all 6 permits held (released after). C:\re\rb\mods_orig did not exist; robocopy backup, hashes equal the docs/test_machine.md table; recrypted locally from the downloaded backup (hashes equal to the scratchpad recrypted-mods), rcs put into the installation, hash check of the 4 files equal (see docs/test_machine.md section 7). rb_export.py --restore-prf of 'Fonda Workout.rbs': no extra boxes, 75600 frames, prefs restored (6309526e). rb compare of the song with that wav, N scanned 3400..3707: EQUAL only at N=3600. Scripts in the scratchpad t063/ (recrypt_all.sh, machine_a.sh, machine_b.sh, scan.sh).
Tests: cargo test --release with RB_MOD_DIR=recrypted shipped, RB_MOD_ORIG_DIR + RB_RB20FUL, RB_THIRDPARTY_DIR: all pass; rb compare --manifest with recrypted RB_MOD_DIR: 175 of 175 equal, --set diag,app,shipped 164 of 164; with the original (non-recrypted) mods the mod set fails with the rb recrypt error. clippy -D warnings and fmt clean. The rbm crate's own tests (RB_RBM_DIR, thirdparty_mods_recrypt) still take ORIGINAL mods (thirdparty now reads Mods.orig): they test recrypt. For the supervisor: new RB_MOD_DIR for the tests is the recrypted-mods folder; RB_RBM_DIR for rbm tests stays the original data folder.
Not done/unconfirmed: the UI (task-047) rule is only written in docs/rbm.md 12.4; tools/rbcorpus.py and rbhost still use --rb20ful with ORIGINAL mods (capture tools, unchanged).

Review round 1: MANIFEST.md is generated; the hand edit was replaced by a change of the generator string (tools/rbcorpus.py) and a regeneration with 'tools/rbcorpus.py manifest' (RB_MOD_DIR/RB_RB20FUL = the original mods; manifest.json unchanged; MANIFEST.md differs from main only in the mod line). Prefs restore after the installation export (6309526e) recorded in docs/test_machine.md section 7. Commit Testing line has the cargo test count (259 pass).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb render, compare and play no longer take --rb20ful/RB_RB20FUL; only rb recrypt does, and a non-recrypted mod fails at load naming the file and 'rb recrypt'. The user's 172 mods are recrypted in ~/rb_stuff/Mods (originals in ~/rb_stuff/Mods.orig; 16680 files and 7285 slots equal rbm.py's reference). The four installation mods on the test machine are recrypted (originals in C:\re\rb\mods_orig, hashes verified); an app export of Fonda Workout equals rb compare at N=3600. Mod tests now run on recrypted mods without the DAT (a gated test still runs rbm::recrypt). rb compare 164/164 and 175/175; cargo test --release, clippy, fmt green. Docs: rbm.md 12.4, test_machine.md 7, AGENTS.md, MANIFEST.md. Branch worktree-agent-aa904bcd5066f8dec, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

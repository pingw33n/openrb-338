---
id: TASK-121
title: >-
  Tests: only RB_CD_ISO, RB_THIRDPARTY_MODS_DIR and RB_THIRDPARTY_SONGS_DIR as
  external inputs
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 13:51'
updated_date: '2026-10-28 16:56'
labels:
  - tests
  - rbm
  - tools
dependencies: []
references:
  - crates/rb-cli/tests
  - crates/rbm/tests
  - crates/rb-cli/src/cd.rs
  - docs/test_machine.md
priority: medium
type: task
ordinal: 141000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) to reduce the tests' external inputs to three:
- RB_CD_ISO, the ReBirth 2.01 CD image;
- RB_THIRDPARTY_MODS_DIR, the user's folder of RECRYPTED third-party mods (no Mods.orig);
- RB_THIRDPARTY_SONGS_DIR, the user's folder of third-party songs.
RB_THIRDPARTY_DIR is gone (the user split it on 2026-10-28). It is not in a milestone, at the user's request.

Today the gated tests also read:
- RB_RB20FUL (the CD's RB20FUL.DAT);
- RB_MOD_ORIG_DIR (the 4 shipped mods, not recrypted);
- RB_MOD_DIR (the 4 shipped mods, recrypted);
- RB_CD_REBIRTH_EXE (the CD's Rebirth.exe);
- the originals of the user's 172 mods (Mods.orig);
- original/ReBirth.dll and original/Rebirth.exe.
Since task-065 rb reads all of these from the CD image (crates/rb-cli/src/cd.rs): RB20FUL.DAT, the installer's ReBirth.dll, Rebirth.exe and the four shipped mods.

What:
1. A shared test helper (a small test-support crate) that, when RB_CD_ISO is set, extracts once per test process (OnceLock) into a temp folder:
   - the DAT, ReBirth.dll, Rebirth.exe and the 4 original shipped mods, checked against their known SHA-256s;
   - the 4 mods recrypted from them.
   Replace every use of RB_RB20FUL, RB_MOD_ORIG_DIR, RB_MOD_DIR and RB_CD_REBIRTH_EXE in tests with it. Tests that need these skip with one clear message when RB_CD_ISO is unset.
2. Where a test compared against RB_MOD_DIR's files (for example recrypt_of_the_originals_matches_the_references and the cd_image comparisons), compare against pinned SHA-256s of the 4 recrypted mods instead (docs/test_machine.md section 7).
3. Tests that read original/ (rbm standard tests, the engine's sample and table tests, cd_image's dll comparison) take the DLL and exe from the helper. Six engine tests failed without original/; they must skip or use the helper. This prepares task-066, which removes original/ from history.
4. The third-party mods test without Mods.orig (crates/rb-cli/tests/thirdparty.rs, the mods test):
   - Before removing the old form, run it once with Mods.orig and the DAT (read only).
   - That run generates a committed manifest, tests/refs/thirdparty_mods.json: per mod file name, its SHA-256 and the SHA-256 of each embedded file and of each slot's channel-0 PCM. Only hashes and names.
   - The test then checks RB_THIRDPARTY_MODS_DIR against the manifest.
   - Document how to regenerate the manifest (a tool with the originals and the DAT as arguments, not test variables).
5. More recryptor coverage that needs no user file: synthetic mods built in the test with a synthetic key table, covering ordinals >= 256 (docs/rbm.md 12.1), payloads of 1024 bytes and less, 1025 and large ones, 199 files, duplicate names, foreign chunks, zero-size filler chunks and an earlier recrypt's pad (12.2), slot samples and other files. Recrypt each and decode the result with tools/rbm.py, the independent decoder, as well as with crates/rbm, and require equal files. These run without any env var, so in CI too.
6. RB_THIRDPARTY_SONGS_DIR's songs test stays as it was (it also uses RB_THIRDPARTY_MODS_DIR for mod resolution when set).
7. Update AGENTS.md (Build and test), the test module docs and docs/test_machine.md to name only RB_CD_ISO, RB_THIRDPARTY_MODS_DIR and RB_THIRDPARTY_SONGS_DIR. Drop the deprecated RB_RBM_DIR alias from the tests (and say whether the CLI keeps it).
- Check: run the whole suite with only the three variables set, and with none. Record the counts and the skipped tests in both runs. With the three set, the corpus mod songs (175 of 175) must still be compared.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 With only RB_CD_ISO, RB_THIRDPARTY_MODS_DIR and RB_THIRDPARTY_SONGS_DIR set, every gated test that ran before runs (corpus 175/175, recrypt, cd_image, standard mod reproduction, thirdparty songs and mods); RB_RB20FUL, RB_MOD_ORIG_DIR, RB_MOD_DIR, RB_CD_REBIRTH_EXE and RB_RBM_DIR are read by no test
- [x] #2 The thirdparty mods test checks RB_THIRDPARTY_MODS_DIR against a committed hash manifest (no Mods.orig, no DAT); regenerating it is documented
- [x] #3 Synthetic recrypt tests (ordinals >= 256, size edges, 199 files, duplicate names, foreign/filler chunks, old pad) pass against both crates/rbm and tools/rbm.py with no env var
- [x] #4 The tests run without original/ (DLL and exe from the ISO, or skipped); AGENTS.md and docs name only the three variables; cargo test and clippy pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. New dev-only crate crates/rb-testkit: when RB_CD_ISO is set, extracts once per process (OnceLock, cached in a temp folder keyed by the image) the DAT, ReBirth.dll, Rebirth.exe and the 4 shipped mods, each checked against pinned SHA-256, and recrypts the 4 mods (checked against pinned recrypted hashes). Also env helpers for RB_THIRDPARTY_MODS_DIR / RB_THIRDPARTY_SONGS_DIR, with one skip message each.
2. Replace RB_RB20FUL / RB_MOD_ORIG_DIR / RB_MOD_DIR / RB_CD_REBIRTH_EXE / RB_RBM_DIR reads in rb-cli, rbm, rb-engine, rb-player, rb-ui tests (and tools/rbm_test.py, driven by a Rust test) with the helper; original/ reads use the helper's DLL/exe.
3. Thirdparty mods test: tools/thirdparty_mods_manifest.py (args: --orig, --mods, --dat, -o) generates tests/refs/thirdparty_mods.json from Mods.orig + DAT via tools/rbm.py once now; the test checks RB_THIRDPARTY_MODS_DIR against it. Songs test uses RB_THIRDPARTY_SONGS_DIR (+ MODS_DIR when set).
4. Synthetic recrypt tests (no env): ordinals >= 256, size edges, 199 files, duplicate names, foreign/filler chunks, old pad; compared with tools/rbm.py extract and crates/rbm.
5. Docs: AGENTS.md, docs/test_machine.md, docs/rbm.md, docs/building.md, assets/README.md, test module docs. 6. Three final runs + fmt + clippy.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User change (2026-10-28): RB_THIRDPARTY_DIR is split into two variables, RB_THIRDPARTY_MODS_DIR (a folder of RECRYPTED .rbm, today ~/rb_stuff/Mods) and RB_THIRDPARTY_SONGS_DIR (a folder of songs, today ~/rb_stuff/songs). The user typed 'RB_THRIDPARTY' and 'RB_THIRDPART_SONGS_DIR'; the intended spelling is RB_THIRDPARTY_*. RB_THIRDPARTY_DIR goes away; don't keep it as an alias.
- The songs test needs only RB_THIRDPARTY_SONGS_DIR. It also uses RB_THIRDPARTY_MODS_DIR for mod resolution when that is set, and says so when it isn't.
- The mods manifest check needs RB_THIRDPARTY_MODS_DIR.
- Regenerating the manifest uses the originals (Mods.orig) and the DAT, passed as explicit arguments to the regeneration command, not as test variables.
- The external test inputs are then RB_CD_ISO, RB_THIRDPARTY_MODS_DIR and RB_THIRDPARTY_SONGS_DIR.
- The task's acceptance criteria and docs read with this split.

Branch: worktree-agent-a48adec5d3a667080 (rebased on main 4f7bd2b).

Design: new dev-only crate crates/rb-testkit (rb_testkit): cd_files() extracts once per process (OnceLock) from RB_CD_ISO into a cache folder temp_dir()/rb-testkit-<hash of iso path,size,mtime> (files written by temp name + rename; reused by later test processes after re-checking the SHA-256): RB20FUL.DAT (embedded::DAT_SHA256), ReBirth.dll, Rebirth.exe (cd::DLL_SHA256 / EXE_SHA256), the 4 original mods (pinned, docs/test_machine.md 7 first table) and the 4 mods recrypted by rbm::recrypt (pinned, second table; they equal the shipped recrypted mods in the supervisor's scratchpad, hashes compared: 12fbe698.., 28cb40e0.., eb1843b6.., 62789353..). thirdparty_mods()/thirdparty_songs() give the other two variables with one skip message. dev-dependency cycles (rb-testkit -> rb-cli, rbm) as already used for rbm in rb-engine.
Baseline (old thirdparty mods test, before removing it, RB_THIRDPARTY_DIR=~/rb_stuff + RB_RB20FUL=the DAT copy): 'mods: 172 checked, 172 load, 0 fail; 54 keep built-in slots; 16680 embedded files and 7285 slots equal rbm.py's' (59 s release).
Manifest: tools/thirdparty_mods_manifest.py --orig ~/rb_stuff/Mods.orig --mods ~/rb_stuff/Mods --dat <DAT> -o tests/refs/thirdparty_mods.json (90 s) -> 172 mods, 16680 files, 7285 slots, 0 errors, 1,992,742 bytes, hashes and names only. New test on ~/rb_stuff/Mods: 'mods: 172 checked, 172 load, 0 fail; 54 keep built-in slots; 16680 embedded files and 7285 slots equal the manifest's; 0 .rbm in the folder are not in the manifest' (same counts as the baseline). songs test with RB_THIRDPARTY_SONGS_DIR + MODS_DIR: 208 files, 2 duplicates, 206 distinct: RBS1X 129, RBS42 77, no failure.
Synthetic recrypt tests: crates/rbm/tests/recrypt_python.rs, 5 tests (ordinals past 255 with slot samples whose DLL key differs from the u16 key; payload sizes 8..3,000,000 around 1024; 199 files; duplicate names incl. slot names; foreign chunks, fillers and an old pad). Each recrypts, then tools/rbm.py (run through tests/common/python.rs, given a sparse DAT holding only the embedded window) and crates/rbm must decode the same files as the mod was built from; the source is also decoded by rbm.py with a stand-in DAT. No env var.
RB_RBM_DIR: dropped from the tests (rbm.rs, tools/rbm_test.py now use RBM_TEST_DAT / RBM_TEST_MODS, set only by the cargo test the_python_tool_tests_pass_on_the_real_files for its child). The CLI KEEPS the deprecated RB_RBM_DIR alias (crates/rb-cli/src/mods.rs, unchanged; removing it is not part of this task). Tests that set/remove RB_MOD_DIR/RB_RB20FUL/RB_RBM_DIR only do so on the child process of the CLI under test (isolation or to test the CLI's own variable); none reads them from its own environment.
Removed: thirdparty_mods_recrypt (rbm.rs; it recrypted the 172 originals of Mods.orig). Its coverage is now the manifest (the SHA-256 of each recrypted file pins rb mod recrypt's output; the embedded-file hashes come from the independent decoder over the originals) plus the synthetic tests. Not confirmed: whether the same recrypt-vs-originals check should stay as an opt-in with arguments; skipped.

Final runs (release, --no-fail-fast, -q):
1. RB_CD_ISO + RB_THIRDPARTY_MODS_DIR=~/rb_stuff/Mods + RB_THIRDPARTY_SONGS_DIR=~/rb_stuff/songs only (the other variables unset with env -u): cargo test --release -q --no-fail-fast: 558 passed, 0 failed, 3 ignored, exit 0. Skipped: only 'the file system folds case' (macOS, unrelated). Gated tests that ran: corpus_mods (mod 8, modapp 3, recrypt of the originals), cd_image real image, recrypt shipped, standard_mod reproduction, rb-engine samples (4) and tables (2) tests with the DLL from the ISO, rbm real_mods tests + the python tool tests, rb-player live_messages mods, rb-ui menu mod tests and the 1.x collection test, thirdparty mods (172/172 against the manifest) and songs (206 distinct).
2. None set: 558 passed, 0 failed, 3 ignored, exit 0; skip lines: 47x RB_CD_ISO is not set (incl. the recrypted-mod tests of live_messages), 1x RB_THIRDPARTY_MODS_DIR, 2x RB_THIRDPARTY_SONGS_DIR, 1x folds case. No test fails without original/ any more (the six engine tests skip).
3. Corpus compare: 175 of 175 equal. The 11 mod songs get their mods from the CD image: rb mod recrypt --iso RB_CD_ISO -o DIR (4 recrypted, hashes as pinned), then rb compare --manifest tests/refs/manifest.json --mod-dir DIR (documented in AGENTS.md).
Also: debug cargo test -p rbm -p rb-cli -p rb-engine -p rb-player -p rb-ui -p rb-testkit with RB_CD_ISO only: 534 passed, 0 failed, 7 ignored; cargo +nightly fmt --all --check clean; cargo clippy --all-targets -- -D warnings clean; python tools tests (66, 3 skipped) and rbhost (43) pass.
Supervisor note: CI needs nothing. Local runs set RB_CD_ISO, RB_THIRDPARTY_MODS_DIR, RB_THIRDPARTY_SONGS_DIR (no longer RB_RB20FUL, RB_MOD_DIR, RB_MOD_ORIG_DIR, RB_CD_REBIRTH_EXE, RB_RBM_DIR, RB_THIRDPARTY_DIR). rb-testkit caches the extracted files in the system temp folder (rb-testkit-<hash>); safe to delete.

Review round 1 (all 8 points done, one amended commit, rebased on main 0aa7858):
1. Subject shortened to 72 characters or less.
2. CORRECTION: the earlier note that the manifest 'pins rb mod recrypt's output' was false (the generator only hashed the user's Mods/). Now tools/thirdparty_mods_manifest.py first runs the current rb (rb mod recrypt, bulk, into a temp dir) over --orig and fails unless every result equals the matching --mods file byte for byte; only then does it write the manifest. Run on the real collection: all 172 equal, manifest byte-identical to the previous one. New opt-in mode --check --orig DIR --dat DAT [--manifest F] [--rb PATH] recrypts the originals with the current rb and compares with the committed manifest: 'equal' on the real collection (exit 0). Documented in AGENTS.md and docs/rbm.md 11.
3. tests/refs/MANIFEST.md now says the cargo tests get the mods from RB_CD_ISO through rb-testkit.
4. AGENTS.md states RB_LIVE_OUT and RBM_TEST_DAT/RBM_TEST_MODS are not test inputs.
5. Description rewritten with the split (-d).
6. cd_image.rs no-op assert removed.
7. rb-testkit: a failed rename is retried 5 times; success if the target then has the right hash.
8. thirdparty.rs: a .rbm in the folder that the manifest does not list now fails the test with the names; documented (AGENTS.md, docs/rbm.md, tool doc).
Reruns after the amend: release --no-fail-fast with the three variables 558 passed 0 failed 3 ignored (1 skip: folds case); with none 558 passed 0 failed 3 ignored; corpus compare 175 of 175 equal; nightly fmt and clippy clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The tests read three external variables only: RB_CD_ISO, RB_THIRDPARTY_MODS_DIR, RB_THIRDPARTY_SONGS_DIR. Branch worktree-agent-a48adec5d3a667080, one commit on main 0aa7858.
- New dev-only crate crates/rb-testkit: from RB_CD_ISO extracts once per process the DAT, ReBirth.dll, Rebirth.exe and the 4 shipped mods (pinned SHA-256) and the 4 mods recrypted (pinned); the tests of rb-cli, rbm, rb-engine (incl. its 6 tables/samples tests), rb-player and rb-ui use it instead of RB_RB20FUL, RB_MOD_ORIG_DIR, RB_MOD_DIR, RB_CD_REBIRTH_EXE, RB_RBM_DIR and original/, and skip with one message without the image.
- thirdparty mods test checks RB_THIRDPARTY_MODS_DIR against tests/refs/thirdparty_mods.json (172 mods, 16680 files, 7285 slots; hashes and names only) and fails on a .rbm the manifest does not list. tools/thirdparty_mods_manifest.py (originals, DAT as arguments) recrypts the originals with the current rb and requires them to equal Mods/ byte for byte before writing, so the manifest pins rb's recrypt output; --check re-verifies the recryptor on the real collection. The old thirdparty_mods_recrypt test is replaced by this and the synthetic tests. songs test needs RB_THIRDPARTY_SONGS_DIR (MODS_DIR for mod resolution when set).
- crates/rbm/tests/recrypt_python.rs: 5 synthetic recrypt tests checked by crates/rbm and tools/rbm.py, no env var.
- tools/rbm_test.py real-file classes take RBM_TEST_DAT/RBM_TEST_MODS, set only by a cargo test for its child. The CLI keeps RB_MOD_DIR and the deprecated RB_RBM_DIR alias (unchanged).
- Docs: AGENTS.md, docs/rbm.md, test_machine.md, building.md, assets/README.md, tests/refs/MANIFEST.md, ci.yml comment.
Numbers: release run with the 3 variables 558 passed 0 failed 3 ignored; with none 558 passed; corpus compare 175 of 175 (mods via rb mod recrypt --iso); debug 534 passed; fmt and clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->

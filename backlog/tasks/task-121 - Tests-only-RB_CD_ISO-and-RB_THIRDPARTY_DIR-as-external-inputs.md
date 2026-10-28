---
id: TASK-121
title: >-
  Tests: only RB_CD_ISO, RB_THIRDPARTY_MODS_DIR and RB_THIRDPARTY_SONGS_DIR as
  external inputs
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 13:51'
updated_date: '2026-10-28 16:10'
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
Why: the user asked (2026-10-28) to reduce the tests' external inputs to two:
- RB_CD_ISO, the ReBirth 2.01 CD image;
- RB_THIRDPARTY_DIR, with its songs/ and its RECRYPTED Mods/ only, no Mods.orig/.
It is not in a milestone, at the user's request.

Today the gated tests also read:
- RB_RB20FUL (the CD's RB20FUL.DAT);
- RB_MOD_ORIG_DIR (the 4 shipped mods, not recrypted);
- RB_MOD_DIR (the 4 shipped mods, recrypted);
- RB_CD_REBIRTH_EXE (the CD's Rebirth.exe);
- RB_THIRDPARTY_DIR/Mods.orig, the originals of the user's 172 mods;
- original/ReBirth.dll and original/Rebirth.exe.
Since task-065 rb reads all of these from the CD image (crates/rb-cli/src/cd.rs): RB20FUL.DAT, the installer's ReBirth.dll, Rebirth.exe and the four shipped mods.

What:
1. A shared test helper (in a tests/common module, or a small test-support crate) that, when RB_CD_ISO is set, extracts once per test process (OnceLock) into a temp folder:
   - the DAT, ReBirth.dll, Rebirth.exe and the 4 original shipped mods, checked against their known SHA-256s;
   - the 4 mods recrypted from them.
   Replace every use of RB_RB20FUL, RB_MOD_ORIG_DIR, RB_MOD_DIR and RB_CD_REBIRTH_EXE in tests with it. Tests that need these skip with one clear message when RB_CD_ISO is unset.
2. Where a test compared against RB_MOD_DIR's files (for example recrypt_of_the_originals_matches_the_references and the cd_image comparisons), compare against pinned SHA-256s of the 4 recrypted mods instead. They are in docs/test_machine.md section 7; check them. Hashes hold no Propellerhead material and may be committed.
3. Tests that read original/ (rbm standard tests, the engine's sample and table tests, cd_image's dll comparison) take the DLL and exe from the helper when original/ is absent. Six engine tests fail today without original/ (crates/rb-engine/tests/samples.rs, tables::tests); they must skip or use the helper. This prepares task-066, which removes original/ from history.
4. The third-party mods test without Mods.orig (crates/rb-cli/tests/thirdparty.rs, the mods test, which today compares every embedded file and slot sample of the 172 recrypted mods with tools/rbm.py over the originals and the DAT):
   - Before removing it, run it once with Mods.orig and the DAT (paths: ~/rb_stuff/Mods.orig, read only; the DAT from the ISO).
   - That run generates a committed manifest, e.g. tests/refs/thirdparty_mods.json: per mod file name, its SHA-256 and the SHA-256 of each embedded file and of each slot's channel-0 PCM. Only hashes and names; no file content.
   - The test then checks Mods/ against the manifest.
   - Document how to regenerate the manifest (a test flag or tools script), which still uses Mods.orig and the DAT.
5. More recryptor coverage that needs no user file: synthetic mods built in the test with a synthetic key table (the Options::expected_sha256 seam of task-064, or a test DAT), covering:
   - ordinals >= 256, where the DLL byte key and the exe u16 key differ (docs/rbm.md 12.1);
   - payloads of 1024 bytes and less, 1025, and large ones;
   - 199 files;
   - duplicate names, foreign chunks, zero-size filler chunks and an earlier recrypt's pad (12.2);
   - slot samples and other files.
   Recrypt each and decode the result with tools/rbm.py, the independent decoder, as well as with crates/rbm, and require equal files. These run without any env var, so in CI too.
6. RB_THIRDPARTY_DIR's songs test stays as is (Mods/ and songs/).
7. Update AGENTS.md (Build and test), the test module docs and docs/test_machine.md to name only RB_CD_ISO and RB_THIRDPARTY_DIR. Drop the deprecated RB_RBM_DIR alias from the tests (and say whether the CLI keeps it).
- Check: run the whole suite with only RB_CD_ISO and RB_THIRDPARTY_DIR set, and with neither. Record the counts and the skipped tests in both runs. With both set, the corpus mod songs (175 of 175) must still be compared.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With only RB_CD_ISO and RB_THIRDPARTY_DIR set, every gated test that ran before runs (corpus 175/175, recrypt, cd_image, standard mod reproduction, thirdparty songs and mods); RB_RB20FUL, RB_MOD_ORIG_DIR, RB_MOD_DIR and RB_CD_REBIRTH_EXE are read by no test
- [ ] #2 The thirdparty mods test checks Mods/ against a committed hash manifest (no Mods.orig); regenerating it is documented
- [ ] #3 Synthetic recrypt tests (ordinals >= 256, size edges, 199 files, duplicate names, foreign/filler chunks, old pad) pass against both crates/rbm and tools/rbm.py with no env var
- [ ] #4 The tests run without original/ (DLL and exe from the ISO, or skipped); AGENTS.md and docs name only the two variables; cargo test and clippy pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User change (2026-10-28): RB_THIRDPARTY_DIR is split into two variables, RB_THIRDPARTY_MODS_DIR (a folder of RECRYPTED .rbm, today ~/rb_stuff/Mods) and RB_THIRDPARTY_SONGS_DIR (a folder of songs, today ~/rb_stuff/songs). The user typed 'RB_THRIDPARTY' and 'RB_THIRDPART_SONGS_DIR'; the intended spelling is RB_THIRDPARTY_*. RB_THIRDPARTY_DIR goes away; don't keep it as an alias.
- The songs test needs only RB_THIRDPARTY_SONGS_DIR. It also uses RB_THIRDPARTY_MODS_DIR for mod resolution when that is set, and says so when it isn't.
- The mods manifest check needs RB_THIRDPARTY_MODS_DIR.
- Regenerating the manifest uses the originals (Mods.orig) and the DAT, passed as explicit arguments to the regeneration command, not as test variables.
- The external test inputs are then RB_CD_ISO, RB_THIRDPARTY_MODS_DIR and RB_THIRDPARTY_SONGS_DIR.
- The task's acceptance criteria and docs read with this split.
<!-- SECTION:NOTES:END -->

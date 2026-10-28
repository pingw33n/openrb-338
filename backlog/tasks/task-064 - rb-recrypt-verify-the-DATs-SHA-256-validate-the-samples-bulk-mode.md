---
id: TASK-064
title: 'rb recrypt: verify the DAT''s SHA-256, validate the samples, bulk mode'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 09:59'
updated_date: '2026-10-28 11:12'
labels:
  - rbm
  - cli
milestone: m-2
dependencies:
  - TASK-059
references:
  - docs/rbm.md
priority: medium
type: enhancement
ordinal: 90000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested by the user on 2026-10-27. Not in a milestone.

Today `rb recrypt` (task-059, crates/rbm/src/recrypt.rs, crates/rb-cli/src/main.rs) checks only RB20FUL.DAT's header magic (crates/rbm/src/lib.rs, DAT_MAGIC) and its length. A different or corrupt DAT with the right header gives a silently wrong result: the samples decrypt to garbage, which recrypt then re-scrambles against the embedded window, and its internal round-trip check passes because it reuses the same wrong key. The mod loads and plays noise.

1. Verify the DAT: `rb recrypt` computes the SHA-256 of the whole file and compares it with embedded::DAT_SHA256 (crates/rbm/src/embedded.rs; 62071e99...2797, the 2001 reissue CD); a mismatch fails before anything is written, with an error that names the expected and actual hash. Also check that the DAT's bytes at the embedded window equal the embedded bytes (redundant with the hash, but a clear message for a different edition). Only one CD edition's DAT is known; say in the error and in docs/rbm.md section 12 that another edition is not supported yet.
2. Validate the result: every decrypted sample must parse as AIFF (the engine's reader, crates/rb-engine/src/samples/aiff.rs) and every JPEG must start with FF D8; a failure is an error naming the file and the embedded name, before the output is written.
3. Bulk recrypt: `rb recrypt` takes several inputs, or a directory, so the DAT is read and hashed once per run: e.g. `rb recrypt --rb20ful DAT IN.rbm... -o OUTDIR` and `rb recrypt --rb20ful DAT INDIR -o OUTDIR` (keep the single-file `IN.rbm -o OUT.rbm` form). Output names equal input names; the output directory must not be an input directory and existing outputs are not overwritten without `--force`. One mod failing does not stop the others; the exit code is non-zero if any failed, with a summary (recrypted, already recrypted, failed with reasons). An already recrypted input is copied unchanged (or skipped; say which). Keep the DAT reads per mod to the few pages each needs.
4. Tests: a wrong-hash DAT (a temporary file with the right magic and length) is rejected; a corrupted sample is caught; bulk over the shipped mods (env-gated, RB_RB20FUL and RB_MOD_DIR) gives the same bytes as recrypting each alone, and hashes the DAT once (count it, e.g. through a test hook or a log line).
Coordinate with task-063 (which drops --rb20ful from render, compare and play and recrypts the user's mods): if task-063 is not merged yet, its bulk recrypt can use this; if it is, nothing in it changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rb recrypt rejects an RB20FUL.DAT whose SHA-256 differs from the embedded DAT_SHA256, before writing anything, with a clear error
- [x] #2 Every decrypted sample is checked to parse as AIFF and every JPEG to start with FF D8; a failure is an error naming the file
- [x] #3 rb recrypt takes several mods or a directory and an output directory, hashing the DAT once per run; one failure does not stop the rest; a summary and a non-zero exit on any failure
- [x] #4 Tests cover the wrong-hash DAT, a corrupted sample, and bulk equal to single (env-gated); docs/rbm.md section 12 and the CLI help describe it
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rbm: new module dat (verify_dat: magic, length, streaming SHA-256 vs embedded::DAT_SHA256, on mismatch an error with expected/actual hash and whether the window bytes differ; says other editions are unsupported); rbm::validate_files (AIFF via engine reader, JPEG FF D8, on the decrypted files, before recrypt). sha2 becomes a normal dep of rbm.
2. rb-cli lib: module recrypt with run(Options)->Report: inputs = files or one dir, -o file (single form) or dir (bulk), --force, DAT located and hashed lazily once (counted in Report.dat_hashes), per-mod KeyTable::read of 2 pages, per-mod failures collected, already-recrypted inputs validated and copied unchanged. main: Recrypt takes INPUT... , --force, prints summary, exit 1 on any failure.
3. Tests: synthetic DAT file (virtual bytes, expected hash injectable) for bulk==single, hash once, corrupted sample, wrong hash (real expected hash, right magic+length); env-gated shipped-mod bulk (RB_RB20FUL+RB_MOD_ORIG_DIR); already-recrypted bulk over RB_MOD_DIR (no DAT needed).
4. docs/rbm.md section 12.5, CLI help.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a2223ff21dcf1f391

Evidence: RB20FUL.DAT was not found on this machine (RB_RB20FUL unset; searched ~/rb_stuff, original/, ~/Downloads, home to depth 5), so the gated real-DAT test (shipped_mods_bulk_equals_single_with_the_real_key_file, RB_RB20FUL + RB_MOD_ORIG_DIR) and the real-hash acceptance were NOT run here; they skip. Ran: rb mod recrypt over ~/rb_stuff/Mods (172 recrypted) and the 4 shipped recrypted mods: 0 failed, all checked with validate_files (no false positives for AIFF/JPEG checks) and copied unchanged; over ~/rb_stuff/Mods.orig without a DAT: 172 failed 'no RB20FUL.DAT found', exit 1, summary printed. Finding: the AIFF/JPEG header checks do not detect a wrong key (the first 1024 bytes are never scrambled), so the SHA-256 is the key file check; documented in docs/rbm.md 12.5. Decisions made within the task text: bulk already-recrypted = checked and copied unchanged; single form = one input file and -o ending .rbm that is not an existing dir; existing output needs --force also in the single form (was overwritten silently before). tests/mods.rs recrypts its stand-in mod in process now (the CLI rejects a stand-in key file). Added [profile.dev.package.sha2] opt-level=3 so debug tests hash 128 MB quickly. Options::expected_sha256 is the only way to accept another DAT (library/test use, not the CLI).

Gated run with the real DAT (SHA-256 matches, accepted by verify_dat), RB_MOD_ORIG_DIR = scratchpad t063/mods_orig_dl (the 4 originals; the test asserts each is Outcome::Recrypted, so they are not recrypted), RB_MOD_DIR = recrypted-mods: shipped_mods_bulk_equals_single_with_the_real_key_file ok (4 mods, bulk == single, one hash, == RB_MOD_DIR); corpus_mods recrypt_of_the_originals_matches_the_references ok. rb mod recrypt ~/rb_stuff/Mods.orig -o scratch with the real DAT: 172 recrypted, 0 failed; diff -r against ~/rb_stuff/Mods: identical (172 files). Supersedes the earlier note that the gated tests did not run.

Review round 1 (rebased onto 60103ac): (1) an output ending in .rbm that is not an existing folder is never made a folder: a missing input or several inputs with it are exit 2; the output folder is now created by the first mod written (recrypt.rs one()), so a missing input or a bad key file leaves nothing; new test a_failed_run_creates_no_output_folder (also the review's probe through the binary). (2,3,6) docs/rbm.md 12.5 reworded: 172 originals recrypt in the run, the recrypted ones pass as already recrypted; single-file form needs --force to overwrite (before it overwrote silently); citation fixed; timing measured: a one-mod run incl. the hash is 0.84 s wall on macOS release. (4,5) test comments fixed. (7) bad-magic and wrong-size errors say another edition is not supported. Re-run with RB_RB20FUL, RB_MOD_ORIG_DIR, RB_MOD_DIR: fmt and clippy clean, 498 pass 0 failed (awk sum), gated tests ran; rb mod recrypt of the 172 originals: 0 failed, identical to ~/rb_stuff/Mods.

Review round 2: docs/rbm.md 12.5: rewrapped the list-item continuation that started at column 0 and the 117-column paragraph; no line over 100 columns in 12.5. Docs only, no rerun of tests (498 pass from round 1).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb mod recrypt now verifies the key file and checks the files, and takes many mods. Branch worktree-agent-a2223ff21dcf1f391, one commit. rbm::dat (crates/rbm/src/dat.rs): verify_dat checks magic, length and the SHA-256 of the whole file against embedded::DAT_SHA256 (errors say another CD edition is not supported; the hash error names expected and actual hash and whether the embedded window bytes match); validate_files checks each decrypted .aif/slot sample parses with the engine's AIFF reader and each .jpg starts FF D8 (a damaged-file check, not a wrong-key detector). rb_cli::recrypt (crates/rb-cli/src/recrypt.rs): run(Options)->Report with bulk (files and folders, -o OUTDIR; single form kept, an -o ending .rbm is never made a folder), --force (also in the single form), output folder created only when a mod is written, per-mod failures, summary and exit 1, one lazy hash of the DAT (Report.dat_hashes), already-recrypted mods checked and copied unchanged. docs/rbm.md 12.5 and the CLI help describe it. Testing: cargo test --release -q with RB_MOD_DIR, RB_RB20FUL, RB_MOD_ORIG_DIR: 498 pass, including tests/recrypt.rs (10 tests) and the gated real-DAT bulk test and corpus_mods recrypt_of_the_originals; rb mod recrypt of the 172 originals with the real DAT: 0 failed, byte-identical to ~/rb_stuff/Mods.
<!-- SECTION:FINAL_SUMMARY:END -->

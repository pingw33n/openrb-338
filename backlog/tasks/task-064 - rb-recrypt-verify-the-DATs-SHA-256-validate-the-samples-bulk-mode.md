---
id: TASK-064
title: 'rb recrypt: verify the DAT''s SHA-256, validate the samples, bulk mode'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-27 09:59'
labels:
  - rbm
  - cli
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
- [ ] #1 rb recrypt rejects an RB20FUL.DAT whose SHA-256 differs from the embedded DAT_SHA256, before writing anything, with a clear error
- [ ] #2 Every decrypted sample is checked to parse as AIFF and every JPEG to start with FF D8; a failure is an error naming the file
- [ ] #3 rb recrypt takes several mods or a directory and an output directory, hashing the DAT once per run; one failure does not stop the rest; a summary and a non-zero exit on any failure
- [ ] #4 Tests cover the wrong-hash DAT, a corrupted sample, and bulk equal to single (env-gated); docs/rbm.md section 12 and the CLI help describe it
<!-- AC:END -->

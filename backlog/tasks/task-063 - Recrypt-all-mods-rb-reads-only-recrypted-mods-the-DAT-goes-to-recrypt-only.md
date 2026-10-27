---
id: TASK-063
title: 'Recrypt all mods; rb reads only recrypted mods, the DAT goes to recrypt only'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-27 09:40'
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
- [ ] #1 rb render, compare and play no longer take --rb20ful or RB_RB20FUL; only rb recrypt does; a non-recrypted mod fails with an error naming rb recrypt
- [ ] #2 The 172 mods in ~/rb_stuff/Mods are recrypted, the originals kept in ~/rb_stuff/Mods.orig, and every file and sample verified equal
- [ ] #3 The shipped mods in C:\ReBirth RB-338\Mods are recrypted (originals in C:\re\rb\mods_orig), and an app export with one equals rb compare
- [ ] #4 The mod tests run on recrypted mods without RB20FUL.DAT; compare stays 164/164 and 175/175; cargo test --release passes; docs and AGENTS.md updated
<!-- AC:END -->

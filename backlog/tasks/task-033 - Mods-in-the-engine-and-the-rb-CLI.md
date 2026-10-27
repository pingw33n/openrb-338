---
id: TASK-033
title: Mods in the engine and the rb CLI
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-27 06:25'
labels:
  - rbm
  - cli
milestone: m-2
dependencies:
  - TASK-032
  - TASK-031
  - TASK-029
priority: medium
type: feature
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Use mods in the engine and the CLI: the song's GLOB mod name ("Standard ReBirth" means the built-in kit) selects a mod found in `--mod-dir DIR` (by the INFO name or file name, as the original resolves it), `--mod FILE` overrides it, and a missing mod is an error unless `--standard-kit` is given (the original offers Download/Standard/Cancel). The golden suite covers the mod songs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rb render uses the song's mod by name from --mod-dir, --mod overrides it, and a missing mod behaves as described
- [x] #2 Every mod song of the corpus passes the golden suite
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. crates/rb-cli/src/mods.rs: resolve the song's GLOB mod (INFO name exact as sub_4155D1 does with strcmp at mod+1011, then file name), --mod-dir/RB_MOD_DIR (RB_RBM_DIR deprecated alias), --mod, --rb20ful/RB_RB20FUL + default search, --standard-kit. 2. Renderer builds the mod SampleSet via rbm (per-slot built-in fallback) and passes it through Devices::implemented. 3. rb compare: same options; sets mod,modapp. 4. aiff.rs rate zero test aligned with the fistp-truncated integer. 5. Tests: rb-cli tests/mods.rs (synthetic mod), tests/corpus_mods.rs (real files, env gated); docs/rbm.md section 10.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-27, per task-053: the rb CLI takes --rb20ful PATH (and RB_RB20FUL, then a default location, documented in --help, the same search style as --rebirth-dll), and errors clearly when a song names a non-standard mod and no RB20FUL.DAT or .rbm is found.

Supervisor 2026-10-27, from task-032's review: crates/rb-engine/src/samples/aiff.rs tests the COMM rate for zero on its raw bytes, while the DLL tests the truncated integer after fistp (sub_100016F4). A rate that is non-zero but truncates to 0 (below 1 Hz) is accepted by rb and rejected by the DLL. Align it here if cheap (a unit test with such a rate).

Branch: worktree-agent-aa279c3f47cd1f5ae

IDA (Rebirth.exe, copy): sub_4155D1 = strcmp((char*)(mod+1011), name) over the mod list: the match is the INFO menu name, exact, case-sensitive; the 'v7' compared value comes from anti-tamper arithmetic [G: 0]. IDA (ReBirth.dll) sub_10001674: fldcw with |0xC00 (truncate), fistp into an 8-byte slot, low dword kept -> sub_100016F4 rejects 0; aiff.rs now does that (rate_is_zero + unit test: 0.5 Hz rejected, 1 Hz and 1.999 Hz accepted, 2^32 and infinity rejected). Msg 55 beyond samples: mod id (selects the mod for msg 52/58), file spec, key table, records: nothing else reaches rendering; confirmed by output, all 11 references byte-exact with samples only. Results: rb compare --manifest tests/refs/manifest.json with RB_MOD_DIR, RB_RB20FUL, RB_REBIRTH_DLL set: 175 of 175 equal (145 diag + 11 app + 8 shipped + 8 mod + 3 modapp). cargo test --release with and without the env vars: all pass (corpus_mods skips with a message without them; ignored in debug); cargo test (debug) pass; clippy -D warnings clean; fmt clean. Deviation: the mod golden test is crates/rb-cli/tests/corpus_mods.rs, not corpus_golden.rs, because rb-engine's tests cannot dev-depend on rbm (cycle gives two rb-engine copies); corpus_golden.rs keeps the excluded-songs test with updated comments. Choices: rb compare --manifest includes mod,modapp by default only when a mod dir is given; the key file is needed only when a drum machine plays; default RB20FUL search: mod dir, its parent, next to the binary, Windows install dirs.

Review round 1: addressed all 7 points (--mod INFO-name mismatch warning + test, case-sensitivity test, comment fixed, dummy root() removed, RB_RBM_DIR in --help, [G: 0] split out in docs/rbm.md 10, doc lines wrapped). Process disclosures: (a) I edited some files with small Python scripts run from the scratchpad (aiff.rs, main.rs, mods.rs/tests edits, docs/rbm.md), against the Edit/Write rule; the remaining edits used Edit/Write. (b) IDA: Rebirth.exe was opened from a scratchpad copy, but ReBirth.dll.i64 was opened IN PLACE from the main checkout (/Users/pingw33n/dev/rb/ida/ReBirth.dll.i64), read-only (one disasm of sub_10001674) and closed with save=false; I did not save, but the open itself may have touched the file's mtime. Should have been a scratchpad copy.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb render and rb compare now use the song's mod: found by INFO name (as sub_4155D1) or file name in --mod-dir / RB_MOD_DIR, --mod FILE overrides, missing mod is exit 1 with a message unless --standard-kit (warning + built-in kit), key file via --rb20ful / RB_RB20FUL / default search (task-053). Engine DSP unchanged; the mod enters as the 46-slot SampleSet from rbm::load_samples. AIFF rate zero test aligned with the DLL's truncated integer. rb compare over the whole manifest with the real mods: 175 of 175 equal; corpus_mods test: 8 mod + 3 modapp byte-exact. Tests: cargo test --release (with and without env) pass, clippy -D warnings, fmt. Docs: docs/rbm.md section 10. Decision: task-053.
<!-- SECTION:FINAL_SUMMARY:END -->

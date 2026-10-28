---
id: TASK-097
title: 'rb: mod subcommand with pack, unpack and pack-standard'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 07:40'
updated_date: '2026-10-28 08:01'
labels:
  - cli
  - rbm
milestone: m-2
dependencies: []
references:
  - crates/rb-cli/src/main.rs
  - crates/rbm/src/recrypt.rs
  - crates/rbm/src/standard.rs
  - docs/rbm.md
priority: medium
type: feature
ordinal: 117000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for an `rb mod` subcommand group: `rb mod pack`, `rb mod unpack` and `rb mod pack-standard`, the last being today's `rb standard-mod` renamed. Unpack and pack let a user look inside a mod and build or edit one.

Context (docs/rbm.md sections 2, 12 and 13):
- A mod is an IFF CAT PRBM. It holds a HEAD chunk (256 bytes), EMBF chunks (one file each: the name and payload; at most 200, OB-14), and INFO (1280 bytes: name, version, description, ftp and web URLs; layout in section 2).
- rb reads only recrypted mods (12.4): they are scrambled against the embedded 2047-byte window (KeyTable::embedded) and carry the 1024-byte EMBF rb-recrypt.pad after HEAD.
- The standard mod (section 13, crates/rbm/src/standard.rs) is laid out in the clear and passed through rbm::recrypt with an all-zero table (the identity). That gives a mod rb loads without RB20FUL.DAT and the original loads with its own (13.3). `pack` builds its output the same way.

What:
1. `rb mod pack-standard`: today's `rb standard-mod` with the same options and behavior. Remove the old top-level `standard-mod` command; the `--standard-mod` option of render, compare and play keeps its name. Update every mention of the command in AGENTS.md, docs/ and the help texts, and the tests (crates/rb-cli/tests/standard_mod.rs and others).
2. `rb mod unpack MOD.rbm -o DIR`: takes a recrypted mod and writes every embedded file under its own name into DIR, decrypted with the reader key each file uses (docs/rbm.md 12.1: the DLL byte key for slot samples, the exe u16 key for the others; Rbm::file / file_by_name). It skips rb-recrypt.pad. It writes DIR/mod.toml with:
   - the INFO fields (name, version, description, ftp, web);
   - the HEAD bytes, if they are not the standard HEAD (hex);
   - the file order (the EMBF order matters: the lookup takes the first of a name, and the folder order rule of 11.3).
   A mod that is not recrypted fails with the same hint as render (run rb recrypt first). unpack does not read RB20FUL.DAT; rb recrypt stays the one command that does. DIR must be empty or new, unless --force is given.
3. `rb mod pack DIR -o MOD.rbm`: reads mod.toml and the files, lays the CAT out in the clear (HEAD, then the pad as recrypt does, then the files in mod.toml order, then INFO), and passes it through rbm::recrypt with the identity table, as the standard mod is built. Its checks:
   - every listed file exists;
   - a file in DIR that is not listed is an error, or a warning that it is skipped (choose one and document it);
   - at most 200 EMBF;
   - INFO string lengths fit their fields;
   - the result loads (Rbm::parse, slots_covered).
   Put the packing in the rbm crate (for example rbm::pack), with the CLI as a thin layer, and let standard.rs use it if that simplifies it.
4. Document it in docs/rbm.md: a new section on the unpacked layout and mod.toml, with the round trip.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rb mod pack-standard does what rb standard-mod did (byte-identical output from original/: SHA-256 2d732978...f3e5 per docs/rbm.md 13.2); standard-mod is gone and AGENTS.md, docs and help texts name the new command
- [x] #2 rb mod unpack writes every embedded file (decrypted) and mod.toml; a non-recrypted mod fails with the recrypt hint and RB20FUL.DAT is never read
- [x] #3 rb mod pack of an unpacked mod gives a byte-identical .rbm for the standard mod and for each recrypted mod in RB_MOD_DIR (a test, skipped when the env is absent, recorded as run in the notes)
- [x] #4 A packed mod with an edited file loads in rb (Rbm::parse, slots_covered) and that file reads back changed; pack's errors (missing file, too many files, oversized INFO field) are tested
- [x] #5 docs/rbm.md documents the layout and mod.toml; cargo test, cargo clippy --all-targets -- -D warnings pass
- [x] #6 rb mod recrypt does what rb recrypt did (same options and behavior); the top-level recrypt is gone and AGENTS.md, docs, tools, help texts, error hints and tests name rb mod recrypt; it stays the only command that reads RB20FUL.DAT
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rbm::pack module: Manifest (INFO fields, optional head hex, optional info_raw hex, file order with name/path), unpack(rbm, table), pack(parts) via clear CAT + recrypt with the zero table; standard.rs uses pack. toml =0.9.12 for parsing mod.toml.
2. CLI: group 'mod' with pack, unpack, pack-standard, recrypt; remove standard-mod and recrypt top-level; update hints/docs/tools/tests.
3. Tests: rbm unit tests (round trip, errors), CLI tests (round trip of standard mod and RB_MOD_DIR mods, edited file, missing file, unlisted file, non-recrypted unpack).
4. docs/rbm.md new section on layout and mod.toml. Unlisted file in DIR is an error.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a267cc78d46dc02ad

Scope addition (user via supervisor, 2026-10-28): move rb recrypt to rb mod recrypt, remove the top-level recrypt, update every mention. Added as AC #6.

Design: rbm::pack (crates/rbm/src/pack.rs) has Manifest, unpack, pack, pack_parts; standard.rs now builds through pack_parts. The CLI side is crates/rb-cli/src/modcmd.rs. toml =0.9.12 (2026-02-10) is the one new dependency. Unlisted file in DIR: error (docs/rbm.md 15.3). Duplicate or unsafe names get a { name, path } entry. head/info_raw hex only when the fields cannot reproduce them; none of the 4 shipped mods needs them. The standard mod's INFO description still says 'rb standard-mod' (kept so the SHA-256 2d732978...f3e5 stays; noted in docs 15.4 and a comment in standard.rs).

Evidence: cargo test --release -q with RB_STANDARD_MOD and RB_MOD_DIR set (4 recrypted shipped mods): 474 passed, 0 failed, 2 ignored. crates/rb-cli/tests/mod_pack.rs ran, not skipped ('round trip of 4 recrypted mods ran'); standard mod round trip byte-identical; SHA-256 of the pack-standard output starts 2d732978, ends f3e5. The one skip in the suite is corpus_mods' recrypt-of-originals test (needs RB_MOD_ORIG_DIR and RB_RB20FUL), not this task. clippy --all-targets -D warnings clean, cargo fmt applied, added lines within 100 columns (AGENTS.md and MANIFEST.md lines were already long single lines).

Review round 1 (path safety), all fixed in the amended commit: safe_path is pub and rejects Windows device names (CON, PRN, AUX, NUL, COM1-9, LPT1-9, with extension, spaces or dots before it); unpack picks every path through pick_path, which checks sanitized paths against the used set and bumps the index on a clash; one fold (char::to_lowercase) shared by unpack and pack; unpack writes with create_new; --force decision: it deletes nothing and only replaces a file that was in DIR before the run with the exact name of a mod file, a mod file never replaces another one; pack checks every mod.toml path with safe_path and compares exact names for unlisted files; tests added (hostile names, 001_a_b + a/b, Latin-1 case pair, bad mod.toml paths, NUL, unlisted case variant); docs/rbm.md 15.1 and 15.3 give the full rule. MANIFEST.md: regenerated with tools/rbcorpus.py manifest, it equals the committed text. Re-run: fmt, clippy clean, cargo test --release -q with RB_STANDARD_MOD and RB_MOD_DIR set: 482 passed, 0 failed, 2 ignored (mod_pack ran, 14 tests, not skipped).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added the rb mod group: pack-standard (was standard-mod, byte-identical, SHA 2d732978...f3e5), recrypt (was top-level recrypt, same options; still the only RB20FUL.DAT reader), unpack and pack. The old top-level names are gone (tested: exit 2); AGENTS.md, docs, tools, help texts, error hints and tests name the new commands. rbm::pack does the work (Manifest/mod.toml, unpack, pack_parts through recrypt with the zero table); standard.rs uses pack_parts. pack(unpack(M)) == M byte for byte for the standard mod and the 4 recrypted shipped mods. An edited file reads back changed. Path safety (review round 1): one naming rule (safe_path, device names included), collision-free sanitized paths, a shared Unicode fold, create_new writes, exact-name unlisted check; hostile mods are built at run time with pack_parts. Errors tested (missing file, unlisted file, 200 files, over-long name, bad paths, non-recrypted unpack with the recrypt hint). docs/rbm.md section 15 documents the layout, mod.toml and the naming rule. Tests: 482 passed, 0 failed (release, mods env set), clippy clean. Branch worktree-agent-a267cc78d46dc02ad, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

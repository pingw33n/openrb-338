---
id: TASK-097
title: 'rb: mod subcommand with pack, unpack and pack-standard'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 07:40'
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
- [ ] #1 rb mod pack-standard does what rb standard-mod did (byte-identical output from original/: SHA-256 2d732978...f3e5 per docs/rbm.md 13.2); standard-mod is gone and AGENTS.md, docs and help texts name the new command
- [ ] #2 rb mod unpack writes every embedded file (decrypted) and mod.toml; a non-recrypted mod fails with the recrypt hint and RB20FUL.DAT is never read
- [ ] #3 rb mod pack of an unpacked mod gives a byte-identical .rbm for the standard mod and for each recrypted mod in RB_MOD_DIR (a test, skipped when the env is absent, recorded as run in the notes)
- [ ] #4 A packed mod with an edited file loads in rb (Rbm::parse, slots_covered) and that file reads back changed; pack's errors (missing file, too many files, oversized INFO field) are tested
- [ ] #5 docs/rbm.md documents the layout and mod.toml; cargo test, cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

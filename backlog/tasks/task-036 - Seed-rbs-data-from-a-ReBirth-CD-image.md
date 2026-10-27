---
id: TASK-036
title: 'rb standard-mod: a standard RBM built from the user''s ReBirth.dll'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 14:02'
updated_date: '2026-10-27 09:40'
labels:
  - rbm
  - cli
  - re
milestone: m-2
dependencies:
  - TASK-059
  - TASK-062
  - TASK-063
references:
  - docs/rbm.md
priority: medium
type: feature
ordinal: 33800
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rescoped by the user on 2026-10-27: this task is only about building a standard RBM from the user's own ReBirth installation. The earlier scope (seeding rb from a CD/ISO image: ISO reader, Wise installer carving, CAB/LZX extraction, RB20FUL.DAT copy) is dropped; the CD facts it collected are in this task's history (git log of this file).

`rb standard-mod --rebirth-dll PATH [-o FILE]` writes a .rbm that holds the original's built-in kit as a mod, with the usual components of a mod (docs/rbm.md sections 2-4):
- the IFF CAT/PRBM container with HEAD, INFO (1280 bytes: a non-empty menu name, the version whose major the app checks at 0x415178, the text, ftp and web fields as the shipped mods carry them) and the EMBF chunks;
- the 46 built-in samples from ReBirth.dll's "AIFF" resources 1000..1045 (crates/rb-engine/src/samples.rs reads them; embed them under the 46 names the lookup uses, docs/rbm.md section 3), unchanged byte for byte;
- the standard UI skin: the panel graphics a mod carries (the shipped mods embed `12017.JPG` ... and `splash.jpg`, 59 JPEGs; docs/rbm.md section 2). Find where the original keeps its standard skin (ReBirth.dll or Rebirth.exe resources: bitmaps by id), how the ids map to the mods' JPEG names, and convert them to the format and names mods use; if the skin is in Rebirth.exe, the command also takes `--rebirth-exe PATH`;
- a Demo.rbs if the shipped mods' layout calls for one (e.g. the ReBirth 1.0 Default Song), or say why not.
The mod is scrambled the same way as the mods task-059 produces (the embedded minimum key window and its padding chunk), so rb reads it without RB20FUL.DAT and the original app can load it too.

The menu name: "Standard ReBirth" is the name the app and rb treat as the built-in kit (sub_4166E5; crates/rb-cli/src/mods.rs). Pick a distinct name (e.g. "ReBirth Standard Kit") so the mod is never confused with the built-in kit; say what you chose.

Nothing of Propellerhead's is committed: the command builds the mod from the user's files at run time; tests read ReBirth.dll from original/ (and Rebirth.exe from original/ if needed) and write the mod to a temporary directory.

Using the standard mod (added by the user on 2026-10-27): the commands that need the built-in kit (`rb render`, `rb compare`, `rb play`, and later the UI) read it from the standard mod with `--standard-mod PATH` instead of `--rebirth-dll`. `--rebirth-dll` remains only on `rb standard-mod`. The lookup order mirrors today's `locate_rebirth_dll` (crates/rb-cli/src/main.rs:314-372): the `--standard-mod` path, then an `RB_STANDARD_MOD` environment variable, then a standard mod in rb's default location (say which: next to the rb binary, or the default mod folder), with an error that says how to make one (`rb standard-mod --rebirth-dll ...`). Replace `RB_REBIRTH_DLL` the same way, and keep `--standard-kit` (a missing song mod falls back to the built-in kit, now loaded from the standard mod). Samples: the standard mod's 46 samples are the built-in kit, so a song with a third-party mod falls back per slot to the standard mod's samples. Tests: the CLI tests build the standard mod once from original/ReBirth.dll into a temporary directory and pass it with `--standard-mod` (the engine tests that read original/ReBirth.dll directly may stay as they are); `rb compare` over the manifest must stay 164/164 EQUAL (175/175 with the mod env vars) with the built-in kit taken from the standard mod. Update AGENTS.md's build-and-test commands, the CLI help, and the docs that name `--rebirth-dll` for render/compare.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb standard-mod --rebirth-dll PATH writes a .rbm with HEAD, INFO, the 46 built-in samples (byte-identical to ReBirth.dll's AIFF resources) and the standard UI skin in the mods' format and names, scrambled as task-059's mods are
- [ ] #2 rb loads the produced mod without RB20FUL.DAT, and the corpus songs rendered with it equal their renders with the built-in kit
- [ ] #3 The produced mod loads in the original app on the test machine (it appears under its menu name, shows its skin) and an export with it equals the export with the built-in kit for at least two songs
- [ ] #4 docs/rbm.md documents the standard skin's source (resource ids and the mapping to the JPEG names) and the mod's layout
- [ ] #5 rb render, rb compare and rb play take the built-in kit from --standard-mod PATH (then RB_STANDARD_MOD, then the default location) instead of --rebirth-dll; only rb standard-mod takes --rebirth-dll; the manifest comparison stays 164/164 (175/175 with the mod env vars) with the kit from the standard mod
<!-- AC:END -->

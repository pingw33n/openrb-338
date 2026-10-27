---
id: TASK-036
title: 'rb standard-mod: a standard RBM built from the user''s ReBirth.dll'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 14:02'
updated_date: '2026-10-27 11:17'
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
- [x] #1 rb standard-mod --rebirth-dll PATH writes a .rbm with HEAD, INFO, the 46 built-in samples (byte-identical to ReBirth.dll's AIFF resources) and the standard UI skin in the mods' format and names, scrambled as task-059's mods are
- [x] #2 rb loads the produced mod without RB20FUL.DAT, and the corpus songs rendered with it equal their renders with the built-in kit
- [x] #3 The produced mod loads in the original app on the test machine (it appears under its menu name, shows its skin) and an export with it equals the export with the built-in kit for at least two songs
- [x] #4 docs/rbm.md documents the standard skin's source (resource ids and the mapping to the JPEG names) and the mod's layout
- [x] #5 rb render, rb compare and rb play take the built-in kit from --standard-mod PATH (then RB_STANDARD_MOD, then the default location) instead of --rebirth-dll; only rb standard-mod takes --rebirth-dll; the manifest comparison stays 164/164 (175/175 with the mod env vars) with the kit from the standard mod
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rb-engine: make samples::pe public (resource reader for any named type) so rbm can read the exe's JPEG and the DLL's AIFF resources raw.
2. rbm: new module standard.rs: build_standard(dll, exe) -> Vec<u8>: HEAD (the shipped mods' 9 lead bytes + an rb text), EMBF 58 x <id>.jpg (exe JPEG 12015..12571 in id order), splash.jpg (exe JPEG 903), the 46 AIFF resources unchanged under the slot names (808, 909, Startup.aif), INFO (menu name "ReBirth Standard Kit", version BC 01 00 00, text, ftp empty, web); laid out in the clear and then passed through rbm::recrypt with an all-zero table (identity descramble) so it gets the pad and the window scramble of task-059. Rbm::load_standard_samples: all 46 slots required, no fallback. Unit tests in crates/rbm/tests.
3. rb-cli: rb standard-mod --rebirth-dll PATH [--rebirth-exe PATH, default next to the DLL] [-o FILE, default Standard.rbm next to the rb binary]. render/compare/play: --standard-mod PATH, then RB_STANDARD_MOD, then Standard.rbm next to the rb binary, then a mod named "ReBirth Standard Kit" in the mod folder; error names rb standard-mod. --rebirth-dll and RB_REBIRTH_DLL removed from them. Third-party mods fall back per slot to the standard mod's samples.
4. CLI tests: a shared helper builds the standard mod once per test binary from original/ into a temp dir; rebirth_dll.rs becomes standard_mod.rs (lookup, errors, byte-identity of samples to the DLL resources, equality of renders).
5. Corpus: rb compare --manifest 164/164 and 175/175 with the mod env, golden test, cargo test --release, clippy, fmt.
6. App check on the test machine: a copy of the installation in C:\re\rb\t036 with the standard mod in its Mods; screenshot of the menu and the skin; export two songs with the GLOB renamed to the mod's name vs the built-in; fit/compare.
7. Docs: rbm.md new section (skin source, id mapping, layout), song_load/AGENTS/CLI help updates.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ab3beaa702f4e8398

Research (2026-10-27, ida/Rebirth.exe.i64 copy, headless, closed save=false):
- The standard skin is in Rebirth.exe, not ReBirth.dll: resource type "JPEG" (named type, string at 0x4D0990), 64 resources, language 1053. 58 of them, IDs 12015..12571, are exactly the 58 numbered JPEGs a shipped mod embeds (PBE2, MSM2: same 58 numbers); plus 900, 901 (480x344), 903, 905 (256x256), 904 (171x900), 906 (94x17). ReBirth.dll has only AIFF 1000..1045 and a version resource.
- sub_41703A (the panel bitmap loader, called per bitmap id): when the open mod is not the built-in kit (sub_4166E5 false, 0x4170C5), it asks the mod for Splash.jpg (id 12999 = 0x32C7, 0x4170CB) or sprintf("%d.jpg", id) (0x4170FA) through sub_41678A; found: decoded from the mod (0x41712C). Not found: id 12999 becomes 905 (0x417188); then sub_46E900(id) loads the JPEG resource of that id from the exe, and id 12999 (the built-in kit's splash) loads resource 903 (push 387h at 0x417198). So the standard skin = JPEG resources 12015..12571 as <id>.jpg + resource 903 as splash.jpg; 905 is the splash of a mod without one. Byte for byte: the standard mod embeds the resource bytes unchanged (they are JPEGs already, FF D8 .. JFIF, the format mods use).
- Demo.rbs: sub_417000 = sub_416710("Demo.rbs") != 0 enables a button in the mod info dialog (sub_41A0C5 0x41A3E0); sub_419E92 (via sub_417022 = sub_41678A("Demo.rbs")) saves it as <menu name>.rbs. It is optional and the built-in kit has none. The exe's SONG resources: 128 (15027 bytes, sub_4098EB) and 129 (141008 bytes, an RB40 CAT, loaded by sub_401EE0 as the new-song template), not a demo song; the ReBirth 1.0 Default Song is an installation file, not in the exe or DLL. Decision for the standard mod: no Demo.rbs.
- The CD's Rebirth.exe (SHA-256 e69fef41..., 1,606,656 bytes) differs from original/ (patched for Windows 11). No copy of the CD exe is on this Mac (no ISO); compared on the test machine if one is there, else [G] that the JPEG resources agree.

Implementation and corpus (local):
- rb standard-mod --rebirth-dll original/ReBirth.dll -o Standard.rbm: 2,574,060 bytes, SHA-256 2d732978903422ef30d39e407df79e61f4fd3393fc8b264697271ca0c131f3e5 (deterministic; built twice equal in crates/rbm/tests/standard.rs). 107 chunks: HEAD, rb-recrypt.pad, 58 <id>.jpg, splash.jpg, 46 AIFF, INFO. Key pages = WINDOW both.
- rb compare --manifest tests/refs/manifest.json --standard-mod Standard.rbm (no mod env): 164 of 164 equal, exit 0.
- Same with RB_MOD_DIR=<recrypted shipped mods>: 175 of 175 equal.
- Standard mod found by INFO name in the mod folder (copied as kit.rbm, no --standard-mod, no RB_STANDARD_MOD): --set mod,modapp 11 of 11 equal.
- Script: scratchpad/worktree-agent-ab3beaa702f4e8398/corpus.sh.

App check (test machine, 2026-10-27, owner worktree-agent-ab3beaa702f4e8398, all 6 redc-win-run permits; no Rebirth.exe was running):
- Installation DLL/exe hashes equal original/ (4c3a6441..., 6a651e81...). Copy: robocopy 'C:\ReBirth RB-338' -> C:\re\rb\t036 /XD Mods, Mods holds only Standard.rbm (2d732978...). Installation and C:\re\rb\mods_orig untouched. Both C:\re\rb\t036 and rb/t036run deleted afterwards.
- rb_export.py --app-dir 'C:\re\rb\t036' --remote-dir rb/t036run --restore-prf --startup-sound off: 808_snare_kit.rbs and 909_snare_kit.rbs (diag 808_snare / 909_snare with GLOB mod name "ReBirth Standard Kit", ftp/www empty; scratch mksongs.py) and the unchanged 808_snare.rbs, 909_snare.rbs: all 4 exported (705,644 bytes each), no Download/Standard dialog for the mod songs, user prefs restored (6309526e...), permits released.
- WAVs: 808_snare_kit 3c088b78..., 909_snare_kit 396d3545..., 808_snare 6f7af35f..., 909_snare 203cb051....
- Fit (scratch fit.sh: rb compare SONG --reference WAV --idle-blocks N --standard-mod Standard.rbm --mod-dir <dir with Standard.rbm>, N 2000..12000): 808_snare_kit EQUAL at N=2704, 909_snare_kit at 2704, 808_snare at 2768, 909_snare at 2784. A first search over 8900..9300 found none (the 2 s wait-after-open protocol gives N far below the corpus' 9088).
- Manual session (scratch session.sh: start the copy, Enter for the MIDI box, screenshots, open 808_snare_kit.rbs, taskkill own PID 17680, copy prf_before.prf back, hash 6309526e... checked, release): the Mods menu lists Standard ReBirth and ReBirth Standard Kit; with the song open, ReBirth Standard Kit is checked and the panel is drawn clean (screenshot scratch shots/mods_menu_after.png).
- The ReBirth CD is mounted on the test machine as E: (RB338V20). Its installer's Rebirth.exe (extracted: CAB at 0x30266C of 'Install ReBirth RB-338.exe', bsdtar) has SHA-256 e69fef41..., and its 58 panel JPEG resources equal original/'s byte for byte; 900, 901, 903, 905 differ (original/'s splash/about screens carry a Steinberg logo, the CD's do not). The standard mod built with --rebirth-exe <CD exe> differs only in splash.jpg (2,572,792 bytes). No effect on audio.

Acceptance criteria, evidence:
- AC1: crates/rbm/tests/standard.rs the_standard_mod_holds_the_builtin_kit_and_panel: 107 chunks (HEAD 256, pad, 58 <id>.jpg + splash.jpg, 46 AIFF, INFO 1280 named ReBirth Standard Kit, version 1.0.0); each slot file equals ReBirth.dll's AIFF resource byte for byte; each JPEG equals Rebirth.exe's resource; key pages = embedded WINDOW and all files covered; the stored sample bytes are scrambled; recrypt(mod) == mod. crates/rb-cli/tests/standard_mod.rs the_command_writes_the_standard_mod: the CLI output equals rbm::standard::build.
- AC2: load_all_samples with KeyTable::embedded (no RB20FUL.DAT) equals SampleSet::from_dll; rb compare --manifest with --standard-mod: 164/164 EQUAL, 175/175 with RB_MOD_DIR; corpus_golden green.
- AC3: app copy in C:\re\rb\t036: Mods menu shows ReBirth Standard Kit (checked with the song open), panel clean; app exports of 808_snare and 909_snare with the standard mod equal rb's render with the built-in kit (EQUAL at N=2704 each, the only N in 2000..12000), as do the same songs exported with the built-in kit (N=2768, 2784).
- AC4: docs/rbm.md section 13 (13.1 skin source: JPEG resources, sub_41703A addresses, the id mapping, CD exe comparison, Demo.rbs; 13.2 layout; 13.3 app; 13.4 rb's lookup).
- AC5: render/compare/play take --standard-mod / RB_STANDARD_MOD / Standard.rbm next to the binary / the mod named ReBirth Standard Kit in the mod folder; --rebirth-dll is only on standard-mod (render --rebirth-dll exits 2, tested); manifest 164/164 and 175/175.
Tests: cargo test --release (with RB_MOD_DIR, RB_MOD_ORIG_DIR, RB_RBM_DIR, RB_RB20FUL, RB_THIRDPARTY_DIR) 265 passed, 0 failed; cargo clippy --all-targets -D warnings clean; cargo fmt. main had not moved (7dd0c46): no rebase needed.

Review round 1 (send back, 3 defects + nit), fixed:
- docs/rbm.md 13.2: the mod has 108 chunks (HEAD, pad, 58 JPEGs, splash, 46 AIFF, INFO), not 107 (the earlier note in this task's Implementation entry said 107 too; the test asserts 1+1+59+46+1 = 108).
- AGENTS.md: the kit is built with -o target/Standard.rbm and then passed with --standard-mod target/Standard.rbm in every command (target/ is not next to target/release/rb).
- docs/rbm.md 13.3: reworded to what was shown: each app export with the standard mod equals rb's built-in-kit render at its own fitted N (2704 vs 2768/2784 built in); the two app WAVs of one song differ from each other.
- Nit: pe.rs doc comment rewrapped.
cargo fmt --check and cargo test -p rb-engine --lib after the fix.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb standard-mod --rebirth-dll PATH [--rebirth-exe PATH] [-o FILE] builds "ReBirth Standard Kit" (crates/rbm/src/standard.rs): HEAD, the recrypt pad, the 58 panel bitmaps of Rebirth.exe (JPEG resources 12015..12571 as <id>.jpg) and its splash (resource 903 as splash.jpg), the 46 AIFF resources of ReBirth.dll byte for byte under the lookup names, INFO; laid out in the clear and passed through rbm::recrypt, so it loads with the embedded 2047-byte window. No Demo.rbs (optional in the original; the built-in kit has none). The skin source is Rebirth.exe (sub_41703A: %d.jpg / Splash.jpg from the mod, else exe JPEG resource id, 903 for the built-in splash, 905 for a mod without one). The CD's Rebirth.exe has the same 58 panel bitmaps; only its splash/about screens differ (no Steinberg logo).

rb render/compare/play now take the built-in kit from the standard mod: --standard-mod, RB_STANDARD_MOD, Standard.rbm next to the rb binary, then a mod named ReBirth Standard Kit in the mod folder; --rebirth-dll and RB_REBIRTH_DLL are gone from them. CLI tests build the mod once per test binary (tests/common/mod.rs); rebirth_dll.rs became standard_mod.rs; a new test checks a mod's missing slots come from the standard mod.

Results: manifest 164/164 EQUAL, 175/175 with the mod env; the original app (copy C:\re\rb\t036, deleted afterwards) lists and loads the mod, shows the panel, and its exports of 808_snare and 909_snare with the mod equal rb's built-in-kit render (N=2704). cargo test --release 265 passed; clippy clean. Branch worktree-agent-ab3beaa702f4e8398, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

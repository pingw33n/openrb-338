---
id: TASK-067
title: >-
  Commit assets/Standard.rbm ("Standard ReBirth") and embed it in rb as the
  built-in kit
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 11:21'
updated_date: '2026-10-28 09:54'
labels:
  - rbm
  - cli
  - tests
milestone: m-2
dependencies:
  - TASK-036
references:
  - docs/rbm.md
priority: medium
type: feature
ordinal: 93000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested by the user on 2026-10-27 (corrections the same day: the path is assets/Standard.rbm; the mod is only for the reimplementation, so do not check how the original treats it; the splash comes from the CD's Rebirth.exe). Unassigned, not in a milestone.

The user decided (2026-10-27) to commit the standard mod to the repository and build it into rb. This reverses, for this one file, the rule so far that nothing of Propellerhead's is committed or bundled (task-053's decision, the m-2 tasks, task-037's idea): the file holds the 46 drum samples of ReBirth.dll and the standard panel skin of Rebirth.exe. The repository is private (github.com/pingw33n/rb). Note the interplay with task-066 (removing original/ and ida/ from history): after this task the samples are in the history again, inside assets/Standard.rbm, by the user's choice.

1. `rb standard-mod` (task-036, crates/rbm/src/standard.rs) writes the menu name "Standard ReBirth" in INFO instead of "ReBirth Standard Kit". "Standard ReBirth" is the name songs use for the built-in kit (the song's GLOB mod name; the original's sub_4166E5 compares with it, string at 0x4BB2D0), so a song made with the built-in kit and the embedded mod name the same kit. The mod is for rb only: do not test it in the original app. Update docs/rbm.md section 13 accordingly (task-036 recorded the app check of the "ReBirth Standard Kit" form; say the committed file is the "Standard ReBirth" form for rb).
2. The splash: build with the CD's Rebirth.exe (SHA-256 e69fef4153856f4ad985797a3f312a0027633c00f835c3348415b031bbd87004, 1,606,656 bytes), not original/'s (which carries a Steinberg logo on its splash and about screens). The CD is mounted on the test machine as E:; task-036 extracted the exe from the CAB at file offset 0x30266C of E:\Install ReBirth RB-338.exe with bsdtar (task-036's notes). Get it read-only from the machine (rcs; no permit needed for reading files), check its hash, keep it only in the scratchpad, never commit it. Its 58 panel JPEGs equal original/'s; only splash.jpg changes (the task-036 build with the CD exe was 2,572,792 bytes).
3. Produce `rb standard-mod --rebirth-dll original/ReBirth.dll --rebirth-exe <CD exe> -o assets/Standard.rbm` and commit it (about 2.5 MB; recrypted, so it loads with the embedded key window). Add assets/README.md: what the file is, the command that made it, the SHA-256 of ReBirth.dll, the CD's Rebirth.exe and the result. Record the result's SHA-256 in docs/rbm.md section 13 too.
4. Embed it in rb (include_bytes! in the crate that loads the kit) and use it as the built-in kit: `rb render`, `rb compare`, `rb play` (and the UI later) need no --standard-mod, RB_STANDARD_MOD, Standard.rbm next to the binary, or ReBirth.dll. Keep `--standard-mod PATH` as an optional override (or drop it; say which). Song mods resolve as before; a missing slot of a third-party mod comes from the embedded kit; "Standard ReBirth" in a song's GLOB is the embedded kit.
5. Tests: a gated test (env var naming the CD exe, e.g. RB_CD_REBIRTH_EXE, plus original/ReBirth.dll) checks that `rb standard-mod` reproduces assets/Standard.rbm byte for byte. The CLI tests and, where practical, the engine corpus tests (crates/rb-engine/tests/*, which read original/ReBirth.dll today) use the embedded kit, so they run without original/ — this helps task-066. `rb compare` over the manifest stays 164/164 (175/175 with the mod env vars) with no kit option at all; cargo test --release, clippy, fmt.
6. Update AGENTS.md (the kit commands), the CLI help, docs/rbm.md section 13 and docs/test_machine.md where they name --standard-mod or the kit lookup.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rb standard-mod writes the menu name "Standard ReBirth"; docs/rbm.md section 13 describes the committed form as rb's built-in kit (no original-app check)
- [x] #2 assets/Standard.rbm, built with the CD's Rebirth.exe (SHA-256 e69fef41...) for the splash, is committed with assets/README.md (source files and SHA-256s), and a gated test shows rb standard-mod reproduces it byte for byte
- [x] #3 rb embeds assets/Standard.rbm and uses it as the built-in kit: render, compare and play need no kit option or ReBirth.dll; compare stays 164/164 and 175/175
- [x] #4 The CLI tests (and the engine corpus tests where practical) run without original/; AGENTS.md, the CLI help and the docs are updated; cargo test --release passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Fetch E:/Install ReBirth RB-338.exe from the test machine read-only (rcs get, no permit), extract Rebirth.exe from the CAB at 0x30266C with bsdtar into the scratchpad, check SHA-256 e69fef41...
2. crates/rbm/src/standard.rs: NAME = "Standard ReBirth"; INFO text names rb mod pack-standard; add EMBEDDED (include_bytes!(assets/Standard.rbm)) and a builtin() loader of the 46 samples (and the splash) from it.
3. Build assets/Standard.rbm with rb mod pack-standard --rebirth-dll original/ReBirth.dll --rebirth-exe <CD exe>; assets/README.md with the SHA-256s.
4. Decision: drop --standard-mod, RB_STANDARD_MOD, Standard.rbm next to the binary and the mod-folder lookup from render/compare/play and rb-ui (the kit is part of rb, like the tables; a different kit is a song mod). rb mod pack-standard -o becomes required (no default next to the binary).
5. rb-cli render.rs / mods.rs: the built-in set from rbm::standard; rb-ui session/app/menu/main: the embedded kit and its splash; export through Renderer::new(mods).
6. Tests: common::standard_mod() = assets/Standard.rbm; a gated test (RB_CD_REBIRTH_EXE + original/ReBirth.dll) for byte equality; the tests that read original/ skip when it is absent; engine corpus tests (rb-engine/tests/common) and rb-player/rb-ui tests load the embedded kit (rbm dev-dependency) instead of original/ReBirth.dll.
7. tools/rbhost/hashcmp.py: drop --standard-mod. AGENTS.md, CLI help, docs/rbm.md 13 (13.2 name, 13.4, hash), docs/test_machine.md, docs/ui_spec.md.
8. fmt, clippy, cargo test --release (with and without RB_STANDARD_MOD/RB_MOD_DIR), compare 164/164 and 175/175.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a95dc4dc43ec19f8a (worktree .claude/worktrees/agent-a95dc4dc43ec19f8a), on main bd7b4cf.

CD exe: rcs get 'E:/Install ReBirth RB-338.exe' (read-only, no permit), tail -c +$((0x30266C+1)) > cab, bsdtar -xf: Rebirth.exe SHA-256 e69fef4153856f4ad985797a3f312a0027633c00f835c3348415b031bbd87004, 1,606,656 bytes (kept in the scratchpad only).
Built (NAME "Standard ReBirth", INFO text naming rb mod pack-standard): rb mod pack-standard --rebirth-dll original/ReBirth.dll (4c3a6441...8b12) --rebirth-exe <CD exe> -o assets/Standard.rbm: 2,572,792 bytes, SHA-256 6355b5f14fc2ebfe7ed9e6e42208f97817f96c5e30ed42fdca4205d3cdab5fbe; a second build is byte-identical. The same with original/Rebirth.exe: 2,574,060 bytes, c8752965...9324 (old name form was 2d732978...f3e5).

Decision taken in this task (the task allowed keep or drop): --standard-mod and RB_STANDARD_MOD are dropped from render, compare, play and rb-ui, with the implicit lookups (Standard.rbm next to the binary, a 'ReBirth Standard Kit' mod in the mod folder). The kit is part of rb like its tables; another kit is a song mod. rb mod pack-standard now needs -o (nothing looks next to the binary).
Code: crates/rbm/src/standard.rs EMBEDDED = include_bytes!(assets/Standard.rbm), builtin_samples() (OnceLock, Arc<SampleSet>), builtin_file(name) (the UI splash). rb-cli render.rs: Renderer::new(mods), load_standard and mods::locate_standard_mod/default_standard_mod/STANDARD_MOD_FILE removed; mods::STANDARD = rbm::standard::NAME. rb-ui: session::locate_standard_mod/load_standard removed, App::standard_path removed, App::load_splash takes the embedded splash, kit_samples lays a mod over the built-in kit, the export uses Renderer::new(mods), main drops --standard-mod.
Tests: rb-cli tests use assets/Standard.rbm (common::standard_mod); standard_mod.rs rewritten (gated reproduction with RB_CD_REBIRTH_EXE, the command, the embedded kit with RB_STANDARD_MOD junk ignored and --standard-mod/--rebirth-dll usage errors, 'Standard ReBirth' never looked up in a mod folder); mod_pack.rs checks the asset's SHA-256. rbm/tests/standard.rs: the embedded mod's layout without original/, the original/ build skipped when absent. rb-engine corpus tests (common::samples, rbm dev-dependency cycle), rb-player and rb-ui tests use rbm::standard::builtin_samples().
IDA (copy of ida/Rebirth.exe.i64, headless, closed save=false): sub_4166E5 = strcmp(this+0x63B, "Standard ReBirth") at 0x4166EC..0x4166FA, the string at 0x4BB2D0 [C].
Results: cargo test --release -q with RB_MOD_DIR and RB_CD_REBIRTH_EXE (RB_STANDARD_MOD unset): 488 passed, 0 failed, 2 ignored; with none of the RB_ variables: 488 passed, 0 failed. Gated test with RB_CD_REBIRTH_EXE: passes (rebuild == assets/Standard.rbm == EMBEDDED); without: 'skipped: RB_CD_REBIRTH_EXE ... is not set'.
Without original/ (rsync copy of the worktree minus original/, ida/; scratchpad noorig.sh): 482 passed, 6 failed, all of them tests of the DLL itself, task-066's scope: rb-engine --test samples (all_46_samples_load_with_the_expected_frame_counts, the_ssnd_offset_is_skipped, pe_errors, a_pe_without_aiff_resources_is_rejected) and rb-engine lib tables::tests (static_tables_match_dll, builder_constants_match_dll). Every rb-cli, rbm, rb-player, rb-ui and rb-engine corpus test passes.
Corpus: rb compare --manifest tests/refs/manifest.json with no kit option and RB_STANDARD_MOD unset, no Standard.rbm next to the binary: 164 of 164 EQUAL; with RB_MOD_DIR=recrypted-mods: 175 of 175.
tools/rbhost/hashcmp.py: --standard-mod dropped; python unittest discover -s tools/rbhost: OK. Rebased onto main a35a95a (backlog-only change).

Review round 1 (send back, docs only): ui_spec.md Mods row now names rbm::standard::builtin_samples instead of the deleted session::load_standard; rbm.md 15 says 'section 13, rb mod pack-standard'; hashcmp.py docstring rewrapped to <= 100 columns; rbm.md 13.2 name paragraph and AGENTS.md kit line rewrapped. The [C] on sub_4166E5 is now backed by the call target: IDA copy (headless, save=false) disassembly of 0x4A76E0 (_strcmp, FLIRT name): loads Str1/Str2 from the stack, compares byte by byte (cmp al,[ecx] / cmp ah,[ecx+1] ...), returns 0 (xor eax,eax at 0x4A7720) when a NUL is reached with all bytes equal, else sbb/shl/inc -> -1/1 at 0x4A7724: a C-string compare. sub_4166E5 returns al=1 when it gives 0 (test eax,eax; jnz 0x41670A). Recorded in docs/rbm.md 13.2. fmt and clippy re-run clean; no code change, tests not re-run.

Review round 2 (send back, wording): the 0x4A76E0 description in docs/rbm.md 13.2 and in the round-1 note above was too narrow ('byte by byte'). Corrected: with the first pointer 4-byte aligned (test edx,3 at 0x4A76E8) it loads a dword at a time (mov eax,[edx] at 0x4A76F0) and checks its bytes one by one; the unaligned path (0x4A772C) goes byte by byte. Returns 0 when the strings are equal (0x4A7720), -1/1 at the first difference (0x4A7724).

Review round 3 (send back, wording): the unaligned _strcmp path was incomplete (edx = 2 mod 4 goes to a 2-byte path at 0x4A7748). docs/rbm.md 13.2 is trimmed to the contract: 0x4A76E0 is the CRT's _strcmp (equal strings give 0 at 0x4A7720; the first difference gives -1 or 1 at 0x4A7724). It keeps sub_4166E5's push of 0x4BB2D0 and this+0x63B, the call at 0x4166FA and test eax,eax at 0x416702. The round-1 and round-2 notes' descriptions of the internals are superseded by this. Paragraph rewrapped to <= 100 columns.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb's built-in kit is the committed, embedded standard mod.

- crates/rbm/src/standard.rs: NAME is "Standard ReBirth", the GLOB name of the built-in kit (sub_4166E5, string 0x4BB2D0, checked in IDA); the INFO text names rb mod pack-standard. EMBEDDED = include_bytes!(assets/Standard.rbm); builtin_samples() and builtin_file() read it.
- assets/Standard.rbm: rb mod pack-standard --rebirth-dll original/ReBirth.dll (4c3a6441...8b12) --rebirth-exe <CD Rebirth.exe, e69fef41...7004, scratchpad only> -o assets/Standard.rbm. It is 2,572,792 bytes, SHA-256 6355b5f14fc2ebfe7ed9e6e42208f97817f96c5e30ed42fdca4205d3cdab5fbe. assets/README.md records the sources.
- The gated test rb-cli tests/standard_mod.rs::the_command_reproduces_the_committed_mod (RB_CD_REBIRTH_EXE plus original/ReBirth.dll) rebuilds it byte for byte.
- render, compare, play and rb-ui (the panel, the 808 mod window's splash, the export) use the embedded kit. --standard-mod, RB_STANDARD_MOD and every kit lookup are dropped, consistently in the CLI and the UI (a decision the task left open: the kit is part of rb). rb mod pack-standard requires -o.
- The rb-cli, rbm, rb-player, rb-ui and rb-engine corpus tests run without original/. The 6 tests left that check ReBirth.dll itself (rb-engine samples.rs, tables::tests) are task-066's to skip.
- Docs: AGENTS.md, CLI help, docs/rbm.md 1, 10, 13 (13.2 table of builds and the name, 13.3 marked as the task-036 form, 13.4 rewritten), 15.4, docs/ui_spec.md. docs/test_machine.md names no kit lookup, so it is unchanged. tools/rbhost/hashcmp.py no longer passes --standard-mod.

Tests:
- cargo test --release: 488 passed, 0 failed (with RB_MOD_DIR and RB_CD_REBIRTH_EXE, RB_STANDARD_MOD unset; and with no RB_ variables).
- rb compare over the manifest: 164/164 EQUAL with no kit option, 175/175 with RB_MOD_DIR.
- clippy -D warnings and fmt are clean; the tools/rbhost python tests pass.

Branch worktree-agent-a95dc4dc43ec19f8a, one commit on main a35a95a.
<!-- SECTION:FINAL_SUMMARY:END -->

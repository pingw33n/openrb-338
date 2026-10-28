---
id: TASK-067
title: >-
  Commit assets/Standard.rbm ("Standard ReBirth") and embed it in rb as the
  built-in kit
status: To Do
assignee: []
created_date: '2026-10-27 11:21'
updated_date: '2026-10-28 09:19'
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
- [ ] #1 rb standard-mod writes the menu name "Standard ReBirth"; docs/rbm.md section 13 describes the committed form as rb's built-in kit (no original-app check)
- [ ] #2 assets/Standard.rbm, built with the CD's Rebirth.exe (SHA-256 e69fef41...) for the splash, is committed with assets/README.md (source files and SHA-256s), and a gated test shows rb standard-mod reproduces it byte for byte
- [ ] #3 rb embeds assets/Standard.rbm and uses it as the built-in kit: render, compare and play need no kit option or ReBirth.dll; compare stays 164/164 and 175/175
- [ ] #4 The CLI tests (and the engine corpus tests where practical) run without original/; AGENTS.md, the CLI help and the docs are updated; cargo test --release passes
<!-- AC:END -->

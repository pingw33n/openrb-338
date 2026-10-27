---
id: TASK-067
title: >-
  Commit asset/Standard.rbm ("Standard ReBirth") and embed it in rb as the
  built-in kit
status: To Do
assignee: []
created_date: '2026-10-27 11:21'
labels:
  - rbm
  - cli
  - tests
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
Requested by the user on 2026-10-27. Unassigned, not in a milestone.

The user decided (2026-10-27) to commit the standard mod to the repository and build it into rb. This reverses, for this one file, the rule so far that nothing of Propellerhead's is committed or bundled (task-053's decision, the m-2 tasks, task-037's idea): the file holds the 46 drum samples of ReBirth.dll and the standard panel skin of Rebirth.exe. The repository is private (github.com/pingw33n/rb). Note the interplay with task-066 (removing original/ and ida/ from history): after this task the samples are in the history again, inside asset/Standard.rbm, by the user's choice.

1. `rb standard-mod` (task-036, crates/rbm/src/standard.rs) writes the menu name "Standard ReBirth" in INFO instead of "ReBirth Standard Kit". "Standard ReBirth" is the name songs use for the built-in kit (GLOB mod name; the original's sub_4166E5 compares the open mod's name with it, string at 0x4BB2D0, and then uses ReBirth.dll's own samples), so a song made with the built-in kit and a song saved with this mod name the same kit. Check what the original does with a mod file of that name in its Mods folder (the duplicate-name rule of task-058, docs/rbm.md 11.3, and sub_4166E5): on the test machine, from a copy of the installation under C:\re\rb\<folder> (never the installation itself; tools/rb_export.py --restore-prf), and record it in docs/rbm.md section 13. If the original rejects or misbehaves with it, say so; the file is meant for rb, not for the original.
2. Produce the mod with `rb standard-mod --rebirth-dll original/ReBirth.dll --rebirth-exe original/Rebirth.exe -o asset/Standard.rbm` and commit it (about 2.5 MB; recrypted, so it loads with the embedded key window). Record its SHA-256 in docs/rbm.md section 13 and in asset/README.md (what it is, how it was made, from which DLL and exe with their SHA-256s). Which Rebirth.exe gives the splash: original/'s carries a Steinberg logo, the CD's differs (task-036 notes); use original/'s unless the user says otherwise, and say which.
3. Embed it in rb (include_bytes! in the crate that loads the kit) and use it as the built-in kit: `rb render`, `rb compare`, `rb play` (and the UI later) need no --standard-mod, RB_STANDARD_MOD, Standard.rbm next to the binary, or ReBirth.dll. Keep `--standard-mod PATH` as an optional override (or drop it; say which). Song mods resolve as before; a missing slot of a third-party mod comes from the embedded kit; "Standard ReBirth" in a song's GLOB is the embedded kit.
4. Tests: a gated test (when original/ReBirth.dll and original/Rebirth.exe exist) checks that `rb standard-mod` from them reproduces asset/Standard.rbm byte for byte. The CLI tests and, where practical, the engine corpus tests (crates/rb-engine/tests/*, which read original/ReBirth.dll today) use the embedded kit, so they run without original/ — this helps task-066. `rb compare` over the manifest stays 164/164 (175/175 with the mod env vars) with no kit option at all; cargo test --release, clippy, fmt.
5. Update AGENTS.md (the kit commands), the CLI help, docs/rbm.md section 13 and docs/test_machine.md where they name --standard-mod or the kit lookup.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb standard-mod writes the menu name "Standard ReBirth"; what the original does with such a mod in its Mods folder is checked on a copy and recorded in docs/rbm.md section 13
- [ ] #2 asset/Standard.rbm is committed with asset/README.md (source files and SHA-256s), and a gated test shows rb standard-mod reproduces it byte for byte
- [ ] #3 rb embeds asset/Standard.rbm and uses it as the built-in kit: render, compare and play need no kit option or ReBirth.dll; compare stays 164/164 and 175/175
- [ ] #4 The CLI tests (and the engine corpus tests where practical) run without original/; AGENTS.md, the CLI help and the docs are updated; cargo test --release passes
<!-- AC:END -->

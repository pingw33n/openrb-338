---
id: TASK-161
title: >-
  orb mod recrypt: rescue files stored unscrambled (a zero key half) in mods
  made with another key file
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-30 10:45'
updated_date: '2026-10-30 10:45'
labels:
  - rbm
  - cli
milestone: m-5
dependencies:
  - TASK-160
references:
  - crates/orb-cli/src/recrypt.rs
  - docs/rbm.md
priority: medium
type: feature
ordinal: 178000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: of the five mis-scrambled mods found on 2026-10-30, the follow-up of the task that adds the past-1024 checks, several files are stored unscrambled. Their half of the key table was all zeros when the mod was made:
- mellow.rbm and House of love Juam1 .rbm: every file;
- TurnMod.rbm: the even-key files;
- AloNaNbox 2.0.rbm: the odd-key files.

These files can be recovered exactly. The rest (TurnMod's odd half, AloNaNbox's even half, all of MaShMoD) need the other key file and stay unrecoverable.

What:
1. In recrypt, when a file fails the past-1024 check with the CD's key, try the file as stored (a zero key). If it then passes (a JPEG end marker, an audio plausibility check), take that plaintext and recrypt it with the embedded window as usual. Apply this per file, never per mod, and only when the CD key fails and the zero key passes.
2. A mod in which every file passes after that is written, with a note in the summary such as "N files were stored unscrambled (made with another edition's key file); recovered". A mod with any file that passes neither way is still refused. Name the files. The partial case is left to a later decision, or a --force-partial that keeps the built-in sample for a lost slot, if the user wants it.
3. Check that the original ReBirth then loads the recovered mods: the samples play and the panel art shows. Do it on the test machine if a run fits (docs/test_machine.md; permits as usual; never write under C:\ReBirth RB-338), otherwise with rbhost --dump-samples, and compare the decoded samples with the plaintext.
4. Tests: a synthetic mod with half its files unscrambled is recovered, and the output descrambles to the original plaintext. A mixed mod with one foreign-key file is refused.
5. Verify on the user's folder, read only, into a scratch folder. Expected: mellow.rbm and House of love Juam1 .rbm are recovered; TurnMod, AloNaNbox and MaShMoD are still refused. Delete the output. Update docs/rbm.md 12.5.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 recrypt recovers a file that fails with the CD key but passes as stored, per file; a mod is written only if every file passes one way, with a summary note
- [ ] #2 mellow.rbm and House of love Juam1 .rbm are recovered from the user's folder; TurnMod, AloNaNbox and MaShMoD are refused; the recovered mods load in the original (test machine or rbhost)
- [ ] #3 Synthetic tests cover recovery and refusal; docs/rbm.md is updated; fmt, clippy and cargo test --release pass
<!-- AC:END -->

---
id: TASK-161
title: >-
  orb mod recrypt: rescue files stored unscrambled (a zero key half) in mods
  made with another key file
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-30 10:45'
updated_date: '2026-10-30 12:01'
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
- [x] #1 recrypt recovers a file that fails with the CD key but passes as stored, per file; a mod is written only if every file passes one way, with a summary note
- [x] #2 Synthetic tests cover recovery and refusal; docs/rbm.md is updated; fmt, clippy and cargo test --release pass
- [x] #3 mellow.rbm and House of love Juam1 .rbm are recovered from the user's folder (every file: 77 and 106); TurnMod, AloNaNbox and MaShMoD are refused; the recovered files equal the correctly scrambled twin mods (mellow 104 of 104 identical; House of love 104 of 106, Demo.rbs and Splash.jpg being that mod's own, valid content)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rbm::recrypt_with (files taken as stored by ordinal). 2. rbm::dat::check_and_rescue: per file, JPEG by check_file as stored; AIFF by jump share (key >0.45 and stored <0.3); undecidable files follow the key-half parity of a sure stored file. 3. recrypt.rs uses it, summary note. 4. Measure on 317 originals + 171 recrypted, tests (rbm + cli), docs/rbm.md 12.5. 5. Verify on the five, rbhost if the machine is free.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a67119c49d8c15d82

Measurements (throwaway example, deleted): AIFF share with CD key vs as stored over 317 originals + 171 recrypted (480 mods, 9 with any file with CD share >0.45): good mods 0 files with CD>0.45 and stored<0.3; the noise drums above 0.45 (Heavy Rhythm Machine 2, Inner mod, Magica, Neurotek 1 each) have stored share 0.556..0.570. Plain-stored files: stored share 0.000..0.069 (mellow 44/44, House of love 46/46, TurnMod 21/44, AloNaNbox 23/46), CD share 0.49..0.61. MaShMoD 0 hits (stored 0.526..0.564). Run over 317 originals: 314 written, TurnMod (27 JPEGs), AloNaNbox (13), MaShMoD (45) refused; mellow 68 files and House of love 98 recovered. Recovered mellow vs twin Mellow (SouthPark).rbm: 95/104 identical (all samples; 9 differing are 8 panel JPEGs + Demo.rbs); House of love Juam1 vs House of love Juamø: 106/106 identical. Design addition: files that cannot decide (noise drum, Demo.rbs with no check) are taken as stored if a sure stored file shares their key parity (the key halves are chosen by parity); a lone noise drum in its half stays keyed (undetectable). Fixtures: tests' jpeg data was FF-free so the keyed version passed the JPEG rule; made noise-like with FF 00 stuffing. Item 3 (original loads the mods): rbhost run.py --mod refuses, Rebirth.exe (PID 12704, not started by me) is running on the machine; I did not kill it. NOT checked; evidence instead is the byte equality with the twins.

The part of the old AC #2 'the recovered mods load in the original (test machine or rbhost)' moved to task-163 (Rebirth.exe was running on the test machine).

Review round 1: the 15 JPEGs (mellow 12015, 12016, 12068, 12558-12562; House 12016, 12019, 12506, 12509, 12550, 12567, 12571) were NOT rescued, they were descrambled with the CD key. Cause (checked with a throwaway example using the twin as known plaintext): stored == twin plaintext for all 104 files of mellow, so the key half is zero over every window (the 'partly zero' guess is refuted); the real bug is that check_file/jpeg_end_ok treated file bytes up to 1024 as never scrambled, but the scramble counts from the payload start (name + NUL first): a file's clear part is 1024 - (name.len()+1), 1014 for 12016.jpg. Short JPEGs (1015..1229 bytes) passed as keyed noise. Fixes: dat::clear_len, jpeg_end_ok(file, clear); a rescued JPEG needs the strict FF D9 end (new jpeg_has_end) stored and not keyed, no cut-short leniency; undecidable files follow their key half only if their window (key_need) lies within the entries proven zero by sure stored files of that half (max key_need per parity). Re-measured over the 317 originals: 314 written, mellow 77 files recovered, House of love 106, TurnMod (27 JPEGs), AloNaNbox (15), MaShMoD (45) refused; only these two mods carry a rescue note (the other 312 report none). Twin compare by name: mellow 104/104 identical; House 104/106 identical, Demo.rbs (valid RBS, tools/rbs.py check ok, mod 'Template Mod') and Splash.jpg (FF D8..FF D9) differ as that mod's own content. My earlier 'House 106/106 identical' was wrong: its compare glob matched the mod itself. Earlier counts 68/98 were from the buggy rule. ~/rb-mods/mods (171) is all recrypted and does not exercise the rescue; good-mod evidence is the 312 untouched originals. Tests added: noise JPEG 1100/1250/1500 bytes without marker is not rescued; short JPEG with FF D9 stored is; a file follows its key half only where proven zero (rbm.rs), CLI Demo.rbs shrunk to 1200 bytes so its window is proven. Result: cargo test --release 661 pass 0 failed, fmt, clippy clean, corpus 175/175.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
recrypt recovers files stored unscrambled per file (rbm::dat::check_and_rescue, rbm::recrypt_with); the mod is written only if every checked file passes, with a summary note. Over the 317 originals: 314 written (mellow 77 files and House of love Juam1 106 files recovered), TurnMod, AloNaNbox, MaShMoD refused naming JPEGs; the other 312 report no rescue. Recovered files equal the twin mods' (mellow 104/104; House 104/106, Demo.rbs and Splash.jpg are that mod's own). Rule: JPEG stored ends with FF D9 and keyed does not; AIFF jump share with key >0.45 and as stored <0.3; undecidable files follow their key half only within the entries proven zero. Review fix: the clear part of a file is 1024 minus its name and NUL (dat::clear_len). Tests: cargo test --release 661 pass 0 failed (3 env vars), fmt, clippy clean, corpus 175/175. The load in the original is task-163. Branch worktree-agent-a67119c49d8c15d82.
<!-- SECTION:FINAL_SUMMARY:END -->

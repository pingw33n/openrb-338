---
id: TASK-160
title: >-
  orb mod recrypt: refuse a mod whose files do not descramble correctly (check
  past byte 1024)
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-30 10:45'
updated_date: '2026-10-30 11:18'
labels:
  - rbm
  - cli
milestone: m-5
dependencies: []
references:
  - crates/orb-cli/src/recrypt.rs
  - crates/rbm/src/lib.rs
  - tools/rbm.py
  - docs/rbm.md
priority: high
type: bug
ordinal: 177000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on 2026-10-30 the user found that `orb mod recrypt` accepts some mods that the original fails to load, and writes corrupted output for them. The supervisor checked the user's folder ~/rb-stuff-peff/ReBirth Mods (317 mods; the user's, read only) with tools/rbm.py check-dir and the CD's RB20FUL.DAT. Five mods were scrambled with a key table that is not the 2001 CD's:
- mellow.rbm, keys 72963593 and 134110473;
- House of love Juam1 .rbm, keys 3876875 and 90141265;
- TurnMod.rbm, keys 10752749 and 50644384;
- AloNaNbox 2.0.rbm, keys 118557743 and 114850228;
- MaShMoD.rbm, keys 20373127 and 84469203.

The scrambling adds key bytes to every payload byte from 1024 on (docs/rbm.md 4). The two halves of the key table are chosen by the key's parity. In these mods one or both halves are either all zeros (the files are stored unscrambled) or foreign bytes found nowhere in the 2001 RB20FUL.DAT:
- mellow and House of love: both halves zero;
- TurnMod: even keys zero, odd foreign;
- AloNaNbox: odd keys zero, even foreign;
- MaShMoD: both halves foreign.

Two pieces of evidence:
- mellow.rbm against its correctly scrambled twin Mellow (SouthPark).rbm gives key bytes of exactly 0, 310 of 310;
- TurnMod.rbm against DJ Turn's MOD.rbm gives a consistent 1806-byte foreign half.

The original descrambles with its own key and gets garbage, so the JPEGs are broken and the audio is noise. recrypt's checks look only at an AIFF header that parses and a JPEG that starts with FF D8, both in the first 1024 bytes, which are never scrambled. So it passes these mods and writes recrypted garbage. tools/rbm.py check-dir already reports them as "N JPEGs without an end marker".

What:
1. After descrambling, check what lies past byte 1024 where it can be checked:
   - Every .jpg/.jpeg longer than 1024 bytes must contain the JPEG end marker FF D9 at its end, allowing trailing zero padding. Confirm the padding rule over the user's two mod folders, ~/rb-mods/mods (171 recrypted) and ~/rb-stuff-peff/ReBirth Mods (317 originals), so that no good mod is refused. Use check-dir's own rule, the one that flagged exactly these five.
   - For AIFF, add a cheap plausibility test of the 16-bit audio past byte 1024. One option is the share of large sample-to-sample jumps compared with real samples. A file scrambled with a wrong key looks like uniform noise: measure the statistic on the good mods and the bad five, and pick a threshold with a clear margin. If no statistic separates them reliably, rely on the JPEG check alone and say so.
2. A mod that fails is not written, with an error naming the file and saying it was probably scrambled with another edition's key file. Bulk mode goes on with the other mods, as today.
3. Apply the same check where orb loads a recrypted mod, if that is cheap. A recrypted file made earlier from such a mod, such as the user's ~/rb-stuff-peff/mods_recrypted/mellow.rbm, should be refused or warned about rather than played as noise. Decide which, and record the decision.
4. Tests: synthetic mods (built in the test; the rbm crate can pack) where the payload past 1024 is scrambled with a wrong table or not at all, must be refused. Good mods must still pass: the shipped four from ORB_CD_ISO, and the gated thirdparty mods test.
5. Verify: run recrypt over ~/rb-stuff-peff/ReBirth Mods into a scratch folder and check that exactly the five above fail. Delete the scratch output. Update docs/rbm.md 12.5 with the checks and the five-mod finding (names and key offsets only, no mod bytes).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 recrypt checks descrambled JPEGs past byte 1024 (end marker) and, if a reliable statistic exists, the audio; a mod that fails is not written and the error names the file and the likely cause
- [x] #2 Over ~/rb-stuff-peff/ReBirth Mods exactly the five mods listed fail; ~/rb-mods/mods and the shipped mods all pass
- [x] #3 Synthetic tests cover a wrong-key and an unscrambled payload; docs/rbm.md 12.5 is updated; fmt, clippy, cargo test --release and the gated thirdparty test pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Measure on the 317 originals (CD key) and the 171 recrypted: JPEG end rule and a 16-bit jump-share statistic. 2. rbm::dat: check_file (per file, reusable by task-161), file_jump_share/audio_is_noise (per mod), validate_files uses them; RbmError::WrongKey; load_samples refuses a noise mod. 3. Tests (rbm crate synthetic, orb-cli recrypt fixture), docs/rbm.md 12.5. 4. Verify on both folders, fmt/clippy/tests/corpus.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a6b6def2e171cb304

Measurements (python, tools/rbm.py descrambling with the CD DAT; 317 originals, 171 recrypted): per-file jump share (|diff|>16384 between neighbouring words past byte 1024) is NOT separable per file: good noise drums reach 0.696 (Heavy Rhythm Machine TR808SD5), 0.644 (Magica TR808HC), 0.524 (Neurotek, Inner mod). Per mod it separates: share of files >0.3 is <=0.087 on 312 good originals (min-over-files <=0.0007) and 1.000 on all five bad (files 0.497..0.61, median 0.557..0.577). Rule: >=4 files and more than half >0.3. JPEG: check-dir's end-marker rule flags the five AND YellowBirth.rbm in ~/rb-mods/mods (59 JPEGs cut short by its packer, docs/rbm.md 11.1), so the plain rule would refuse a mod AC2 says must pass. Added a second clause: without FF D9 a JPEG passes if every FF past 1024 is followed by 00,01,FF or C0..FF; YellowBirth 59/59 pass, bad five have 0..8 of 32..59 such files (short) and the rest fail. Results: recrypt of 317 originals: 312 recrypted, exactly the five failed (each by its first JPEG); ~/rb-mods/mods 171 already recrypted, 0 failed (YellowBirth included); ~/rb-stuff-peff/mods_recrypted: 3 failed (mellow, TurnMod, AloNaNbox), 169 pass. Decision (item 3): load_samples refuses (WrongKey), not warns. Item 3 loads decoded audio, rule same. Scratch deleted at the end.

Review round 1: (1) audio refusal now names the first 3 noisy files and the count (dat::check_audio), tests assert a file name; (2) load_samples measures the file bytes past 1024 (file_jump_share), as validate_files, test with 600-1000 frame noise samples added; (3) false-accept limit and the 66/256 set corrected in dat.rs and docs/rbm.md; (4) real test count is 655 pass, 0 failed (earlier note of 557 was wrong, my awk summed only the first fields); (5) commit body wrapped; (6) validate_files doc, duplicate 'past byte 1024' in messages, FF 00 stuffing in the cut-short test. Re-run: recrypt of 317 originals 312 written / 5 failed (same five), ~/rb-mods/mods 171 / 0 failed; fmt, clippy clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
recrypt and Rbm::load_samples now check past byte 1024 (rbm::dat::check_file per file, check_audio/audio_is_noise per mod, RbmError::WrongKey naming the files). Over ~/rb-stuff-peff/ReBirth Mods: 312 recrypted, exactly the five refused; ~/rb-mods/mods 171 pass (YellowBirth needed the JPEG second clause: cut-short JPEGs are valid data); mods_recrypted refuses mellow, TurnMod, AloNaNbox. Audio statistic is per mod only (noise drums reach 0.696 per file). Tests: cargo test --release 655 pass, 0 failed with gated tests, fmt, clippy, corpus 175/175. Decision recorded: loading refuses. Branch worktree-agent-a6b6def2e171cb304.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-061
title: 'rbsmsg.py: the 4.0 mixer converter, so rbhost renders RBS 4.0 songs'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 08:38'
updated_date: '2026-10-27 12:36'
labels:
  - tools
  - tests
milestone: m-2
dependencies: []
references:
  - docs/song_load.md
  - tools/rbhost/rbsmsg.py
priority: low
type: enhancement
ordinal: 54000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: tools/rbhost/rbsmsg.py (the .rbs to engine-message converter that rbhost renders from) raises 'file version 4.0: the 4.0 mixer converter is not implemented' (mixer_payload), so no RBS 4.0 song can be rendered by the original DLL through rbhost. task-058 met two such songs in the user's collection (RB-SEMx.rbs and a TB-909e mkII song under ~/rb_stuff/CaveysRebirthStuff, read in place via RB_THIRDPARTY_DIR, never committed); rb itself loads 4.0 files (crates/rb-engine/src/load.rs implements the exe's 4.0 MIXR converter 0x4352C0 and the 4.0 mixer-track converter sub_437380, docs/song_load.md section 5.1).
Steps: port the 4.0 MIXR and mixer-track conversion into rbsmsg.py from docs/song_load.md (and load.rs), with unit tests in tools/rbhost/rbsmsg_test.py; then render the two songs with run.py --mod (--remote-dir, --idle-blocks 9088) and compare with rb render.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rbsmsg.py converts an RBS 4.0 song's mixer as the exe does (unit tests)
- [x] #2 RB-SEMx.rbs and the TB-909e mkII song render with rbhost and equal rb render byte for byte
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Port the 4.0 MIXR converter (0x4352C0) and the mixer-track converter sub_437380 from docs/song_load.md 4.1/5.1 and crates/rb-engine/src/load.rs into tools/rbhost/rbsmsg.py (exact-version tests like the exe's sub_47DA82), unit-test them, recrypt the two mods for rb, render RB-SEMx and TB-909e mkII b7x7 with rbhost (--mod original, --rb20ful, --idle-blocks 9088) and compare with rb render by --match-hash.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a18b59e9aa1dcdd8e

rbsmsg.py: version_key/is_4_0/is_4_1 (CVersion key incl. HEAD byte 7, equality as sub_47DA82: 4.0.1 and 4.1.1 get no converter/forcing); mixer_payload_40 (enabled=1, comp id = MIXR[1]!=0 then c+2 per comp flag, PCF id c+1 per PCF flag, last channel wins, MIXR[2] unread); Mixer40 (tracker of last comp/PCF flag tick, updated before the tick check, signed compare; off flag kept as value 0 only when tick > tracker; ctrl 4+8c unaccepted, 5..8+8c -> 6..9+6c, 9+8c -> 2, 10+8c and 1 -> 1); track_batches(..., mixer40=) used for track 0 of an exact 4.0.0 file. Previously the 4.1 check ignored the revision; now exact.

Tests: tools/rbhost/rbsmsg_test.py +5 (mixer_40, mixer_40_by_version, renumbering, flags, invalid tick moves tracker). python3 -W error::ResourceWarning unittest discover -s tools/rbhost: 42 pass; -s tools: 65 pass (3 skipped).

Run (machine, one redc-win-run permit per run, released; remote folder C:\re\rb\t061 deleted afterwards): mods recrypted for rb with rb recrypt --rb20ful (scratch t061/mods); rb render --idle-blocks 9088 --standard-mod target/Standard.rbm (RB_MOD_DIR=mods). Then run.py --remote-dir t061 --idle-blocks 9088 --mod ~/rb_stuff/Mods.orig/<mod>.rbm --rb20ful RB20FUL.DAT --match REF.wav --match-hash SONG. Songs: ~/rb_stuff/CaveysRebirthStuff/Mod Demo Songs/RB-SEMx.rbs (80181 frames, sha256 6e48cecfe133786506db499cbcd0f97ef6e914f5ca37e0a463a748014aaf3ac9) and TB-909e mkII b7x7.rbs (5093052 frames, sha256 2b8773ce86e5d4fcbec3274cd826b822e43fb92fab81a234704fc63ff5ac48ea): both 'identical: True', rbhost sha256 == rb render sha256. Note hashcmp.py is built-in-mod only, so these used run.py --match-hash (same comparison by sha256). Scripts in scratchpad t061/go.sh, rbh.sh.

Caveat: the Rust and Python ports are both written from docs/song_load.md, so equality shows they agree and the DLL takes the messages; the exe's own export of a 4.0 song was not run (docs/song_load.md notes it as [G]). Docs updated: song_load.md (open items), rbm.md 11.4.

Review round 1 (all five fixed): (1) song_load.md 4.1 no longer says no 4.0 song was available. (2) test_mixer_40_by_version asserts fixed payloads for 4.0.0 (converted), 4.0.1, 4.1.0 (forced), 4.1.1 (neither) and 4.2.0 (as stored). (3) new test_load_messages_dispatch: load_messages converts track 0 (ctrl 5 -> 6) only for 4.0.0, not 4.0.1/4.1/4.2. (4) Mod files per song, exactly: both songs name their own mod in GLOB (RB-SEMx.rbs -> RB-SEMx, TB-909e mkII b7x7.rbs -> TB-909e mkII), not the standard kit. rbhost loaded the original ~/rb_stuff/Mods.orig/RB-SEMx.rbm and ~/rb_stuff/Mods.orig/TB-909e mkII.rbm with RB20FUL.DAT (run.py --mod/--rb20ful, uploaded to C:\re\rb\t061\x.rbm); rb render loaded the recrypted copies made with rb recrypt from those same files, scratchpad t061/mods/RB-SEMx.rbm and t061/mods/TB-909e mkII.rbm, found by GLOB name through RB_MOD_DIR; the built-in drum kit came from target/Standard.rbm. (5) version_key reads major, minor and revision all from head.build() (HEAD bytes 5..7, version string applied over 5 and 6). Tests now: rbhost 43 pass, tools 65 pass.

Review round 2: commit message Testing line corrected from 42 to 43 rbhost tests.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rbsmsg.py now converts RBS 4.0.0 songs (4.0 MIXR 0x4352C0 and mixer track sub_437380), with exact-version gating like the exe. RB-SEMx.rbs and TB-909e mkII b7x7.rbs render with rbhost (original DLL, original mods) byte-identically to rb render (sha256 match, --idle-blocks 9088). Tests: rbsmsg_test +5, rbhost 42 pass, tools 65 pass. Branch worktree-agent-a18b59e9aa1dcdd8e, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

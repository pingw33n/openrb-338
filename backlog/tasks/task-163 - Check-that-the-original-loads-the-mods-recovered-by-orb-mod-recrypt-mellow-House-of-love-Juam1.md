---
id: TASK-163
title: >-
  Check that the original loads the mods recovered by orb mod recrypt (mellow,
  House of love Juam1)
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-30 11:42'
updated_date: '2026-10-30 12:16'
labels:
  - rbm
  - testmachine
milestone: m-5
dependencies:
  - TASK-161
references:
  - tools/rbhost/run.py
  - docs/test_machine.md
  - docs/rbm.md
priority: low
type: task
ordinal: 179000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-161 recovers files stored unscrambled in mods made with another key table. Its check that the original ReBirth loads the recovered mods was not done, because Rebirth.exe was running on the test machine. The agent may not start or kill it. The byte equality of the recovered files with the correctly scrambled twin mods is the current evidence:
- mellow.rbm against Mellow (SouthPark).rbm: all 44 samples equal;
- House of love Juam1 .rbm against House of love Juamø.rbm: 106 of 106 files equal.

What:
1. With orb, recrypt ~/rb-stuff-peff/ReBirth Mods/mellow.rbm (INFO name 'southpark Mod') and 'House of love Juam1 .rbm' ('Template Mod') into a scratch folder. These are the user's files: read only.
2. Build a song that names each mod: tools/rbs.py build with glob.mod_name.
3. Run tools/rbhost/run.py --mod MOD --rb20ful RB20FUL.DAT --dump-samples --remote-dir NAME -o OUT SONG.rbs.
4. Compare OUT/*.samples/slotNN.bin with the AIFF audio of the twin mods. Expected: equal.

Test machine rules:
- Use C:\re\rb\<folder> and delete it afterwards.
- Never write under C:\ReBirth RB-338.
- Never kill Rebirth.exe. If it is running, stop and report.

Record the result in docs/rbm.md 12.5.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rbhost's dumped samples for both recovered mods equal the twin mods' audio, or the difference is explained; the result is recorded in docs/rbm.md 12.5
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Recrypt both mods with orb, build songs naming them, run rbhost --mod --dump-samples on the test machine (no Rebirth.exe running), compare slotNN.bin with the twins' AIFF audio, record in docs/rbm.md 12.5.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a919c92c8dfde1b4c (rebased on main a759010).

Ran (scratch task163/): orb mod recrypt --rb20ful RB20FUL.DAT mellow.rbm 'House of love Juam1 .rbm' -o rec (77 and 106 files recovered); rbs.py build with glob.mod_name 'southpark Mod' / 'Template Mod'; for each: tools/rbhost/run.py --mod rec/X.rbm --rb20ful RB20FUL.DAT --dump-samples --remote-dir task163 -o out_X X.rbs (tasklist showed no Rebirth.exe; 1 permit taken and released; rb/task163 deleted with rcs rm -r). rbhost: 'mod 412 loaded', samples written. Compare (rbm.py embedded_file + aiff_samples of twins vs slotNN.bin int16 LE): mellow 44 equal, 0 differ, 2 slots absent in the twin (built-in); House of love 46 equal, 0 differ.

Review round 1 (fixes): docs/rbm.md 12.5 no longer claims this run showed the built-in fallback for mellow's 2 missing slots (21 TR808MC.aif, 27 TR909bd3.aif; not compared, rule cited); [C] qualified; 46/46 is sample slots, not files, and 77 recovered + 27 files of payload <= 1024 bytes (short JPEGs, in the clear) = 104 (counted with scratchpad task163/chk.py: 'embf 104 payload<=1024: 27 rest 77').

Commands and outputs of the run (2026-10-30 14:11-14:12, scratch dir scratchpad/task163, dumps since deleted):
- tasklist on the machine before: rcs_client.py exec tasklist /FI 'IMAGENAME eq Rebirth.exe' -> output 'INFO: No tasks are running which match the specified criteria.' (run.py repeats the check: 'no Rebirth.exe running').
- locky list: redc-win-run permits 6, available 6, no holders. run.py took 1 permit as worktree-agent-a919c92c8dfde1b4c ('acquiring 1 permit of redc-win-run') and printed 'released the permits (ok)' after each run.
- orb mod recrypt --rb20ful RB20FUL.DAT in/mellow.rbm in/hol1.rbm -o rec: 'mellow.rbm: 77 files were stored unscrambled ... recovered (Startup.aif, 12016.jpg, 12057.jpg, ...)'; 'hol1.rbm: 106 files ... recovered (12015.jpg, 12016.jpg, 12017.jpg, ...)'; '2 recrypted, 0 already recrypted (copied), 0 failed'.
- printf '{"glob":{"mod_name":"southpark Mod"}}' > s1.json; printf '{"glob":{"mod_name":"Template Mod"}}' > s2.json; rbs.py build s1.json mellow.rbs; rbs.py build s2.json hol1.rbs (16840 bytes each). (A first try with top-level mod_name failed: 'unknown field .mod_name'.)
- tools/rbhost/run.py --mod rec/mellow.rbm --rb20ful RB20FUL.DAT --dump-samples --remote-dir task163 -o out_mellow mellow.rbs, and the same with hol1: both printed 'rbhost: mod 412 loaded from C:\re\rb\task163\x.rbmmsg (DLL file spec tag 0xf11e54ec)', 'rbhost: samples of mod 412 written to C:\re\rb\task163\x_samples', '75600 frames, sha256 d61d6d6e...1726'. Remote folder removed with rcs_client.py rm -r rb/task163.
- python3 scratchpad/task163/cmp.py (rbm.py embedded_file + aiff_samples of the twin vs slotNN.bin int16 LE) printed: 'mellow equal 44 different 0 twin lacks (built-in) 2 []' and 'hol1 equal 46 different 0 twin lacks (built-in) 0 []'.
- cargo +nightly fmt --all --check: clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The original ReBirth.dll loads both mods recovered by orb mod recrypt; its dumped sample tables equal the twins' AIFF audio slot by slot (mellow 44/44 present slots, House of love 46/46). Recorded in docs/rbm.md 12.5. Docs only, no tests.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-163
title: >-
  Check that the original loads the mods recovered by orb mod recrypt (mellow,
  House of love Juam1)
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-30 11:42'
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
- [ ] #1 rbhost's dumped samples for both recovered mods equal the twin mods' audio, or the difference is explained; the result is recorded in docs/rbm.md 12.5
<!-- AC:END -->

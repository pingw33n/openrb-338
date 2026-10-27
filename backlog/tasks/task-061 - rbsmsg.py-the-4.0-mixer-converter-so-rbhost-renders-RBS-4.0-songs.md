---
id: TASK-061
title: 'rbsmsg.py: the 4.0 mixer converter, so rbhost renders RBS 4.0 songs'
status: To Do
assignee: []
created_date: '2026-10-27 08:38'
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
- [ ] #1 rbsmsg.py converts an RBS 4.0 song's mixer as the exe does (unit tests)
- [ ] #2 RB-SEMx.rbs and the TB-909e mkII song render with rbhost and equal rb render byte for byte
<!-- AC:END -->

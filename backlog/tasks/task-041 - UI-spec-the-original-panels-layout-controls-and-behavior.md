---
id: TASK-041
title: 'UI spec: the original panel''s layout, controls and behavior'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 16:23'
labels:
  - ui
  - re
  - docs
milestone: m-3
dependencies: []
references:
  - docs/song_load.md
  - docs/test_machine.md
priority: high
type: spike
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The UI must reproduce the original panel's layout and behavior (not its art). Write `docs/ui_spec.md`: every section of the panel (transport and global controls, the two TB-303s, TR-808, TR-909, mixer, the four effects, master, pattern/bank selectors, song position and loop displays), each control with its position and size relative to the panel (measured from screenshots of the original, docs/test_machine/ has some; take more on the test machine per doc-001), its value range and detents, its mouse and keyboard behavior (drag direction and sensitivity, fine adjust, click on switches, the 303 step editing modes and keys, the 808/909 step buttons and instrument selector, pattern copy/paste/clear if needed), what it displays, and which engine parameter it drives (the parameter numbers of docs/song_load.md). Sources: the manual (/Volumes/downloads/rebirth_2.01_manual.pdf, or the copy on the CD), screenshots, and Rebirth.exe's panel code in IDA (headless copy, read-only) where behavior is unclear. Scope: what the menu functions below need and the panel itself; from the original menu only load/save song, export, choose RBM and the Vintage flag. Mark each control's behavior [C] (observed or read in the code) or [G].
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/ui_spec.md lists every panel section and control with relative geometry, range, interaction, display and engine parameter
- [ ] #2 The 303 and drum step-editing behaviors and the pattern/bank/song-mode controls are specified completely enough to implement
- [ ] #3 Screenshots used are committed under docs/ui_spec/ and each behavior is marked [C] or [G]
<!-- AC:END -->

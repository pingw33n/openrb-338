---
id: TASK-039
title: 'RE: why the PCF with wave 127 silences the 808'
status: To Do
assignee: []
created_date: '2026-10-26 15:48'
labels:
  - re
  - engine
milestone: m-1
dependencies: []
references:
  - docs/export_state.md
  - tests/songs/diag/README.md
priority: low
type: spike
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The diagnostic songs pcf_lp_wave127 and pcf_bp_wave127 (PCF on the 808 channel, wave 'pattern' 127, frequency 64, resonance 64, amount 100, decay 64) export all zeros in the original app, byte-identical to the silence song, while wave 0 and wave 64 are audible (task-012, docs/export_state.md section 13). Either the PCF's wave 127 setting legitimately silences the channel (a fact the Rust PCF of task-027 must reproduce) or the songs set the PCF wrong (rbs.py or the spec). Find out from the DLL's PCF code (0x100166D0, ctor sub_10016490) and the song's PCF chunk, and record it in docs with addresses; if the songs are wrong, fix the spec so they isolate what the README says.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The cause of the silence is stated in docs with addresses (engine behavior or song construction)
- [ ] #2 If the songs are wrong, the spec is fixed and the songs rebuilt; otherwise the README row says the export is silent and why
<!-- AC:END -->

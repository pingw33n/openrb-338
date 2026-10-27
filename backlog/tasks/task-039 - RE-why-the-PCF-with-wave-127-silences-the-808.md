---
id: TASK-039
title: 'RE: why the PCF with wave 127 silences the 808'
status: To Do
assignee: []
created_date: '2026-10-26 15:48'
updated_date: '2026-10-27 03:04'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Settled by task-027 (branch worktree-agent-ae2be2426a5f128a5), not marked Done here (AC #2's README row is still to write): engine behavior, not song construction. The PCF block 0x100166D0 indexes the pattern data with the raw wave byte (no check; RBS42.txt gives 0..0x37, the exe's load accepts 0..127). Wave 127 reads length word_10033310[127] = 0 and resolution word_10033380[127] = 0 (bytes past the 55 patterns, from the file), so `step = q % len` divides by zero (idiv at 0x10016811) in every block. SafeSoundCallback 0x1001AE40 runs the render in a C++ try/catch(...) (FuncInfo 0x10152AD8, handler 0x1001B055, continuation 0x1001B03D) that catches it and resumes past sub_100139E0's int16 conversion and past the zero fill, so every callback returns with the host's buffer unwritten: the whole export is zeros, not only the 808. Wave 64 is audible because it reads length 16 and resolution 0, with the 16 steps all 0 (no trigger). Facts in docs/initial_investigation.md section 8.2 and docs/export_state.md (the wave-127 paragraph); rb-engine reproduces both wave-127 songs exact (crates/rb-engine/tests/corpus_pcf_comp.rs). What remains for this task: the tests/songs/diag/README.md rows (AC #2: the songs are right, the export is silent because of the fault).

Correction (task-027 review round 1): wave 127 is not the only faulting wave. The pattern length is 0 for 17 waves: 55, 111 and 113..127. Each of them divides by zero at 0x10016811 in every block and silences the export the same way; wave 112 reads a length of 120. 73 waves of 55..127 have a resolution other than 8/16/32. The rb CLI now warns when a render faults.

Supervisor 2026-10-27: the bug itself is recorded in docs/original_bugs.md by task-051; this task's remaining AC #2 (the diag README row) can cite it.
<!-- SECTION:NOTES:END -->

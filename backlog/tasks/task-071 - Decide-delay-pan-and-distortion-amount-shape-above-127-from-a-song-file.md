---
id: TASK-071
title: 'Decide: delay pan and distortion amount/shape above 127 from a song file'
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-27 13:27'
updated_date: '2026-10-27 13:33'
labels:
  - decision
  - engine
milestone: m-3
dependencies: []
references:
  - docs/original_bugs.md
  - crates/rb-engine/src/delay.rs
  - crates/rb-engine/src/dist.rs
priority: low
type: spike
ordinal: 96000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Context: rb clamps the delay's pan (crates/rb-engine/src/delay.rs:74-76, clamp_pan, in slot 11 and in the msg-86 data block) and the distortion's amount and shape (dist.rs:116-118, clamp_param, slot 11 and msg 94) to 127, a deliberate deviation recorded in docs/original_bugs.md OB-9 and OB-10: the DLL stores the byte raw and its update indexes past its tables (delay 0x1000F8B1 into the row after PAN[127]; dist 0x10010932 / 0x100109DC), and rb does not model the adjacent memory. Task-055 found that a song file can carry such a byte: the exe's DELY and DIST parses (sub_43465C called at 0x43589A, sub_4345A6 at 0x435B28 in sub_434D05) copy the chunk bytes unchanged and the packers (sub_422EB7, sub_423048) and the send (sub_438F22) do not check them; a TRAK event cannot (the track load's range is 0..127). So a hand-made .rbs with DELY pan or DIST amount/shape 128..255 renders differently in rb (no shipped or corpus song has one). The panel's messages (delay sub 84, dist sub 92) send 0..127 only.

Options: (a) keep the clamp (rb renders such a file as if the byte were 127), documented as now; (b) model the original: read the adjacent bytes as the DLL does at its preferred base, as rb does for the PCF's waves past the 55 patterns (PCF_DATA, OB-5), after confirming on the test machine (rbhost) with diagnostic songs whose DELY pan / DIST bytes are 128 and 255, and checking whether the bytes past the tables are static .data or run-time tables (.data above 0x10034400 is computed at run time); (c) reject such a file at load.

Recommendation: (a), unless the user wants bit-exactness for crafted files: no real song carries these bytes, and (b) needs run-time table dumps and a corpus capture.
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Decided by @pingw33n on 2026-10-27: option (a), keep the clamp. A song file whose DELY pan or DIST amount/shape byte is 128..255 renders in rb as if the byte were 127, as documented in docs/original_bugs.md OB-9 and OB-10 and docs/song_load.md section 4.10; no real song carries such bytes and the panel sends 0..127 only. No code change.
<!-- SECTION:FINAL_SUMMARY:END -->

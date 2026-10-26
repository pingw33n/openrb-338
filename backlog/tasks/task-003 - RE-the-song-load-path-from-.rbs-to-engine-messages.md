---
id: TASK-003
title: 'RE: the song-load path from .rbs to engine messages'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
labels:
  - re
  - docs
milestone: m-0
dependencies: []
references:
  - docs/RBS42.txt
  - docs/initial_investigation.md
priority: high
type: spike
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rebirth.exe (the UI) parses the .rbs file and turns it into messages to the engine in ReBirth.dll through the shared-memory port: packets `[78, dev, msg, payload]` built in sub_423C17 and sent with sub_40B1E3 / sub_40B194, received by the DLL dispatcher 0x1001CDF0 and the device handler 0x10011ED0 (see docs/initial_investigation.md sections 5.2 and 10). This conversion was not traced in the survey. Both the Rust engine (m-1) and the offline DLL host (rbhost) need it exactly.

Trace the song-load path in Rebirth.exe (ida/Rebirth.exe.i64; open a copy headless, read-only, per doc-001) from the .rbs reader (IFF CAT RB40, docs/RBS42.txt) to the messages it sends, and document every mapping in `docs/song_load.md`:
- the order of messages at song load (msg 33 reset, msg 36 globals, msg 41 device/pattern data, msg 37 track events, msg 35 parameters, anything else) with payload layouts;
- the track position scaling (RBS delta units, 24 PPQ, to the engine's 192 PPQ; expected x8) and the loop start/end conversion (GLOB Bar*768);
- the TB-303 step conversion (RBS flags bit0 slide, bit1 accent, bit2 up, bit3 down, bit4 note) to the engine's 6-byte step `{note, slide, accent, up, down, rest}`; note the copy-protection scrambler in sub_420FB7 and under what condition it fires;
- the TR-909 step value mapping (RBS 0..3 to the engine's levels 1..4 or otherwise) and TR-808 accent bytes;
- the delay step mode polarity (the engine code makes mode 0 = 1/3 beat, RBS42.txt says 0 = quarters), PCF and compressor routing values, the "Vintage" GLOB byte 482 to the engine's global flag +21 (2.0 sound when non-zero);
- which values decide the engine's state at an export: the pattern-mode device chunks (MIXR, 303, ...) or the song-mode track events chased at locate;
- the mixer flag +0x16D (host msg 21 case at 0x10012696): who sets it and its value.
Verify each mapping in the disassembly, not only Hex-Rays.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/song_load.md lists every message the exe sends at song load, in order, with its payload layout and the exe/DLL addresses that build and read it
- [ ] #2 Each RBS chunk field and TRAK controller is mapped to its engine message and value, including the position scaling, 303/808/909 step encodings, delay mode, routing and the vintage flag
- [ ] #3 The scrambler in sub_420FB7 is described: its trigger condition and whether it can affect a normal song load
- [ ] #4 docs/initial_investigation.md section 10 item 1 is updated to point to the new document and its [G] items resolved or marked
<!-- AC:END -->

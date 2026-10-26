---
id: TASK-003
title: 'RE: the song-load path from .rbs to engine messages'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 10:11'
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
- [x] #1 docs/song_load.md lists every message the exe sends at song load, in order, with its payload layout and the exe/DLL addresses that build and read it
- [x] #2 Each RBS chunk field and TRAK controller is mapped to its engine message and value, including the position scaling, 303/808/909 step encodings, delay mode, routing and the vintage flag
- [x] #3 The scrambler in sub_420FB7 is described: its trigger condition and whether it can affect a normal song load
- [x] #4 docs/initial_investigation.md section 10 item 1 is updated to point to the new document and its [G] items resolved or marked
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Trace the RBS reader sub_434D05 (called from sub_425C4D / sub_4269CE) chunk by chunk: parse helpers, the exe's intermediate structs, the packers (sub_420AE7, sub_4236AD, sub_4213E7, sub_422063, sub_422CE4 ...) and the senders sub_40B1E3 / sub_40B28C / sub_40B142.
2. Trace the receiving side in the DLL: dispatcher 0x1001CDF0, song-device handler 0x10011ED0, the per-device host-message handlers (vtable slot 10) and the setParam / chase code, to confirm each payload layout and value meaning.
3. Check mappings against the shipped songs in tests/songs/shipped (scratch Python parsers, not committed).
4. Analyse the copy-protection index obfuscation (sub_420FB7 and siblings) and its trigger.
5. Write docs/song_load.md; update docs/initial_investigation.md (section 10 items 1-2, the [G] items on 303 rest flag, 909 levels, delay mode, vintage flag, msg 33 at load) and RBS42 errata in song_load.md. Docs only, no code.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a570c30a7ddf28463

Evidence, as the work went (exe = Rebirth.exe, DLL = ReBirth.dll; IDA copies opened headless in the scratchpad, closed without saving):
- RBS reader: sub_434D05, called by sub_425C4D (File > Open, via sub_40B575) and sub_4269CE (benchmark song). Chunk names at 0x4CF1B8..0x4CF268. File version = HEAD bytes 5..7 (CVersion, sub_47DB95; HEAD[4] must be 0xBC); version objects 4.0 = 0x4DB938, 4.1 = 0x4DB928, 4.2 = 0x4DB918 (sub_43295F/4329A2/4329E5).
- Senders: sub_40B1E3 [78,song,msg,payload] (sub_423C17), sub_40B28C [78,song,41,dev,sub,payload] (sub_423C98), sub_40B194 [78,song,msg] (sub_423BA0), sub_40B142 [type,payload] (sub_423B76). All multi-byte values big-endian (sub_465FFF / sub_465F5E).
- Load order: msg 52 {BE16 mod} (sub_425A5D, reply 'A'+song id), optional top-level 58 (sub_426C67 -> sub_40ABB5), then in sub_434D05: 36 (sub_438B4B, 23 bytes), 41 mixer 0/143 (sub_438D43, 29 B), delay 5/86 (sub_438F22, 6 B), PCF 7/116 (sub_439016, 8 B), dist 6/94 (sub_4390F8, 4 B), comp 8/104 (sub_4391B6, 5 B), 303 1/39 and 2/39 (sub_4395B8, 3148 B), 808 3/56 (sub_439DCC, 6240 B), 909 4/73 (sub_43A637, 6241 B), then msg 37 per TRAK 0..8 (sub_436B6F; sub_436F5A for the 808 track of files < 4.2; sub_437380 for the mixer track of 4.0 files), then msg 32 (0x426610). No msg 33 is sent at load: the survey's claim is refuted.
- TRAK position: disasm 0x436CF3 imul edx,0C0h; shr edx,3 -> engine tick = rbs*24 (not x8). Checked on the 18 shipped songs: last event / 32 matches the loop end in bars (TGV 544 vs 547, Groove 233 vs 235, Fast Forward 248 vs 250); /96 does not. RBS position unit = a 32nd note.
- 303 steps: sub_434309 turns the RBS flags into {note, b0, b1, b2, b3, b4}, byte 5 = bit4. Silent Default Song has every step 0x10 -> bit4 = rest. DLL note-on 0x10023E04 tests step[5]==0 for play.
- Vintage: sub_433F96 copies GLOB[482] raw (0 for files < 4.2); DLL +0x16D (= global struct 0x158 + 21) tested at 0x10023F2D: zero -> tail call 0x100248C4. Menu 'Vintage ReBirth Synth Sound' (cmd 323, sub_42C204 0x42C32E) toggles the byte and sends msg 35 {21,v}; the check mark shows the byte (0x42BF09). So 0x100248C4 is the 2.0 sound, the inline code of 0x10023F10 is Vintage: the survey's labels are swapped.
- Delay mode: 0x1000F7E5 [+0x36]!=0 -> 4T/100 (16ths) else 3T/100 (1/3 beat); exe passes DELY[2] raw. PCF id: DLL sub_10012C14 0x10012E41.. puts PCF on channel id-1 for ids 1..4; comp ids 2..5 on channels, 1 = master. Shipped TRAK mixer ctrl 2 values 0..4.
- 909: exe copies steps raw (sub_43375D); DLL 0x1002AAC0: 1/3 -> x0.8, 2/4 -> x2.0, >2 flam, AC column value scales accent (base = AC*acc*0.03125+1). 909 parse sub_43348D copies RBS[13] into slot 16 (MidTomDecay <- LowTomDecay), verified at 0x433558.
- Protection: index expressions in sub_420AE7, sub_420FB7, sub_421296, sub_420D10, sub_4236AD, sub_423427, sub_423597, sub_423C98, sub_423BA0, sub_422EB7, sub_4232A3, sub_423048, sub_42314F, sub_4215E5, sub_421ABD, sub_4228CA, sub_42272B, sub_422CE4, sub_423A37 use .bss words filled by sub_4065F2 with 2048-byte pages of Rb20ful.dat chosen by PID, parent HWND and HINSTANCE; strings decoded with the s[j]-=s[j+1] scheme (kernel32.dll, CreateFileA, \\.\SICE, IsDebuggerPresent, Rb20ful.dat, Rb20upd.dat, 'ReBirth Win', RB338V20).
- Failed hypotheses: x8 track scaling; rest = !bit4; GLOB[482] inverted on the way to +21; RBS 909 0..3 shifted to 1..4; msg 33 at load.

Acceptance criteria evidence:
- AC1: docs/song_load.md §2 lists msg 52, optional 58, 36, nine msg 41 blocks, msg 37 per track, msg 32 in send order, with builder (exe) and reader (DLL) addresses; §1 has the packet forms and the DLL dispatch tables (0x10153164, 0x10151D6C, per-device handlers); §3-§5 the payload layouts.
- AC2: §3 (GLOB -> msg 36 byte by byte), §4.1-4.8 (every DEVL chunk byte -> msg 41 byte, 303/808/909 step encodings, delay mode, PCF/comp routing), §5 (TRAK position x24, controller table with ranges per track), §7 vintage/+0x16D, §10 RBS42 errata. Checked on the 18 shipped songs (scratch scripts rbs.py, trak.py, s303.py, sdrum.py in the scratchpad; not committed).
- AC3: §9 describes the index obfuscation in sub_420FB7 and 18 other packers, the Rb20ful.dat pages behind it and the trigger (pages missing or wrong, which a normal load cannot reach because start-up aborts at 0x40EA7A).
- AC4: docs/initial_investigation.md §10 items 1 and 2 now point to docs/song_load.md; [G] items resolved: 303 rest flag (§6.1), +21 vs GLOB[482] (§6.3, plus the swapped 2.0/vintage labels in §2, §6.1-§6.3), 909 levels (§7.2), delay mode (§8.4); §5.2 packet table corrected; §9 'msg 33 on song load' refuted.
Follow-up task created: task-034 (engine side of song creation, song-mode defaults, 4.0 mixer track, global bytes 2 and 22).

Review round 1 (send back, small fixes):
1. song_load.md section 3: 'byte 0/1' for the sound model now reads 'value 0/1'.
2. msg 35 id 2 setter: the msg-35 case at 0x100125A7 calls sub_10021190, a thunk to 0x10021194; song_load.md section 3 and task-034 now both say so.
3. song_load.md section 2 now gives the dispatch evidence: song-handler table 0x10151D6C maps msg 21 -> 0x100122FC (query, packs sub_10018B00, replies msg 36, writes nothing), msg 20 -> 0x10011FBD (query), msg 35 -> 0x10012555 with id table 0x10151E14, id 21 -> 0x10012690 (store at 0x10012696). The survey's 'msg 21 case' is msg 35 id 21.
4. song_load.md section 2 states no msg 35/20/21 is sent at load, listing the senders reached from sub_425C4D / sub_434D05; sub_42DA72 (song window, built after the load) is marked [G], not traced.
5. 'nothing is reset in an existing one' replaced by a [G] statement pointing to task-034 item 1.
6. Shipped-song checks: the scratch scripts rbs.py (IFF chunk parser, GLOB/MIXR survey), trak.py and trak2.py (TRAK decoder, controller ranges, positions vs loop end), s303.py (303 flag survey), sdrum.py (808/909 step values), dec.py/dec3.py (string decoders) and pe.py/jt*.py (PE and jump-table readers) were in /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/worktree-agent-a570c30a7ddf28463/ and are not kept in the repository. To reproduce with any RBS parser over tests/songs/shipped/*.rbs: version = HEAD[5..7]; vintage = GLOB[482]; 303 flags at 9+34p+3+2s; 808 steps at 32+194p+12s+i, 909 at 33+194p+12s+i; TRAK: sum the varint deltas, compare max/32 with GLOB loop end/768.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Traced the .rbs load path in Rebirth.exe (sub_425C4D -> sub_434D05) to the engine messages and their DLL readers; documented in docs/song_load.md. Branch worktree-agent-a570c30a7ddf28463, one commit (docs only).

Main results:
- Load sends top-level 52 (new song), then song msgs 36, 41 x9 (mixer, delay, PCF, dist, comp, 303 x2, 808, 909), 37 per track, 32. No msg 33 at load (survey claim refuted).
- TRAK ticks = RBS position x 24 (unit = 32nd note), not x8; loop points are already ticks.
- 303 rest = flag bit4; 909 step values pass raw (no 1..4 shift); 808 steps 0/1; 909 MidTomDecay chunk byte is taken from LowTomDecay (exe bug, 0x433558).
- Delay mode 0 = triplets, 1 = 16ths; PCF id 1..4 = channels; comp 1 = master, 2..5 = channels.
- +21 / +0x16D = GLOB[482] unchanged; 0 selects 0x100248C4 and the plain send. The survey's 2.0/vintage labels for 0x10023F10 / 0x100248C4 were swapped; corrected in docs/initial_investigation.md. Every shipped song uses 0x100248C4.
- Song mode: TRAK chase decides all automatable values (device chunks are not copied to live in song mode); pattern mode: chunks decide.
- The protection scrambler only fires without the genuine Rb20ful.dat pages; a normal load always has them.
- RBS42.txt errata listed (§10).

Testing: none (docs-only); mappings checked against the 18 shipped songs with scratch scripts.
Follow-up: task-034. Supervisor notes: task-023's description uses the old swapped engine labels; task-030's key file (app field this[325]) is the Rb20ful.dat opened by sub_406A79.
<!-- SECTION:FINAL_SUMMARY:END -->

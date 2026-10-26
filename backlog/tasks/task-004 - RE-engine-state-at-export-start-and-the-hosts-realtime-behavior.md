---
id: TASK-004
title: 'RE: engine state at export start and the host''s realtime behavior'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 10:36'
labels:
  - re
  - docs
milestone: m-0
dependencies: []
references:
  - docs/initial_investigation.md
priority: high
type: spike
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The export does not reset the engine (docs/initial_investigation.md section 9): the 303 oscillator and filter states, the leftover sub-block buffer in sub_100139E0, the mixer's 16-block counter, the smoothed gains, the delay buffer and write pointer, the compressor/PCF states and the global snare/bass-drum counters carry over from whatever the app did before. To make the original's output deterministic we need the complete list of that state and of every event that changes it on the host side.

Trace in Rebirth.exe and ReBirth.dll (copies of ida/*.i64, headless, read-only) and document in `docs/export_state.md`:
- how the exe drives the engine in realtime (the DirectSound/MME audio thread, SyncSoundCallback vs SafeSoundCallback, buffer sizes) and whether the engine is called while stopped (does the 303 keep running when idle);
- what the exe does around an export (sub_427988: the realtime audio object's vfunc +8/+4 around the render), whether a realtime callback can run during the export, and what is in the leftover buffer when the export starts;
- every place msg 33 (full reset, sub_10014140) is sent and what it resets; what song load, New, Open and app start do to engine state;
- the full list of engine state that survives into an export, each with its address, its reset points (if any) and its effect on the output;
- the startup sound (preview player at +1064 in SafeSoundCallback) and anything else mixed into the output.
End with a proposed protocol (a sequence of user or host actions) that should make an export start from a known state, and the residual state it cannot control.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/export_state.md lists every piece of engine state that survives into an export, with addresses, reset points and output effect
- [x] #2 The exe's realtime driving of the engine while playing, stopped and exporting is described with addresses
- [x] #3 The document ends with a proposed deterministic capture protocol and the state it leaves uncontrolled
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Static analysis only (headless copies of ida/*.i64, read-only, closed with save=false); no test-machine run: task-010 does the experiments.
2. Trace the exe realtime path: driver objects (MME_CHANNEL, CDirectSoundChannel), callbacks sub_40C7F0/sub_40C834, buffer sizes, the reply-wait pump sub_40AC44/sub_408DF5, the export driver sub_427988 and the benchmark sub_401EE0.
3. Trace the DLL side: SafeSoundCallback, the time-budgeted drain sub_1001CC40, sub_100139E0 leftover logic, top-level 52/53/54/57/58 (new song, activate, delete, preview), msg 33 sub_10014140 and every device reset slot, stop, the song ctor sub_10011350.
4. Write docs/export_state.md: realtime driving, export sequence, reset points, the full list of surviving state with addresses, reset points and output effect, the preview player, the shared-memory port names (for rbhost), a proposed protocol and the residual.
5. Correct docs/initial_investigation.md sections 3, 9, 10 and docs/song_load.md sections 2 and 11 where this confirms or refutes [G] items.
6. One commit, task To Do -> Done.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a9823411345b7e5e3

Evidence (static, headless IDA copies t4exe/t4dll in the scratchpad, read-only, closed save=false):
- Exe realtime: channel at app+0x254C made by sub_40C863 (card list dword_4E05AC, card slot 0 with (44100, sub_40C7F0, sub_40C834, 0, 2) at 0x40C954). MME_CHANNEL vtable 0x4BE764, ctor sub_4A5C17: 1024 frames per buffer (sub_4A2870 arg at 0x4A5C5D), 16 WAVEHDRs, buffers in flight word_4BE728[idx] = 2..16, play-ahead sub_4A5539 = 1024000*count/44100 ms, so 371 ms = 16 buffers (index 14). Refill sub_4A5B68 -> sub_4A5A55 calls cb1 then cb2 with (buf,1024,0). Worker period 1024000/(3*44100) = 7 ms (0x4A6003). DirectSound refill sub_4A3F79/sub_4A4040: multiples of 512 frames (0x4A4028), ring 16384 (0x4A4030).
- sub_40C7F0 = Sync then Safe unless app byte +0x500; sub_40C834 = Safe if +0x500: one Safe per buffer.
- SafeSoundCallback 0x1001AE40: try-lock engine mutex (WaitForSingleObject 0 ms, 0x10009454), drain sub_1001CC40 (budget (n*1000/44100)/3 ms, >= 1 message, compare at 0x1001CDB1), render transport+0x16C via sub_100139E0, preview mix 0x1001AF15..0x1001B037 (h = s/2 toward zero, add/adc/sar at 0x1001AF7E).
- Block 0x10013444 has no playing test: devices run every block while stopped.
- Reply wait sub_40AC44 pumps Safe(..,0,0) only without a channel (sub_408DF5); export calls channel slot 2 at 0x427BD9 with no null test, so no channel = crash.
- Export sub_427988: stop 0x427A02, queries 0x427A35/0x427ACB/0x427B5E (reply waits), channel stop 0x427BD9, locate 0x427C1D, play 0x427C83, render sub_4280D5/sub_427EA7, stop 0x427DA6, msg 33 0x427DC9, Safe(NULL,0,0) 0x427DD6, channel start 0x427DE7. Benchmark sub_401EE0 same pattern.
- Leftover (sub_100139E0): realtime calls multiples of 64 never create one; final Safe(NULL,0,0) drops it (L becomes -(64-L), store 0x10013BBA). Survey claim "up to 63 samples before the locate" refuted.
- Top-level dispatch table 0x10153164 (by type), queued types via sub_1001D694 to EngineIdle (table 0x101531F8): 52 -> sub_1001D840 new song (operator new 0x1910 + ctor sub_10011350, new devices), 54 -> sub_1001DC64 delete, 57 -> sub_1001DB60, 58 -> sub_1001DC14 preview start (mod record +184). 53 inline -> sub_1001BDA4 activate (old song devices slot 6 via sub_10013D40).
- Msg 33 sub_10014140: master current +0x180 = 0, meters, buffers +0x160C = 1e-6, leftover blocks 0, device slot 5. Slot 5/6 table: 303 0x10026CC0/0x10025630; 808 0x1000F4F0->0x1002A2D0; 909 ->0x1002DD40; delay ->0x100100E0 (global buffer 0x10054794 = 1e-6); dist/PCF/comp/mixer nullsub 0x10012C00. Drum voice slot 1 = 0x1001EB30 (active byte, +0x18).
- Stop sub_1001E100 -> sub_100218A0 -> sub_10021840: slot 4 (= slot 6 except delay nullsub).
- 303 2.0 audio 0x100248C4 stores phase +0xCA4 (+= table[CB4>>4] per sample, -2 when > 1.0: consts 0x10153C34 = 1.0, 0x10153C40 = 2.0). Note-on 0x10023CD0 does not reset phase. Pitch table 0x10125364 built at 0x10023B4D: table[0] = 110*2^(1/4)/16/44100 = 1.8538e-4 (consts 0x10153A80 = 2.0, A90 = 1.0, A98 = 55.0, AB0 = 1/44100).
- Globals: 808 snare word_1014CBD4 (0x1002A27D), 909 snare word_1014E548 (0x1002DCED), 808 BD toggle word_1014CBD0 (0x1002A106/0x1002A12A), 909 BD toggle word_1014E544 (0x1002DB82/0x1002DC3B). dword_10124708 written only (0x10023EDD), never read.
- Delay ctor sub_1000F5A0: write ptr +0x6C = 0, counter +0x30 = 0 (CEffect base sub_10010420).
- Port: sub_1001B980 -> sub_100179E0: mappings "ReBirth DLL->Panel"/"ReBirth Panel->DLL", 0x7D10 bytes, "<name>_SEMA", "<name>_EVENT".
- Startup sound: sub_426C67 sends 58 when mod record word +1088 == 1 and app byte +0x533 (prefs +536, sub_4098AA) is set.
Failed hypotheses: (1) "no audio device" protocol (no realtime, N = 0) is impossible in the app: export dereferences the channel at 0x427BD9. (2) Repeated exports in one process as a reset: msg 33 resets the 303 but the realtime restarts at once, so N is again unknown, and master/global counters differ.
No test-machine run (static task; task-010 measures).

AC evidence:
#1 docs/export_state.md section 7 (tables 7.1 to 7.5): every surviving state with address, init/reset points (section 6), class (fixed/settles/free/global) and output effect; section 5 the leftover buffer.
#2 docs/export_state.md section 2 (MME/DirectSound channels, callbacks, SafeSoundCallback, drain budget, what runs while stopped, pumping without audio, EngineIdle) and section 4 (export sequence with addresses); playing vs stopped in 2.4, exporting in 4.
#3 docs/export_state.md section 9: protocol steps 1-6 and the uncontrolled residual (N idle blocks: 303 phase and delay pointer, fitted; first-chunk two-message drain; heap; unproven settling).
Also corrected: docs/initial_investigation.md sections 1, 3, 8.4, 9, 10 and docs/song_load.md section 2 steps 3-4 and section 11 ([G] new-device-objects and msg 58 now [C]).

Review round 1 (send back, documentation only), fixed in the amended commit:
1. Addresses added: 303 amp/envelope/filter rows now list +0xCA8, +0xCD4, +0xCBC, +0xCC0/+0xCC4, +0xCDC..+0xCE8 and the other reset fields (reset stores 0x10026CE6..0x10026D51), with [G] for which is the accent env / cutoff smoother; drum voice fields from FILTER_VOICE 0x1001E4D0: +8 active, +0xC ptr, +0x10 left, +0x14 hold, +0x18 gain g (so voice reset 0x1001EB30 clears the gain, not a position: corrected), +0x1C m, +0x20 c, +0x24 y; distortion +0x44/+0x48/+0x54/+0x58 (0x10010A50), compressor +0x50/+0x5C/+0x64/+0x66 (0x1000EC70), PCF +0x50..+0x84 (0x100166D0); constructors not read marked [G].
2. [G] added to the settling claims (summary, delay buffer, channel/master/delay gains, 303 coefficients, investigation section 9 table).
3. Store addresses cited: phase +0xCA4 ctor 0x10023A6B, reset 0x10026D40; pitch +0xCB0/+0xCB4 ctor 0x100239ED/0x100239F3, reset 0x10026CD4/0x10026CDA; coefficients ctor 0x10023A2F..0x10023A41 (checked in the disassembly).
4. Leftover sentence fixed: L <= 0 fails the test at 0x10013AE0, rem = n, so the next call stores its own rem (0 for multiples of 64 and for n = 0).
5. Startup gate: sub_4098C3 tests app byte +0x533 (call 0x426C85), sub_4098AA writes it (0x4098B7) from prefs +536 (0x40E2F8); export_state.md now says both, consistent with song_load.md.
6. Worker period written as the unsigned integer division 0xFA000/(3*44100) = 7 (exact 7.74).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Traced in the exe and the DLL (static, headless IDA, read-only) how the app drives the engine and what state reaches an export; written to the new docs/export_state.md.

Findings: the app keeps a realtime stream open (MME: 1024-frame buffers, 16 in flight = the 371 ms play-ahead; DirectSound: multiples of 512) and renders the active song every buffer, stopped or playing, with all devices running. The export stops the channel, applies locate+play in the first chunk drain, renders, then sends stop, msg 33, a 0-frame SafeSoundCallback and restarts the channel. The leftover buffer is always empty at export start (survey claim refuted). Every song load creates a new song with new device objects (msg 52 -> 0x1001D840). Msg 33 resets the 303s, drum voices, delay buffer and sets master gain to 0, nothing else. With MME the mixer/effect update phases are fixed. The remaining dependence is on N, the idle blocks since activation: the 303 oscillator phase (+0xCA4) is free-running (table[0] = 1.8538e-4/sample at idle) and the delay write pointer advances; other states settle. Process globals: snare noise counters and BD toggles (four words at 0x1014CBD0..0x1014E548). The startup sound is the preview player (top-level 58) mixed into every SafeSoundCallback. Exporting without an audio channel is impossible (null deref at 0x427BD9).

Protocol (section 9): restored prefs with MME, startup sound off, fresh process, open the song, wait a fixed T, export, quit; fit N (a multiple of 16) with rbhost/Rust.

Docs: docs/export_state.md (new); docs/initial_investigation.md sections 1, 3, 8.4, 9, 10 and docs/song_load.md sections 2, 11 corrected. Branch worktree-agent-a9823411345b7e5e3, one commit. Tests: none (docs only).

Follow-ups for the supervisor: task-011 (rbhost) should take an idle-block count N option and use the port names of section 2.7; task-010 should test the protocol and the startup-sound preference; task-034 item 1 (new device objects) is now answered, the constructor defaults remain.
<!-- SECTION:FINAL_SUMMARY:END -->

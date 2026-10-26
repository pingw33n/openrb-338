---
id: TASK-011
title: 'Tool: rbhost, an offline host for the original ReBirth.dll'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 13:03'
labels:
  - tools
  - re
  - testmachine
milestone: m-0
dependencies:
  - TASK-003
  - TASK-004
  - TASK-002
priority: high
type: feature
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An exported WAV from the GUI depends on uncontrolled state (docs/initial_investigation.md section 9). A small Win32 program that loads the original ReBirth.dll itself and drives it offline removes that: a fresh process, a known block alignment, no realtime thread. It also gives us the engine's run-time tables (all computed at init into BSS) and its state for the Rust port.

Write `rbhost` (C, i686 Windows, cross-built on the Mac with llvm-mingw: see ~/dev/dcre/common/tools/fetch_llvm_mingw.sh; sources under tools/rbhost/): it loads ReBirth.dll, emulates whatever the panel side of the shared-memory port needs (EngineOpen, the "ReBirth Panel->DLL"/"DLL->Panel" mappings, semaphores and events; docs/export_state.md), sends the song-load messages of docs/song_load.md for a given .rbs, then the export sequence (stop, locate to loop start, play, the 2048-frame SafeSoundCallback loop with the exe's double-precision chunking, stop) and writes a WAV. Options: `--dump-tables DIR` writes the BSS tables listed in docs/initial_investigation.md section 10 item 4 (with their addresses) after init; `--dump-state` writes the engine objects at export start. It must not need the GUI or a registration; if the DLL refuses to run without the panel, document why and what the minimal emulation is.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rbhost renders a given .rbs to WAV on the test machine without Rebirth.exe running, from a script that builds it on the Mac, uploads and runs it
- [x] #2 Two renders of the same song by rbhost are byte-identical
- [x] #3 --dump-tables writes every table of section 10 item 4 with a manifest of addresses and sizes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE (headless copies of both IDBs): the DLL's exports and their calling conventions, DllMain and EngineOpen requirements, the shared-memory ring layout (port slots 1/2, sub_10017D40/sub_10017F50), the semaphore/event objects, the exe's start-up handshake (msg 50), the mod registration (msg 56 for "Standard ReBirth", id 411), the top-level and idle-queue dispatch tables, the builders and addresses of the section 10 item 4 tables, the device objects to dump.
2. tools/rbhost/rbsmsg.py: .rbs -> the song-load messages of docs/song_load.md (msg 36, the nine msg 41, msg 37 batches with the exe's validation, msg 32) in a small binary file, using tools/rbs.py; unit tests.
3. tools/rbhost/rbhost.c (C, i686, llvm-mingw): LoadLibrary ReBirth.dll, the x87 control word the app runs with, EngineOpen, the panel side of the two named rings, handshake (50), mod (56), new song (52), optional startup sound (58), the song messages, activate (53), --idle-blocks N as 1024-frame SafeSoundCallback calls, the export sequence of docs/export_state.md section 4 (stop, --post-stop-blocks, locate, play, the double-precision 2048-frame chunk loop, stop, 33, the 0-frame call), WAV output. Options --dump-tables DIR (with manifest), --dump-state DIR, --drum-counters.
4. tools/rbhost/build.sh and tools/rbhost/run.py: build on the Mac, convert, upload to C:\re\rb\rbhost, run under one permit of redc-win-run, download.
5. Validate on the test machine: two renders byte-identical, the table dump, and a k fit for a 303 song against a recorded app export (task-010 scripts) if possible.
6. docs: the facts (ring layout, handshake, mod registration, table addresses, FPU control word) into docs/export_state.md and initial_investigation.md section 10 item 4; tools/rbhost/README.md; a pointer in docs/test_machine.md.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-004 (2026-10-26, docs/export_state.md): use the shared-memory port names and protocol of section 2.7 and the export sequence of section 4. Besides the plain offline render, rbhost needs an idle-block option (--idle-blocks N, N a multiple of 16) that renders N 64-frame blocks of the active, stopped song before the export, as the app's realtime stream does while stopped (sections 2.4, 7, 9): the 303 oscillator phase and the delay write pointer depend on N, so the app's exports can be reproduced by fitting N. Also an option to preset the four process-global drum counters (section 7.5) and to mix the startup sound through the preview player (section 8) for the comparison with app exports.

Branch: worktree-agent-a7be7bcd4d2048bf3

RE for rbhost (headless copies of ida/ReBirth.dll.i64 and ida/Rebirth.exe.i64, 2026-10-26):
- DLL exports (cdecl): EngineOpen() 0x1001B0E0 (no args; new engine 0x430 -> [dword_1011D0B0+0x370], returns 1), EngineIdle() 0x1001B070, SafeSoundCallback(int16 *buf, int n, int x) 0x1001AE40, SyncSoundCallback 0x1001ADB0, EngineClose 0x1001B1A0. The exe calls them cdecl with add esp,0Ch (sub_408DF5). dword_1011D0B0 (the 0x378 global object) is built in DllMain (0x1000DAD0 -> sub_1001E190) at process attach.
- Exe start-up (sub_40FAA9 -> sub_409A56): LoadLibrary, GetProcAddress of the 11 exports, EngineOpen(); then the panel port (CReBirthPanelPort, sub_4285E0) is opened on the DLL's mappings and the exe sends top-level 50 (1 byte), pumping with sub_408DF5 (SafeSoundCallback(buf,0,0)) until the reply '@' (64) with 6 bytes. On shutdown it sends 51 (sub_40D479, 0x40D79A).
- Top-level dispatch sub_1001CDF0: jmp [0x10153164 + 4*type] for type 5..78 (IDA's switch labels are off). 50 -> 0x1001CE70: sets engine byte +0x374 = 1 and replies '@' + 5 bytes of CVersion (sub_1000E030); 51 -> clears +0x374; 52,54,55,57,58,63 -> queued (sub_1001D694); 53 -> activate (sub_1001D7F4); 56 -> queued only when +0x374 is set, else error reply sub_1001DE00(18); 59 -> status 'L'; 76/77 settings; 78 song. Idle queue sub_1001D6D4: jmp [0x101531F8 + 4*type]: 52 sub_1001D840, 54 sub_1001DC64, 55 sub_1001D990 (mod bank from the panel's struct), 56 sub_1001DA70 (built-in bank), 57 sub_1001DB60, 58 sub_1001DC14, 63 reply 'H', 78 song msg via sub_10012940.
- Mods: the exe's built-in mod is COpenMod "Standard ReBirth" with id 411 (sub_417AE9 stores word 411 at +1684; sub_4166D0 reads it); sub_41693C sends 56 {u16 BE id} for it (55 with a 0x2546-byte sample table for a file mod) and waits for the reply. DLL sub_1001DA70 -> sub_1001A030 appends a mod record to the list dword_10118280 (looked up by id in sub_1001A860, used by the song ctor): 24 808 samples (resources 1000..1023), 21 909 samples (1025..1045), startup sample (1024), reply 'C'. Msg 52 {u16 id} gives the song that record (song+0x174); without msg 56 the lookup returns 0.
- The port ring (sub_100179E0, CReBirthDLLPort sub_1001B980, vtable 0x10033C44: slot 1 read 0x1001BB10 -> sub_10017D40, slot 2 write 0x1001BB50 -> sub_10017F50): mapping of 0x7D10 bytes, header dword 0 = bytes used, dword 1 = bytes free (32000 at creation), dword 2 = read offset, dword 3 = write offset, data at +16, ring of 32000 (0x7D00) bytes; a message is a little-endian u16 length then the bytes, each byte at (offset+16), offset = (offset+1) % 32000. Writer: lock, while len+2 > free {unlock; WaitForSingleObject(event, 4000) (timeout throws); lock}; write; used += len+2; free -= len+2; unlock. Reader: lock; if used > 0: read length, if > buffer size return 0 without consuming, else copy, used -= len+2, free += len+2, SetEvent(event); unlock. The lock "<name>_SEMA" is CreateSemaphoreA(0, 1, 1, name) (sub_100091B0), taken with WaitForSingleObject(h, 10000) (sub_100092F0, timeout throws) and released with ReleaseSemaphore. The event is the auto-reset "<name>_EVENT". The DLL writes "ReBirth DLL->Panel" and reads "ReBirth Panel->DLL"; the drain reads into byte_1011A9A0 with a 10000-byte limit.
- Activation sub_1001BDA4 (msg 53): with no current song it calls slot 6 (sub_10013D40) on the NEW song, so the first activation also clears the global delay buffer to 1e-6; rbhost needs no Untitled song for that.
- x87 precision: the exe's CRT start-up _fpmath 0x4A7540 -> _setdefaultprecision 0x4AA860 calls _controlfp(0x10000, 0x30000), i.e. 53-bit precision (control word 0x027F); new Win32 threads (the MME worker) start with 0x027F too. The DLL never sets the control word (no _controlfp import). rbhost sets 0x027F before loading the DLL.
- Tables (section 10 item 4), builders and guards: T f32[128] 0x10054550 and PAN f32[128][128][2] 0x10034550 (sub_100046B0, from the song ctor sub_10011350 once, byte_100E0128); 303 pitch f32[6144] 0x10125364 (303 ctor sub_10023920, once, byte_10124704); 303 FILT f32[8][1024][4] 0x1012B364 (sub_100234A0, every 303 ctor); PCF LP f32[8][512][3] 0x1010C270 (sub_100172C4) and BP 0x10100270 (sub_10017574), once (byte_1010026C) from the PCF ctor sub_10016490; Dist W f32[32][256] 0x100D47A4 (sub_10011210, once, byte_100D47A0; the 0x10011210 of investigation 8.1 is the builder, not the table); T808 u16[128] 0x1014CBD8 (808 ctor sub_10026F70, first ctor, word_1014CCD8); T909 u16[128] 0x1014E54C and DT f32[16] 0x1014E64C (909 ctor sub_1002A380, first ctor, word_1014E68C). All are built by the first new song (msg 52).
- Device objects of a song (ctor sub_10011350): mixer 0xA0 at song+0x170, 303 x2 0xD04, 808 0x1910 (voices at dev+4*1591..1602: 5x0x20, 3x0x30, 3x0x28, 1x0x2C), 909 0x1920 (voices dev+4*1592..1603: 4x0x20, 5x0x30, 2x0x28, 1x0x2C), delay 0x74, dist 0x60, PCF 0x8C, comp 0x6C; the device list song+0x18C, count byte song+0x1C4.

Toolchain: llvm-mingw 20260922 (ucrt, macOS universal) was already installed in ~/llvm-mingw by ~/dev/dcre/common/tools/fetch_llvm_mingw.sh (~/llvm-mingw/VERSION = 20260922); tools/rbhost/build.sh uses $LLVM_MINGW (default ~/llvm-mingw)/bin/i686-w64-mingw32-clang -O2 -static -msse2 -mfpmath=sse. Nothing installed in the repo.

Design: the .rbs -> message conversion is Python (tools/rbhost/rbsmsg.py, on top of tools/rbs.py, unit-tested on the Mac), rbhost.exe replays a small binary message file (.rbhm). run.py does build + convert + upload + run + download.

Export chunk loop: checked against sub_4280D5 disassembly: n = ftol((fild end - pos) / (fild tempo * (192.0 / 2646000000.0))) with the constants at 0x4BBE08 (192.0) and 0x4BBE10 (2646000000.0) divided at run time; step = (fild tempo * fimul n) * (192/2646000000), stored as double, pos = pos + step; n >= 2048 -> 2048, else the last chunk. rbhost computes the same in SSE2 double.

Run 1 (2026-10-26 14:43, one permit): tools/rbhost/run.py -v -o OUT tests/songs/diag/songs/303_accent0.rbs: the DLL loaded at 0x10000000, x87 CW at entry 0x037F (mingw) -> 0x027F; replies '@' 40 bc 02 00 00 00 (msg 50), 'C' (56), 'A' 00 (52), 'I' (73) for each msg-37 batch; 176400 frames in 32 ms. The DLL runs without Rebirth.exe and without any registration: no emulation beyond the panel side of the two rings and the 50/56 handshake was needed.

Run 2 (AC #2): run.py --runs 2 --dump-tables --dump-state on 303_accent0, 808_snare, delay_straight_fb64: each pair byte-identical (sha256 175e0dd2..., 0f70bef4..., c060ebca...). Table dumps identical across all six runs. State dumps of two runs differ only in 13 objects at pointer/handle offsets (heap addresses, e.g. +0xA of every device, the 808/909 voice pointer arrays at +0x18DA.., engine +0x0/+0x412.. handles, transport +0x16E/+0x172), as expected from the heap.

Table check (tabcheck.py on the dump): T[127] = 1, T[126] = 0.9583333, T[0] = 0; T808[0,64,127] = 203, 256, 321 = trunc(256*2^((i-64)/192)); T909[0,64,127] = 188, 256, 346 = trunc(256*2^((i-64)/144)); DT = 0.999625 .. 1.0 in steps of 2.5e-5; pitch[0] = 1.8539e-4, pitch[3072]*22050 = 65.406 Hz; every table non-zero where expected.

Fit to an app export: tools/rb_export.py --restore-prf --startup-sound off --wait-after-open 10 on 303_accent0, 808_snare, silence (fresh process each, 14:44-14:46).
- Sweep 1 (rbhost without an untitled song, idle 4800..11184 step 16): no exact match; the best was idle 9088 (k = 568): +-1 LSB on 68 samples (34 frames, all before frame ~5000; rms 0.041 over 20000 frames), every other N rms >= 72. Varying --post-stop-blocks M (0..160, total 9088) and --first-chunk-drain gave the same file (sha 0e7c844e...), so the stop's position does not matter for this song. --idle-fpu-cw 0x37F (64-bit precision during idle) gave the same file too; 0x07F differed.
- Failed hypotheses: the stop position, the first-chunk drain, the idle thread's x87 precision.
- Cause found: activation sub_1001BDA4 with no current song calls slot 6 (sub_10013D40) on the NEW song, whose 303 note-off (0x10025630 -> sub_10023C74) sets the release constants +0xCC8/+0xCCC before its idle blocks; in the app the opened song replaces "Untitled song", so slot 6 goes to the untitled song and the new song idles with the constructor's values. rbhost now creates and activates an empty untitled song first (default; --no-untitled to skip).
- Sweep 2 with the untitled song, idle 9056..9120: idle 9088 is byte-identical to the app's export (all 176400 frames). So the app export of 303_accent0 in that run is exactly the model "load, activate, N = 9088 idle blocks (k = 568 buffers of 1024 frames), export".

Further runs (2026-10-26, final build with the untitled song):
- 808_snare: sweep from idle 7600 step 16: the first identical render is idle 9088, the same N as 303_accent0 in that app session. silence at idle 9088: identical (all zeros). The whole WAV files (with the 44-byte header) of all three are byte-identical to the app's: sha256 1efcc132... (303_accent0), f31175af... (808_snare), 0a90d043... (silence).
- AC #2 again: run.py --runs 2 --idle-blocks 9088 --dump-tables --dump-state on 303_accent0, 808_snare, delay_straight_fb64: pairs identical (1efcc132..., f31175af..., dece122a...); once more after the last rebuild for 303_accent0 with --match: both identical to the app export.
- --drum-counters 9,0,0,0 on 808_snare: first difference from the app export at frame 64 (the first snare), as expected for a moved snare counter.
- --startup-sound on silence with idle 0: the startup sample is in the export (max 12274, 48 % non-zero samples), so the preview path works; with idle blocks it runs out before the export.
- Unit tests: /usr/bin/python3 -m unittest tools/rbhost/rbsmsg_test.py tools/rbs_test.py tools/rb_export_test.py tools/rbcmp_test.py: 47 pass (15 new).
- Reproduce: tools/rb_export.py --restore-prf --startup-sound off --wait-after-open 10 -o APP tests/songs/diag/songs/303_accent0.rbs; tools/rbhost/run.py --idle-sweep 8000:10400:16 --match APP/303_accent0.wav -o OUT tests/songs/diag/songs/303_accent0.rbs (N differs per app session).
- Scratch scripts (not committed): wavstat.py, tabcheck.py, cmpdirs.py, rank.py, diffs.py in the session scratchpad.

Acceptance criteria:
- #1: tools/rbhost/run.py builds rbhost.exe on the Mac (build.sh), uploads it with the message file to C:\re\rb\rbhost and runs it with rcs exec; no Rebirth.exe was running in any rbhost run (rb_export.py stops its app before releasing; rbhost loads the DLL itself). Renders of 303_accent0, 808_snare, delay_straight_fb64, silence passed rb_export.check_wav (frames = the export length formula).
- #2: --runs 2 gave identical WAVs for three songs (1efcc132..., f31175af..., dece122a...), with both the first and the final build.
- #3: --dump-tables writes t, pan, pitch303, filt303, pcf_lp, pcf_bp, dist_w, t808, t909, dt909 (every table of section 10 item 4) and manifest.json with name, file, IDA address, size, type, shape and builder; identical across runs; values checked against the formulas (tabcheck.py).

Review round 1 (send back: evidence and doc fixes; code passed). Fixes:
1. AC #1 evidence: run.py now runs tasklist for Rebirth.exe before every rbhost run (rb_export.Machine.rebirth_pids), refuses if one exists, and records the list in the sidecar as "rebirth_exe_pids_before". Run 2026-10-26 15:03 (one permit): tools/rbhost/run.py --idle-blocks 9088 -o OUT tests/songs/diag/songs/303_accent0.rbs tests/songs/diag/songs/808_snare.rbs tests/songs/diag/songs/silence.rbs: log "no Rebirth.exe running" before each of the three runs, sidecars "rebirth_exe_pids_before": [].
2. App export vs rbhost, full SHA-256 of the whole WAV files (shasum -a 256 on the downloaded files):
   - capture (app, Rebirth.exe, 2026-10-26 14:44-14:46, one session, fresh process per song): tools/rb_export.py --restore-prf --startup-sound off --wait-after-open 10 -o APP tests/songs/diag/songs/303_accent0.rbs tests/songs/diag/songs/808_snare.rbs tests/songs/diag/songs/silence.rbs
   - render (rbhost, 15:03): tools/rbhost/run.py --idle-blocks 9088 -o OUT tests/songs/diag/songs/303_accent0.rbs tests/songs/diag/songs/808_snare.rbs tests/songs/diag/songs/silence.rbs
   - 303_accent0: app 1efcc1325788342319ebb4f425aa604462c89a78313f8f8bfe2272ce5e4bbaaf, rbhost 1efcc1325788342319ebb4f425aa604462c89a78313f8f8bfe2272ce5e4bbaaf
   - 808_snare: app f31175af005b32f2a3cdc595991e475fceb79edf4db39118b918e29c3c381979, rbhost f31175af005b32f2a3cdc595991e475fceb79edf4db39118b918e29c3c381979
   - silence: app 0a90d04342bfc159bae46c1dfd351be1d646c6378b5d60b7aee3c722b48c38db, rbhost 0a90d04342bfc159bae46c1dfd351be1d646c6378b5d60b7aee3c722b48c38db
   - N = 9088 was found by: run.py --idle-sweep 9056:9136:16 --match APP/303_accent0.wav (first match 9088) and run.py --idle-sweep 7600:10800:16 --match APP/808_snare.wav (first match 9088). That the app's N was 9088 for both is marked [G] in export_state.md 12 (an inference from the matches).
3. Song count made consistent (three songs: 303_accent0 and 808_snare with fitted N, and silence) in export_state.md 1 and initial_investigation.md 9.
4. rbsmsg.py usage: OUT.rbhm.
5. initial_investigation.md 8.1: the W address correction marked [C].
6. README and test_machine.md 9: "loaded read-only" reworded (nothing is written to the installation).
Tests: 47 Python unit tests pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rbhost works and reproduces the app's exports byte for byte. Branch worktree-agent-a7be7bcd4d2048bf3, one commit "tools: rbhost, an offline host for the original ReBirth.dll".

- tools/rbhost/: rbhost.c (C, i686, built on the Mac by build.sh with llvm-mingw 20260922 from ~/llvm-mingw), rbsmsg.py (.rbs -> the song-load messages of docs/song_load.md, 15 unit tests), run.py (build, convert, upload to C:\re\rb\rbhost, run under ONE permit of redc-win-run, download; --runs, --idle-sweep A:B:S --match APP.wav for fitting N), README.md.
- The DLL needs no panel emulation beyond the two shared-memory rings: the handshake msg 50, the built-in mod "Standard ReBirth" registered with msg 56 {411}, msg 52, the load messages, msg 53. The x87 control word must be the app's 0x027F. An empty untitled song must be active before the song (else activation note-offs the new song's 303s: +-1 LSB in the first ~5000 frames).
- Options: --idle-blocks N, --post-stop-blocks M, --drum-counters A,B,C,D, --startup-sound, --first-chunk-drain, --no-untitled, --dump-tables DIR (10 tables + manifest.json), --dump-state DIR (engine, transport, song, mixer, 8 devices, 24 drum voices, drum counters, delay buffer + manifest.json).
- Results: two renders byte-identical (3 songs x 2); the WAVs of 303_accent0, 808_snare and silence from a fresh app session (rb_export.py protocol of export_state 11.5) are byte-identical to rbhost renders with --idle-blocks 9088 (k = 568). A render takes ~50 ms.
- Docs: export_state.md 2.7 (ring layout, [G] -> [C]), 3 (activation without a current song), new 12 (what the DLL needs, results), 1/11.5 updated; initial_investigation.md 8.1 (W is at 0x100D47A4), 9, 10 item 4 (table addresses, builders, guards); song_load.md 2 (mod registration); test_machine.md 9.
- Tests: 47 Python unit tests pass; the test-machine runs are in the notes.
<!-- SECTION:FINAL_SUMMARY:END -->

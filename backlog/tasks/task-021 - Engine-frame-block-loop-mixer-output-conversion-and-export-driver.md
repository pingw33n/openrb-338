---
id: TASK-021
title: 'Engine frame: block loop, mixer, output conversion and export driver'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 17:44'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-019
priority: high
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The engine frame (docs/initial_investigation.md sections 3, 4, 9): the 64-frame block loop with the leftover buffer and the 0x4000 cap, the mixer block (1e-6 bus fill, per-channel device then inserts dist, PCF, comp, the send accumulation, the 0.93/0.07 smoothing with the in-block ramp, the delay, the master comp, the master gain), the 16-block mixer update with the counter starting at 10, the T and PAN curves, routing, the float-to-int16 output conversion, and the export driver (the double-precision chunking of section 3 with 2048-frame calls). An explicit initial-state struct lets a caller set the state the decided capture protocol leaves (the leftover buffer, the counters, the smoothed gains) per docs/export_state.md. Devices are traits so the synths, drums and effects plug in later; test with a stub device against hand-computed mixer outputs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The block loop, mixer and output conversion follow sections 3 and 4 with unit tests for smoothing, pan law, routing order and int16 conversion
- [x] #2 The export driver's frame count equals the original's for the corpus songs' tempos and loop lengths (test)
- [x] #3 The silence song of the corpus renders exactly equal to its reference
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Check sections 3 and 4 against the disassembly: mixer block 0x10013444, mixer update 0x10012C14, render/convert 0x100139E0, song ctor 0x10011350 (initial values, device lists), msg 33 0x10014140, the exe's export loop 0x4280D5 (constants 0x4BBE08/0x4BBE10).
2. crates/rb-engine/src/frame/: device traits (ChannelDevice slot 14/4/5/6, InsertEffect slot 15/14/16, SendEffect slot 14), a Transport seam for the sequencer (task-022) with an idle placeholder, MixerParams (the mixer device's block, ctor defaults), MixerState and EngineState (explicit initial state: counter, meter index, channel gains, send gains, peaks, routing, master, leftover), the block loop, the 16-block update, msg 33, the leftover logic and the int16 conversion.
3. frame/export.rs: the chunk plan of sub_4280D5 in f64 and the export driver as rbhost runs it (N idle blocks as 1024-frame calls, stop, locate, play, chunks, stop, msg 33, 0-frame call).
4. Unit tests with stub devices and effects (hand-computed smoothing/ramp, pan law, routing order, update phase, leftover, int16 conversion, msg 33).
5. Integration test: frame counts against manifest.json for every diag and shipped entry; silence song rendered at N=9088 hashed as rbhost's WAV against the manifest sha256 (sha2 dev-dependency).
6. Facts into docs/initial_investigation.md sections 3 and 4.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a8067c33c0150e9da

Disassembly checked (headless scratchpad copies of ida/ReBirth.dll.i64 and ida/Rebirth.exe.i64, closed save=false):
- Mixer block 0x10013444: counter test 0x10013453, sequencer 0x1001346A, meter index (i+23)&63 at 0x10013481, master fill with 0x358637BD only for non-null output pointers, channel devices slot 14 with float** (0x10013580), inserts slot 15 at +0xC4/+0xC8/+0xCC (dist, PCF, comp), channel peak 0x100135FA, send loop 0x10013650 f32(bus*g + send), smoothing 0x100136B7 (0.93 = dbl_10151F48, 0.07 = dbl_10151F50, 1/32 = dbl_10151F58), mix loop 0x10013742 (pair p uses old + p steps, L gain +0xB0/+0xA8 into out[0], R gain +0xAC/+0xA4 into out[1]), send effect slot 14 (0x10013865) with (song+0x1FC, L, R), master comp 0x100138AC, master gain 0x100138AE (unrounded m), master peaks 0x10013960.
- A channel with its own output entry (2,3,4,11) is not mixed into the master (0x100136B1); sub_100139E0 always leaves them null, so all four channels share the scratch unk_1010012C.
- Mixer update 0x10012C14: targets f32(1e-6 + PAN[lev][pan][0|1]) to +0xA8 / +0xA4, send T[lev]*T[send]+1e-6 or Vintage (PAN0+PAN1)*T[send]*0.5 + dbl_10151F28, routing rebuilt each time, slot 16 on dist, PCF, comp (list song+0x1EC from sub_10014104), master target 2*T[master]+1e-6, meters (127 scale, 0.92 decay). Callers: only the block and the song ctor (0x100119D4).
- Ctor sub_10011350: gains/targets/send 1e-6 (0x100113AF..), master 1.0/1.0, counter 10, leftover 0, send buses at +0x160C = 1e-6; the closing mixer update reads the mixer device's ctor defaults (sub_10019CC0: master 100; sub_10019D50: enabled 1, level 100, pan 64, send 0, dist 0).
- Delay 0x1000F940 refills the send bus with 1e-6 in both paths (so the mixer frame leaves it to the SendEffect).
- msg 33 sub_10014140: master current 0, peaks 0, the three +0x160C buses 1e-6, 18 leftover blocks 0, slot 5 in device order.
- sub_100139E0: clamp 0x4000, leftover logic as docs/export_state.md 5, conversion 0x10013C4E with dbl_10151F70 = 32768, flt_10151F78 = 32767, flt_10151F7C = -32768 (NaN -> -32768), fistp truncation.
- Exe sub_4280D5: fild start/end/tempo (int32), 192.0 (dbl_4BBE08) / 2646000000.0 (dbl_4BBE10) at run time, __ftol, step = (tempo*n)*(192/2646e6), render+write before pos += step.
Facts added to docs/initial_investigation.md sections 3 and 4.

Implementation: crates/rb-engine/src/frame/mod.rs (traits ChannelDevice/InsertEffect/SendEffect/Transport, placeholders Silent/Bypass/IdleSend/Idle, MixerParams, MixerState/OutputState/EngineState as the explicit initial state, Frame::block = 0x10013444, Frame::mixer_update = 0x10012C14, Engine::render = 0x100139E0, Engine::reset = msg 33, to_i16), frame/export.rs (chunk_sizes, frame_count, render_idle, export with N idle blocks as 1024-frame calls exactly as tools/rbhost/rbhost.c, wav_bytes). Tests: frame/tests.rs (13 unit tests with stub devices: hand-computed smoothing and ramp, pan law and sends incl. Vintage, routing order and update order, update phase from counter 10, leftover equivalence and negative L, msg 33, int16 conversion, chunk plan) and tests/corpus_frame.rs.

Results:
- Frame counts: all 153 manifest entries (145 diag + 8 shipped) equal. A one-off check (not committed) found that only shipped/Fast Forward (140 BPM, 192000 ticks) differs from floor(exact) = 18900000: the accumulation gives 18899999, as the reference; 152 of 153 end on a chunk not a multiple of 64.
- Silence song at --idle-blocks 9088: WAV sha256 0a90d04342bf... equal to the manifest (all zeros; with the placeholder devices this checks the driver, frame count, header and the conversion of the 1e-6 guards, not the device DSP).
- cargo test: 61 pass (rb-engine 49 unit + 2 corpus + 1 rbsmsg, rbs 5 + 4); clippy -D warnings clean; fmt clean.
Reproduce: cargo test -p rb-engine frame; cargo test -p rb-engine --test corpus_frame.
Not confirmed: the device-side meaning of the scratch +0x140C/+0x150C path (null master pointers never occur in the app).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The engine frame is in crates/rb-engine/src/frame: the 64-frame block (0x10013444) with the 1e-6 guards, per-channel device then dist/PCF/comp inserts, send accumulation, 0.93/0.07 smoothing with the per-pair ramp, send effect, master comp and master gain; the 16-block mixer update (0x10012C14) with the counter starting at 10, T/PAN targets, Vintage send and routing; the leftover block and 0x4000 clamp of sub_100139E0; the int16 conversion; msg 33; and the export driver (sub_4280D5 chunking in f64, N idle blocks as 1024-frame calls then stop, locate, play, chunks, stop, msg 33, 0-frame call, as rbhost). Devices, inserts, the send effect and the sequencer are traits with placeholders; EngineState is the explicit initial state. All checked against the disassembly; new facts with addresses in docs/initial_investigation.md sections 3 and 4.
Numbers: frame counts equal the manifest for all 153 diag and shipped songs (Fast Forward 18899999, one below the exact count); the silence song at N = 9088 equals its reference WAV sha256 exactly.
Tests: cargo test 61 pass (13 new frame unit tests, 2 corpus tests), clippy and fmt clean.
Branch worktree-agent-a8067c33c0150e9da, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

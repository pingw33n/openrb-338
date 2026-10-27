---
id: TASK-072
title: 'RE: the PCF''s stale step slot on the 303 channels'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 13:49'
updated_date: '2026-10-27 16:11'
labels:
  - re
  - engine
milestone: m-3
dependencies:
  - TASK-069
references:
  - crates/rb-engine/src/pcf.rs
  - crates/rb-engine/src/tr808.rs
  - crates/rb-engine/src/tr909.rs
  - docs/original_bugs.md
priority: low
type: task
ordinal: 97000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-069 found that the PCF block (0x100166D0) with a wave whose resolution is not 8, 16 or 32 (waves 55..127, OB-7 in docs/original_bugs.md) takes its step from the uninitialised stack slot var_1C (entry esp-0x1C), and that between PCF blocks the code before it at the same stack depth rewrites that slot. On the 808 and 909 channels it is the channel's audio slot (return addresses 0x10028278, 0x1002B8B4; rb: ChannelDevice::stack_word). The 303 audio (0x10023F10 / 0x100248C4) did not write the slot in the measured session, so with the PCF on a 303 channel (pcf_id 1 or 2) the slot holds older stack contents: in task-069's watch log the other writers per block were the sequencer clock sub_1001EF20 (0x1001F3B7, tick blocks only), the delay 0x1000F940 (0x1000FA9E, 64 writes per block, pointers like 0x100F012C + 0x100*k + 0xFC), the mixer update 0x10012C14 (0x10012C3E), the 808/909 audio, and code outside the block loop (the pump's 0-frame calls, the start of each render call). rb keeps the PCF's last quotient there, a [G] stand-in.

Do: confirm statically that the 303 audio never writes esp0-0x1C (both sound models), then measure var_1C at PCF entry with the PCF on channel 1 and 2 (rbhost --live with a patched vtable thunk and a DR0 write watchpoint, as in task-069's notes: the thunk is a naked function that reads [esp-0x1C] and jumps to 0x100166D0, the watch a VEH that sets Dr0/Dr7 from a RaiseException), in the live path and in the export path (2048-frame chunks), and model it in rb-engine (BlockContext::stack_word) if it is deterministic; otherwise document it as not reproducible. Only matters for a junk wave with a trigger, or a junk wave followed by a real one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The value of var_1C at the PCF's entry on the 303 channels is documented with addresses in docs/initial_investigation.md 8.2 and OB-7
- [x] #2 rb reproduces it or OB-7 says why it cannot
- [x] #3 cargo test --release and the corpus compare stay green
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Static: the 303 audio slots (0x10023F10 2.0, 0x100248C4 vintage) against S = their esp0-0x1C; every other writer of S in a block and around it (mixer update, sequencer clock, inserts, delay, render call 0x100139E0 and its lock).
2. Model the slot as a word the frame carries through the block (Frame state), written in the original's order: render-call start, mixer update, clock, each channel's audio and inserts, delay; the PCF on any channel reads it (BlockContext::stack_word).
3. Measure with an instrumented rbhost (PCF vtable thunk logging [esp-0x1C] per PCF block; DR0 write watch) in --live (1024-frame calls) and in the export path (2048-frame chunks), PCF on channels 1 and 2 with a junk wave, across delay on/off, compressor on channel/master/off, stop/play, tempo; compare the logged word per block with rb's.
4. rb-engine: the model; unit tests; rb-player live test pinning rbhost's hash of a session with the PCF on the 303s.
5. Docs: initial_investigation 8.2, OB-7; cargo test --release, golden, rb compare 164/164.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-af3f570f2533fac78

Static (objdump of original/ReBirth.dll and IDA, read-only copy, closed save=false):
- Slot S = mixer block esp-0x24 (sub_10013444: 4 pushes + sub esp,5Ch = B; devices/inserts called after one push, entry esp B-8, S = entry-0x1C).
- 303 audio slot 14 = 0x10023F10 (4 pushes, sub esp,19Ch: S = esp+0x190; highest local esp+0x148; calls 0x10013DA0 from esp0-0x1AC; the vintage path 0x1002489A calls 0x100248C4 from esp0-0x1B0, frame 0x1A8 below that): never writes S, both sound models. Confirmed.
- Other writers of S (watch EIP is the next instruction): mixer update sub_10012C14 0x10012C3A mov [esp+34h],ecx = [[mixer+0x170]+0x30+4] = comp_id (the switch at 0x10012F5B..0x100130E6 reads it as the compressor id: 2..5 channels, 1 master); clock sub_1001EF20 0x1001F3B3 mov [esp+34h],eax = [globals+4] = tempo, on the path to the devices' step loop (t > lastTick, not the song-end return); mono comp 0x1000EC70 0x1000ECE9 mov [esp+8],ecx = +0x58 (release 0x3F7F9724, written only by the ctor 0x1000EA38) when enabled (+0x38); PCF 0x10016792/A0/AE (q) when enabled and res 8/16/32; delay 0x1000F940 0x1000FA9A mov [esp+10h],ecx (called at 0x10013865 from B-0x10): its right-output pointer arg3-4+4k, last arg3+0xFC, only when enabled (+0x34; the 0x1000FAD1 path writes none); the stereo comp 0x1000EDE0 on the master has no local at S (esp+0x1C there; stores at +0x18, +0x20 fstpl..).
- Render call sub_100139E0: L/R float outputs 0x100E012C/0x100F012C (0x10013A35..4A), advanced 0x100 per whole block (0x10013B88); the partial block (0x10013BAA) renders into song+0x20C+0x100c, so the delay's word is song+0x408 (heap). The try-lock sub_10009450 at 0x10013A5E (WaitForSingleObject via [0x1002F00C], timeout 0) runs at the start of every call, n = 0 too; the system DLLs below it write S, last ntdll (watch EIP 0x7751E801) value 0.
Measurement: scratch rbhost_dbg.c = tools/rbhost/rbhost.c + a naked thunk in CPCFInsert vtable slot 15 (0x10033AD4+0x3C) logging [esp-0x1C] and the call index at every PCF block, both paths, plus --watch (VEH + DR0 write watchpoint on S after the first PCF block). Not committed. Runs on the test machine in C:\re\rb\task072 (deleted after) under one redc-win-run permit.
- Live: session pcf_303_session over full_mix (new test pcf_junk_wave_on_the_303_channels; RB_LIVE_OUT writes pcf303.rblv), rbhost_dbg --watch --live: 7354 PCF blocks; words 0x0001FBD0 (tempo 130000) x3280, 0x3F7F9724 x1867, 0x1002B8B4 x448, 0 x209, 0x100F0228+0x100k x~95 each, comp ids 2 x86, 5 x85, 1 x39. Writers seen: 1000FA9E (delay), 1001E300/E4D0/E700/E9A0 (808/909 voice calls), 1001F3B7 (clock), 1000ECED (comp), 10012C3E (update), 100167A4 (PCF q), ntdll/kernelbase at each call start (last 7751E801 = 0).
- Exports (rbhost_dbg --idle-blocks 9088, scratch songs from pcf_lp_freq120_res20 with wave 108): s1 pcf 1 comp off delay on 120 BPM; s2 pcf 2 comp ch0 delay off 133; s3 pcf 2 comp master delay on 97.003 vintage; s4 pcf 1 comp ch3 delay off 120; s5 pcf 2 comp ch0 disabled delay on 143.21 vintage.
Model in rb: Frame::stack_word set by LOCK_WORD (Engine::render), the update, the clock (Sequencer::tick now returns whether it reached the step loop), ChannelDevice::stack_word, InsertEffect::stack_word (PCF q, comp release), SendEffect::stack_word (delay, Frame::out_end = 0x100F012C + 4*(pos+63), None for the partial block).
Result: rb's word per PCF block (a temporary log, removed) equals rbhost's in all 66485 PCF blocks (live 7354, s1 11839, s2 11569, s3 12492, s4 11839, s5 11392), 0 None. WAVs: live session rb 7e57e65c.. = rbhost (plain and instrumented build) 7e57e65c..; before the fix rb gave f8d83621... s1..s5 rb compare EQUAL (before: s1 and s4 differed at frame 66, rb -51 ref -84; s2, s3, s5 were equal before too because 303 #2 is silent in those songs, so their evidence is the word log). Plain rbhost WAVs of s1, s2, s4 equal the instrumented ones (sha256 f3bf38b8.., 43aeadc1.., 498a8cb3..).

AC evidence: #1 docs/initial_investigation.md 8.2 (the [G] on the 303 channels is now [C] task-072, with every writer and address; the 'Past the patterns' item says [C] on every channel) and docs/original_bugs.md OB-7 (now [C]; What, Where, Measured, rb). #2 rb reproduces it: Frame::stack_word; the word matches rbhost in all 66485 PCF blocks; only the heap pointer after a partial block with the delay on is not reproducible (None, the last-quotient stand-in), and the lock's 0 is the test machine's Windows 11 (OB-7 says so). #3 RB_STANDARD_MOD=target/Standard.rbm cargo test --release: 399 pass, 0 fail (corpus_golden included); rb compare --manifest tests/refs/manifest.json --standard-mod target/Standard.rbm: 164 of 164 equal; cargo clippy --all-targets -D warnings clean; cargo fmt applied.
Tests added: frame::tests::the_stale_slot_through_render_calls, the_clock_and_an_earlier_insert_write_the_slot; pcf::tests::the_block_leaves_its_quotient_on_the_stack; comp::tests::the_mono_block_leaves_the_release_factor; delay::tests::the_enabled_block_leaves_its_output_pointer; rb-player live_messages pcf_junk_wave_on_the_303_channels (rbhost sha256 7e57e65c1cf82e28d1a47ac00d57607c5b440a2ee8423ed19382e1018dca4913).
Reproduce: RB_LIVE_OUT=DIR cargo test --release -p rb-player --test live_messages pcf_junk writes DIR/pcf303.rblv; rbhost.exe --live pcf303.rblv full_mix.rbhm out.wav (full_mix.rbhm from tools/rbhost/rbsmsg.py tests/songs/diag/songs/full_mix.rbs) gives 7e57e65c...

After the rebase onto main bc41845: cargo test --release with RB_STANDARD_MOD set to the absolute path of target/Standard.rbm gives 417 pass, 0 fail (main's new rb-ui menu test does not find the mod through the relative path, unrelated to this task); clippy clean. The diff against main lists only this task's 13 files.

Review round 1 (send back): the partial-block case (the delay leaving song+0x408 after a block shorter than 64 frames) is static only, never measured (the sessions use 1024/2048-frame calls). Marked [G] (derived from 0x10013BAA, not observed) in initial_investigation 8.2 and OB-7 Where; OB-7 Measured now says no measured PCF block followed a partial block with the delay on. Docs only; commit amended.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Found and modeled. On the 303 channels the PCF's stale var_1C (OB-7) is not left by the 303 audio (0x10023F10 / 0x100248C4 never reach esp0-0x1C) but by the last of these writers, in block order: the render call's try-lock sub_10009450 at 0x10013A5E (0 on the test machine's Windows 11, at the start of every call), the mixer update's compressor id (0x10012C3A, every 16th block), the clock's tempo (0x1001F3B3, blocks that reach a new tick), an enabled compressor on channel 0 (release factor 0x3F7F9724, 0x1000ECE9), and from the block before the 909's return address 0x1002B8B4, a compressor on channel 3, or the enabled delay's last right-output pointer 0x100F012C + 4*(pos+63) (0x1000FA9A; a heap address after a partial block, the one case rb cannot know). rb-engine now carries the word through the block (Frame::stack_word, Frame::out_end, LOCK_WORD; Sequencer::tick returns whether it stepped; stack_word hooks on the PCF, compressor and delay). Measured with a scratch instrumented rbhost (vtable thunk + DR0 write watchpoint): rb's word equals the DLL's in all 66485 PCF blocks of a live session and five exports; the new live test pcf_junk_wave_on_the_303_channels has rbhost's hash 7e57e65c (rb before: f8d83621); scratch export songs s1/s4 went from differing at frame 66 to EQUAL. Docs: initial_investigation 8.2, OB-7 now [C]. Tests: cargo test --release 417 pass on the rebased tree (golden included), corpus compare 164/164 equal, clippy clean. Branch worktree-agent-af3f570f2533fac78, one commit on main bc41845.
<!-- SECTION:FINAL_SUMMARY:END -->

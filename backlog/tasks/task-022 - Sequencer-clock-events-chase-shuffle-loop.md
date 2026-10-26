---
id: TASK-022
title: 'Sequencer: clock, events, chase, shuffle, loop'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 18:24'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-018
  - TASK-021
priority: high
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The sequencer (docs/initial_investigation.md section 5): the double position advanced by tempo*4.6439909297052154e-06 per block before rendering, integer ticks, the track events applied in the first block whose tick reaches them, the loop-end wrap with chase, locate/start/stop semantics, song end at 767231, shuffle (96*(t/96)+max(0,t%96-shuffle/6)), device step calls only when the tick increments. Unit tests on tick sequences for given tempos and on chase results.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Section 5's clock, chase, locate/start/stop, loop wrap, song end and shuffle are implemented with unit tests
- [x] #2 Event and step timing (the block in which each fires) is tested for at least three tempos
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE (before coding, idalib on a scratch copy): sequencer tick 0x1001EF20, locate 0x1001F984, stop 0x1001E100/0x100218A0, play 0x1001BE74/0x1001F7E4, song mode 0x1001FBE0, the track list (ctor 0x100144B0, insert 0x10014750, chase 0x100151F0, stop prune 0x100149F0), msg 35/36 dispatch (0x10012555, 0x100126B2), activation 0x1001BDA4, the mixer's set-param 0x10015C00, song-mode slot 9 0x10016280 and data msg 143 0x1001588F, the send-bus registration 0x100140B4 (msg 33's extra buses).
2. New module crates/rb-engine/src/seq.rs (+ seq/tests.rs): Track (the DLL's sorted circular list as a Vec in list order, with the insert rule, the stop prune, the chase and the clock's apply loop) and Sequencer (pos f64, lastTick, the saved song-mode tick, 9 tracks, the global struct) with tick, locate, play, stop, song mode and the msg 35 setters, in the original's operand order (pos + tempo*K, trunc via i64, pos - inc for the loop test, pos + (start - end) for the wrap).
3. Reshape the frame's traits: a SongDevice supertrait for the vtable slots the sequencer and the load use (1 step(tick, shuffled, tempo), 2 relocate, 3 start, 4 stop, 5 reset, 6 silence, 9 song mode, 10 data, 11 set_param), under ChannelDevice, InsertEffect, SendEffect, plus a concrete MixerDevice (live and pattern-mode parameter sets, set_param, song mode, the msg 143 rule). Devices gets the mixer and a dispatch in song order (mixer, 303, 303, 808, 909, delay, dist, PCF, comp). The Transport trait and Idle go; Frame owns the Sequencer. Every call lands between blocks, i.e. on sample 0 of the next block.
4. Engine: new() runs the ctor's tempo 140000 and locate 0; load() replays the load message stream in order (msg 36 field by field, msg 41 x9 in load order, msg 37 inserts, msg 32); activate() models top-level 53 (the global delay buffer clear when a song was current, slot 6 on the song otherwise); export_song() = load, activate as rbhost does, idle, export. Fix the export.rs comment.
5. Unit tests with hand-derived expectations: the block of each step and event at 60, 120, 140 and 300 BPM, shuffle offsets, event order (tracks, reverse arrival within a tick, events before steps), chase at the loop start, loop wrap, song end, stop prune, locate/play/stop, mixer song-mode rules. The silence corpus test and the frame-count test still pass.
6. Docs: initial_investigation.md sections 4, 5 and 5.1 (the exact algorithm with addresses; msg 33's spare send buses), export_state.md 3 and 6.2/6.3, song_load.md 5, 6 and 11.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26, from task-021's review (non-blocking items for this task):
- crates/rb-engine/src/frame/export.rs:86-87 says export runs "after the song is loaded and activated", but export() does not model activation (msg 53), which per tools/rbhost/rbhost.c:544-555 changes the 303 release constants. Model activation in the export path (or in the 303 task if it only affects the 303) and fix the comment.
- frame/mod.rs:732-739 Engine::reset (msg 33) refills only the one send bus with 1e-6; the DLL also refills +0x170C and +0x180C (0x10014155..0x10014183; 256-byte buses at 0x160C + k*0x100) whose readers are unknown. Find their readers (they may be the drum or 303 buses) and model them where they belong; record the result in docs/initial_investigation.md.
- The frame's Device/Effect/Transport traits pass only what task-021 needed; reshape them here as the sequencer requires.

Branch: worktree-agent-a02d6926c050263a1

RE evidence (idalib, scratch copy of ida/ReBirth.dll.i64, force_headless, closed save=false); all checked in the disassembly, Hex-Rays only as a second opinion:
- Clock sub_1001EF20: fild tempo; fmul dbl_10153808 (bytes 0x3ED37A7281B22126 = 4.6439909297052154e-06); fld pos; fadd st,st(1) (pos + inc); fistp qword (low dword) = t; jle when t <= lastTick (signed). Song mode only: loop test fild loopEnd; fcomp [pos] (and 4100h; jz skip: proceed if loopEnd <= pos), then fld pos; fsub inc; fild loopEnd; fcompp (jnz skip: proceed if loopEnd > pos - inc). Wrap: apply events <= loopEnd-1, record merge (byte2, sub_1001F520), pos = pos + fild(loopStart - loopEnd), chase to loopStart (sub_100151F0 + sub_10012B40 for ctrl 0..127), slot 2 on every device. Then apply events <= t (0x1001F2AD). Song end: fld pos; fcomp dbl_10153810 (0x412769FE00000000 = 767231.0); jz end means pos > 767231 strictly; end path 0x1001F484: sub_10021194(0), host msgs, sub_100218A0, locate 0, no step. Steps 0x1001F433: every device slot 1 (+4) with (t, 96*(t/96)+max(0,t%96 - shuffle/6), globals tempo); lastTick = t at 0x1001F479. Tempo source sub_1001BCC0: MIDI clock if sync else current song's globals+4.
- Device list song+0x18C (count +0x1C4) in ctor order: mixer, 303, 303, 808, 909, delay, dist, PCF, comp. Vtables: CDevice slots 1/2/3 = retn 0Ch 0x10010380 / retn 4 0x1001BF40 / retn 0x10012C00 for mixer, delay, inserts; 303 slots 1/2/3 = 0x100253A0/0x100255E0/0x100255B0, 808 0x10027690/0x10028030/0x10028080, 909 0x1002AAC0/0x1002B670/0x1002B6C0; set-param slot 11 (+0x2C): mixer 0x10015C00, 303 0x10026180, 808 0x10029050, 909 0x1002C6D0, delay 0x1000FDC0.
- Track list: ctor sub_100144B0 (head = list, tick 0x7FFFFFFF, quant bytes +0xD = 1, dirty +0xC); insert sub_10014750 (quantise sub_100146E4 = ceil to 24q, drop > 767231, insert before the first node with tick >= t unless same tick and ctrl -> overwrite: same-tick events in reverse arrival order); chase sub_100151F0; stop prune sub_100149F0 (repeat of the controller's previous value removed unless tick 0; last init 0xFF by sub_10014274).
- Transport: msg 35 table 0x10151E14 (ids 0,1,2,3,4,8,12,16,20,21,22); stop sub_1001E100 -> sub_100218A0 (slot 4 only when playing, prune, chase) + locate 768*(t/768) unless transport+0x160 (MIDI sync, SetMIDISyncActive via sub_1001C4C4); play sub_1001BE74 -> 0x1001F7E4 with ecx = song+8 (0x1001BEE9): slot 3 on every device when not playing; locate sub_1001F984 (max 0; record forward-apply or song-mode chase; pos, slot 2, lastTick); song mode sub_1001FBE0 (+ sub_10013FB0 slot 9 on change); msg 36 order at 0x100126B2; globals ctor sub_10018AC0 (tempo 120000, loop end 768, shuffle 64, byte22 1). transport+0x15C == 46 (ctor argument, [G] ReWire) keeps the values instead; not modelled.
- Mixer device: set-param 0x10015C00; slot 9 0x10016280 (live <-> pattern copy +0x58); msg 143 0x1001588F (pattern copy; song mode copies master+enables live, pattern mode all, 0x10015BB2).
- Supervisor item 1 (activation msg 53): sub_1001BDA4: a stopped current song gets slot 6 (sub_10013D40); its only effect on the new song is the delay slot 6 0x100100E0 (global buffer 0x10054794 refilled with 1e-6, byte_10054790 = 1); with no current song slot 6 runs on the new song. Modelled as Engine::activate(Activation); export::export_song does load + activate(ReplacesCurrent) + export as rbhost's default; export.rs comment fixed.
- Supervisor item 2 (msg 33's extra buses): sub_100140B4 registers send effects (object song+0x1DC+4k, bus pointer song+0x1FC+4k = song+0x160C+0x100k, count +0x1E8); its only caller is the ctor's delay; the block loops k < count (0x1001381F). So +0x170C and +0x180C have no reader; documented (initial_investigation section 4, Engine::reset doc), not modelled.
- Supervisor item 3 (traits): SongDevice supertrait (slots 1 step, 2 relocate, 3 start, 4 stop, 5 reset, 6 silence, 9 song mode, 10 data, 11 set_param) under ChannelDevice/InsertEffect/SendEffect; Devices gains the concrete MixerDevice, device_mut(id) and for_each (song order); Transport/Idle removed, Frame owns seq::Sequencer. All calls land between blocks (sample 0 of the next block): the original has no sub-block positions.
- Failed hypothesis: an event inserted at the applied node's tick would be applied next; no, it goes before the node (reverse order), test insert_rules fixed accordingly.
Numbers: inc per block 60/120/140/300 BPM = 0.2786394557823129/0.5572789115646258/0.6501587301587302/1.3931972789115645; first block reaching ticks 1,48,96,768: 60k 4,173,345,2757; 120k 2,87,173,1379; 140k 2,74,148,1182; 300k 1,35,69,552 (hand: ceil(n/inc), cross-checked with a python accumulation in the scratchpad nums.py).
Results: cargo test all pass (rb-engine lib 64, corpus_frame 2, corpus_seq 2, rbsmsg_corpus 1, others unchanged); clippy -D warnings clean; rustdoc -D warnings clean; silence corpus song exact (sha256 of the WAV equals the manifest's) through export_song; frame-count test 153 songs pass. corpus_seq: load state (globals, positions, track lengths, applied pointers, mixer live set derived independently) over all 153 corpus songs; export transport end state over 40.
Reproduce: cargo test -p rb-engine --release; cargo test -p rb-engine --release --test corpus_seq.

Review round 1 (send back, one doc defect): docs/song_load.md said the exe sends ticks up to 767976 and that batch boundaries do not matter, without evidence. Fixed: 767976 is now derived from the exe's check sub_42FD60 (multiples of 24 below 768000, song_load section 5) and marked as inferred from the two limits (no corpus song exceeds tick 417792); the batch claim cites the null hint at each batch start (mov edi, 0 at 0x10021061) and the hint rule of sub_10014750 (used only when its tick is below the new tick). The DLL drop at 767231 (cmp 0BB4FFh at 0x1001477B, also 0x10014BF2, 0x10014D08) stays.
Reviewer's non-blocking notes for the next tasks:
- Activation's real effect on the new song is the process-global delay buffer 0x10054794 refilled by the delay's slot 6 (0x100100E0, also byte_10054790 = 1). Engine::activate calls delay.silence(), but the placeholder delay does nothing yet: the delay task (task-026) must implement slot 6 on that global buffer.
- The mixer's set-param and msg 143 also set dirty bytes (+0x81, +0x82, +0x85 + 6c + k; stores at 0x10015CC8, 0x10015CE4, 0x10015CFB, 0x1001593F, 0x10015981; readers 0x10015F9B..0x1001609E). They are not modelled and their readers were not checked.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Sequencer implemented in crates/rb-engine/src/seq.rs from the disassembly of sub_1001EF20 and the transport functions (locate 0x1001F984, stop 0x1001E100/0x100218A0, play 0x1001BE74, song mode 0x1001FBE0, track list insert 0x10014750 / chase 0x100151F0 / prune 0x100149F0): f64 position advanced by tempo*TEMPO_STEP before rendering, integer ticks, events applied in the first block whose tick reaches them (tracks in order, same-tick events in reverse arrival order) before the devices' step calls, the loop wrap with chase at the loop start (the loop-end tick never fires), song end at pos > 767231, shuffle, stop prune and the bar locate. The frame's traits were reshaped into a SongDevice supertrait (step/relocate/start/stop/reset/silence/song mode/data/set_param) with a concrete MixerDevice (live and pattern-mode sets, msg 143 rule); Transport/Idle removed. Engine::load now replays the whole load message stream (msg 36, 41 x9, 37, 32), Engine::activate models msg 53, export::export_song is rbhost's default run. Supervisor review items: activation modelled (only the global delay buffer clear reaches the new song), msg 33's buses 1 and 2 have no reader (documented), traits reshaped. Docs: initial_investigation sections 4, 5, 5.1; export_state 6.2, 6.3; song_load 3, 6, 11. Tests: 16 new seq unit tests (step and event blocks at 60/120/140/300 BPM, shuffle, order, insert, prune, chase, wrap, song end, locate/play/stop, song mode, mixer rules), corpus_seq over the corpus; all cargo tests pass, clippy clean, silence song exact, frame counts match. Branch worktree-agent-a02d6926c050263a1, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

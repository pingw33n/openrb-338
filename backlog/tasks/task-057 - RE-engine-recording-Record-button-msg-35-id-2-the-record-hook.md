---
id: TASK-057
title: 'RE + engine: recording (Record button, msg 35 id 2, the record hook)'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 07:34'
updated_date: '2026-10-27 16:36'
labels:
  - re
  - engine
milestone: m-3
dependencies:
  - TASK-043
references:
  - docs/initial_investigation.md
  - crates/rb-engine/src/seq.rs
  - crates/rb-player/src/lib.rs
priority: low
type: feature
ordinal: 52000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the transport's Record button (docs/ui_spec.md section 3) toggles globals byte 2 (msg 35 id 2, sub_10021194) and in song mode records the panel's controller changes into the tracks. rb stores the byte (Sequencer::set_record) and models its effect on locate, but not the recording: the record hook sub_10021340 (called by the device messages, e.g. the 303's msg 40 at 0x10025CFC via sub_10012BC0; acts when globals byte 0 and byte 2 are set: it sets the track's hold byte +4+ctrl and inserts the event, quantised with sub_100146E0 while playing or sub_10014720 otherwise, loop-end adjusted, with sub_10014750 into a list), the merge when record is turned off or the loop wraps (sub_1001F520, Sequencer::tick's comment at 0x1001F0C7), and the hold mask's effect on the clock. The real-time player (task-043) therefore has no Record command.

Do: read the record path in ReBirth.dll (sub_10021194, sub_10021340, sub_1001F520 and the list it records into), document it with addresses in docs/initial_investigation.md section 5 (update its [G] items) and docs/export_state.md where it touches the transport, implement it in rb-engine's sequencer (allocation: the recorded events grow a Vec; the callback must not allocate, so reserve capacity up front or record into a fixed buffer, and say which), add Command::Record to rb-player, with tests including crates/rb-player/tests/no_alloc.rs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The record path is documented with addresses and its [G] items in docs/initial_investigation.md section 5 are confirmed or refuted
- [x] #2 rb-engine records as the original does and rb-player has a Record command, with tests; the callback stays allocation-free
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE (done in research, see notes): sub_10021194 (msg 35 id 2), the hook sub_10012BC0 -> sub_10021340, the merge sub_1001F520, the list helpers (sub_100146E4 round up, sub_10014720 round down, sub_10014750 insert, sub_10014840 find, sub_10014B30 erase, sub_100149A0 clear), the node pool (sub_100142C0: 100000 nodes, sub_10014434 refuses a node when <= 1 is free), the callers (loop wrap 0x1001F0C7, song end 0x1001F484, msgs 30/31, MIDI sub_1001C3C4).
2. Docs: docs/initial_investigation.md section 5 (the record list, hook, merge, record off, song end; [G] items), docs/song_load.md 4.10 (which controllers each device message hooks), docs/export_state.md where the transport is touched, docs/ui_spec.md 16, docs/original_bugs.md (recorded events past the merge tick are lost; the applied pointer of a deleted node).
3. rb-engine seq.rs: Track gets the recording list (+0x198) and the hook/merge; both lists reserve the pool's capacity up front (fixed storage, no growth in the callback), and the sequencer refuses a node as the pool does. Sequencer::record_hook, set_record with the merge and chase, the merge at the loop wrap, song end through set_record(0). frame/edit.rs: device_param, mixer_global and mixer_channel call the hook for exactly the controllers the DLL hooks.
4. rb-player: Command::Record(bool), Snapshot.recording; no_alloc test covers recording (hook, wrap merge, record off). Unit tests in seq/tests.rs.
5. Evidence: a recording session in crates/rb-player/tests/live_messages.rs rendered by rbhost --live (one permit) and compared by hash.
6. rb-ui: Link maps msg 35 id 2 to Command::Record and follows Snapshot.recording; the save path gets the recorded tracks from the player (or a follow-up task if it proves too big).
7. Golden test, cargo test --release, clippy, rb compare --manifest (164/164).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a41452d085f0fddbf

RE (IDA copy of ida/ReBirth.dll.i64, headless; llvm-objdump of original/ReBirth.dll for ranges):
- msg 35 jump table: 0x10012555 (msg 35, inner table 0x10151E14), id 2 at 0x100125A7 -> sub_10021190 (falls into 0x10021194). Msg 30 = 0x100124EA -> sub_1001FEB0, msg 31 = 0x10012508 -> sub_100201E0 (both merge every track at trunc(pos) after a range clear sub_1001FFE0; editing messages, not sent by the panel paths rb models).
- sub_10021194(v): if globals byte 2 and v == 0: T = trunc(pos) (fistp at 0x100211E7), sub_1001F520(track, T) for every track (0x10021206); then if song mode, every track chased to T (sub_100151F0 at 0x10021294, +0x228 =, sub_10014660 clears list +0xC, apply ctrl 0..127 != 0xFF with sub_10012B40). Then byte 2 = v (0x10021327).
- Hook sub_10012BC0(song, dev, ctrl, v) -> sub_10021340(seq, ...): needs byte 0 and byte 2 (0x10021352, 0x10021367); pool free count (sub_1000DEE0 = [pool+4]) of 99, 50 or < 10 sends top-level msg 79 to the host (sub_1001D440(79)), a warning only; finds the track with +0x104 == dev; hold byte +4+ctrl = 1 (0x10021402); t = trunc(pos) rounded UP to 24*q(main list +0x108 quant of ctrl) when playing (byte 1, sub_100146E0 = sub_100146E4) or DOWN when stopped (sub_10014720: t/unit*unit); if loop on (byte 3) and t == loop end (+0x10) then t = loop start (+0xC); sub_10014750 into the RECORD list track+0x198 (its own quant, 1 from the list ctor; sub_10021560 / slot 7 sets only +0x108's).
- Merge sub_1001F520(track, T) (retn 4, ecx = track): flags[128] = 0, vals[128] = 0xFF (sub_10014274). Walk the main list while tick <= T: every rec node with tick <= the main node's tick sets flags[ctrl]; vals[m.ctrl] = m.value; a main node whose ctrl is flagged is unlinked and freed (sub_10014B30), unless its tick is 0. Rec nodes up to T set flags too (0x1001F649). For c = 0..127 flagged: t1 = roundup(T+1, 24*q_main[c]) (sub_100146E0); if no main node (t1, c) (sub_10014840), insert (t1, c, vals[c]) (the controller returns to its old automation). Then every rec node with tick <= T goes into the main list (sub_10014750, main quant, same tick+ctrl replaces). Then the whole record list is freed (sub_100149A0), including nodes above T, and the hold mask track+4 cleared (0x1001F767).
- Callers of the merge: loop wrap 0x1001F0C7 (byte 2 set: every track with T = loopEnd - 1, after the apply up to loopEnd - 1, before the wrap), sub_10021194, msgs 30/31, sub_1001FD10 from sub_1001C3C4 (MIDI song position, out of scope).
- Song end 0x1001F484: sub_10021194(0) (so the merge and chase run), host told {2,0},{1,0}, sub_100218A0, locate 0.
- Node pool dword_10100230 (sub_10021A10 -> sub_100142C0): 0x186A0 = 100000 nodes of 12 bytes, one for the process (all songs); sub_10014434 allocates while the free count > 1; frees go back (sub_10014484, sub_100149A0, sub_10014550).
- Stop prune sub_100149F0 is on +0x108 only (0x100218E5).
- Hook call sites: the controller is the message's raw ctrl and the value raw (e.g. 0x10025D3B..0x10025D57). 303 sub 40: ctrl 0..8 (9, 10 and >10 skip it); 808 57: 0..28; 909 74: 0..29 (raw ctrl, the store permutes 20..22); delay 84 (0x1000FD6C), dist 92 (0x10010F66), compressor 102 (0x1000F1CE), PCF 114 (0x10016E8A): every k, also outside the switch; mixer 145 ids 1, 2 (0x1001596C, 0x100159AE); 147 k 1..4 with ctrl 6c+5+k (0x10015A5E..0x10015A75). sub_100261F0 (the 303's MIDI note/CC input) also calls it: out of scope.

Implementation (seq.rs): Track gets the recording list (+0x198), Track::record (hook list part), Track::merge (sub_1001F520); insert_into = sub_10014750 on either list with the pool check of sub_10014434; Sequencer::record_hook, set_record(v, devices) with merge + chase, merge_all at the loop wrap, song end through set_record(0); free_nodes() emulates the pool per song (POOL_NODES = 100000). Storage: both lists of every track reserve POOL_NODES (Vec::with_capacity, address space only; pages touched as used), and the pool check refuses a node before any list can outgrow it, so the callback never grows a Vec (reserve up front, not a separate fixed buffer). edit.rs: hooks(d, ctrl) per the DLL's call sites; device_param, mixer_global (1, 2), mixer_channel (k 1..4) call the hook after the store.
rb-player: Command::Record(bool), Snapshot.recording.

DLL evidence (rbhost --live, one redc-win-run permit, C:\re\rb\wt057; scratchpad task057/run.sh NAME:SONG...; .rblv written by RB_LIVE_OUT=DIR cargo test --release -p rb-player --test live_messages recording; .rbhm from tools/rbhost/rbsmsg.py):
- record_session over full_mix (record while playing over a 2-bar loop with a wrap, 303 pattern and 808 on/off changes rounded to the bar, mixer level and PCF id, effects, record off mid-pass, record on while stopped, locate forward with record on, play, record off, loop off): rbhost bd067446fcb9f5bb193f0bfc7c2cb620f2b28fe004941ec8e88ba7c10ee1bfb8 = rb-player, first run. Control without the Record commands: 6ad5b2fb... both.
- sweep_session over mixer_level_sweep_303 (level moves against the level automation: hold; record off in the same call as a move rounded past T: OB-18; record on/off in pattern mode; song end with record on at bar 999): rbhost e821590917a3cbe3e77a34d7e63fcb4275ea6607495694631e4cf39fe52e5a94 = rb-player; control 4e928260fef75db368f81c18c87399ff6904243c31a2d06ea32e43750d28eea6 both. An earlier sweep without the lost move gave the same e821... in rbhost: the DLL drops the move (OB-18 confirmed by the DLL).
- Mutation checks (temporary env switches, removed): no return events in the merge -> both sessions differ; no hold -> sweep differs; round down while playing -> both differ; keep recorded nodes above T -> sweep differs (363009cb...). So the sessions see the hold, the rounding, the return and OB-18.
- First version of the sweep had its lost move at call 140 where T = 1248 is a multiple of 24 (nothing lost); moved to call 141 (T = 1257).

Track data checked byte for byte against the DLL: song msg 22 {track, u32 from, u32 to} (song handler table 0x10151D6C indexed by msg: msg 22 = 0x10012346 -> sub_10020EC0) replies [78, slot, 37, track, u32 from, u32 to clamped to the list length (sub_10014670), then 7-byte records 99, u32 BE tick, ctrl, value] of the first list (+0x108), written straight to the DLL->Panel port (engine+4, CReBirthDLLPort, sub_1001DED0). (A first try with msg 23 got no reply: 23 is 0x10012363; I had misread the table base, which is 0x10151D6C, msg 18 at 0x10151DB4.) New rbhost option --live-replies FILE (tools/rbhost/rbhost.c) writes every reply of a live session as {u32 call, u32 len, bytes}. The two sessions query all nine tracks at two calls (record: 253 and 599; sweep: 142 and 599): the DLL's 18 payloads equal rb's (track_reply over Track::events()) byte for byte, sha256 e44e1bff... (2199 bytes) and 70392789... (2003 bytes); scratchpad task057/replies.py compares. The WAV hashes did not change with the queries.

UI and save path:
- Exe check (IDA copy of ida/Rebirth.exe.i64): the transport poll sub_42A058 tests the Record press (0x42A1C7) only inside the song-mode block (0x42A074) and sends {2,0} after {0,0} when the mode switch goes to pattern mode, whatever record was (0x42A49B, 0x42A4CC). The export sub_427988 sends no {2,v} (stop at 0x427A02 is the message, not the Stop button). The exe's TRAK writer sub_437CFF reads the tracks back from the DLL with song msg 22 {track, from, from+1284} (0x437DE6) and writes 8*tick/192 (0x437E9E): a saved song carries the DLL's tracks; every shipped song has exactly the ctor's tick-0 events (18, 9, 9, 29, 30, 5, 3, 7, 3 at position 0 in tracks 0..8; scratchpad task057/tick0.py over tests/songs/shipped).
- rb-player: Tracks (a second triple buffer; each list reserves POOL_NODES so the callback's clear + extend_from_slice never allocates), published when (generation, Sequencer::revision) changes; Player::tracks(). Sequencer::revision (msg 37 batch, merge, stop prune) and Sequencer::merges (merges that moved recorded nodes).
- rb-ui: Link::command maps msg 35 id 2 to Command::Record; follow takes Snapshot::recording; Link::recorded_tracks() copies the player's tracks once merges > 0 for the open song (tracks_from = the next generation at load); document::compose writes them (pos = tick/24); App marks the song modified when the copy changes. model.rs: Record does nothing in pattern mode, the switch to pattern mode emits {2,0} after {0,0}. Tests changed to the exe's behavior: keyboard.rs (Insert in pattern mode does nothing), wired.rs (mode switch logs {0,0},{2,0}). New test menu.rs a_recorded_song_saves_its_tracks: record knob moves while playing, record off, save, reopen (tracks equal as per-tick sets; a reload reverses a tick's order, as the original's load does), save again = same bytes, and rb render of the saved file = render of the source with the live engine's tracks, != the source's render.
- Decision not taken: always saving the engine's tracks (as the exe does) would break the byte-identical unedited save of task-047; filed task-080 (Decide, @pingw33n) with option A (task-057's rule: the file's tracks unless recorded) recommended. task-057 implements A, the existing behavior plus the recording case, so it does not wait on it.
- Follow-up found: the exe ignores Rewind/FF/Bar/Loop arrows in pattern mode (same block); rb-ui's Panel::arrow does not: task-081 (@agent-worker).

Acceptance:
- AC1: docs/initial_investigation.md 5 (hold mask, song end, the two lists and the node pool, msg 35 id 2, msg 22/30/31 rows) and the new 5.3 Recording (hook, merge, when it merges, DLL check), all with addresses; the section's record-related statements are now [C] from the disassembly and the DLL runs; docs/song_load.md 4.10 and 11, docs/export_state.md 4 (Record), docs/ui_spec.md 3 and 16, docs/original_bugs.md OB-18 [C, confirmed by rbhost] and OB-19 [C code, G effect].
- AC2: rb-engine records as the DLL (two rbhost --live sessions equal byte for byte in audio and in the nine tracks queried by msg 22 at two points each); rb-player Command::Record, Snapshot::recording, Player::tracks; unit tests seq/tests.rs (7 new), frame/tests.rs panel_messages_record; no_alloc.rs records 300 calls of knob/909/mixer messages over two loop wraps and record off with 0 allocator events, and checks the published track copy.
Tests after the rebase onto main 8b9142f: cargo test --release 428 passed 0 failed (RB_STANDARD_MOD, RB_MOD_DIR set), debug cargo test 424 passed, golden test passed, rb compare --manifest --standard-mod target/Standard.rbm 175 of 175 EQUAL (175 entries on this main), clippy -D warnings clean, fmt clean. The diff against main lists only this task's files. Remote folder C:\re\rb\wt057 deleted; permits released; IDA copies closed with save=false.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Recording is modelled after the DLL and wired end to end.

RE (docs/initial_investigation.md 5.3, song_load.md 4.10, export_state.md 4, ui_spec.md 3/16, original_bugs.md OB-18/19): record byte 2 via sub_10021194; the hook sub_10012BC0 -> sub_10021340 (song mode and record only; hold byte; trunc(pos) rounded up while playing, down when stopped, to the first list's unit; loop end -> loop start; into the recording list track+0x198); the merge sub_1001F520 (the recorded span replaces the controller's events up to T except tick 0, the controller returns to its old value at the next unit, recorded nodes above T are lost = OB-18, the applied pointer can dangle = OB-19); merges at the loop wrap (T = loopEnd-1), at record off (then a chase in song mode) and at the song end; the process-wide 100000-node pool (sub_100142C0, sub_10014434). The exe: Record works only in song mode, pattern mode sends {2,0} after {0,0}, the export sends no record message, the save writes the DLL's tracks (sub_437CFF, song msg 22).

Code: crates/rb-engine/src/seq.rs (recording list, record, merge, per-song pool; both lists reserve a whole pool up front so the callback never grows them; Track's Clone keeps the reservation), frame/edit.rs (hooks per the DLL's call sites), frame/mod.rs (Engine::set_record, recording); rb-player Command::Record, Snapshot::recording, Tracks and Player::tracks (a second triple buffer); rb-ui Link maps id 2, follows the engine's byte, saves recorded tracks, marks the song modified; model.rs Record only in song mode and {2,0} on pattern mode; tools/rbhost --live-replies.

Evidence: rbhost --live sessions record (full_mix) and sweep (mixer_level_sweep_303) equal rb-player's WAV sha256 and the DLL's msg-22 track dumps byte for byte; mutation checks show the sessions see the hold, the rounding, the return events and OB-18. rb-ui test: a recorded song saves, reopens with the same tracks, saves back identically and renders as the recorded engine state.

Tests: cargo test --release 428 pass, debug 424 pass, golden green, corpus compare 175/175 EQUAL, clippy clean.

Branch worktree-agent-a41452d085f0fddbf, one commit. Follow-ups: task-080 (Decide, @pingw33n: always save the engine's tracks as the exe does; recommended no), task-081 (rb-ui: wind, bar and loop arrows inert in pattern mode).
<!-- SECTION:FINAL_SUMMARY:END -->

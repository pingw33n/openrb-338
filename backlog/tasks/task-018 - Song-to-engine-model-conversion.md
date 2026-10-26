---
id: TASK-018
title: Song to engine model conversion
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 16:39'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-017
  - TASK-003
priority: high
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Convert the rbs model into the engine's initial state and event stream exactly as Rebirth.exe does at song load and export (docs/song_load.md from m-0): global parameters (tempo BPM*1000, loop start/end in ticks, shuffle, the sound model flag), the devices' internal pattern formats (303 98-byte patterns with the 6-byte steps, 808/909 194-byte patterns with their step values), the mixer and effect parameter bytes, and the track events at 192 PPQ. Unit tests against the mappings in docs/song_load.md, plus a test comparing the converted message stream with rbhost's log of the messages it sends for the same song (if rbhost can log them; otherwise against hand-checked values for three songs).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every mapping of docs/song_load.md is implemented and unit-tested
- [x] #2 The conversion of at least three corpus songs equals the original's messages (rbhost log or hand-checked values in the test)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. IDA (Rebirth.exe, read-only copy): trace what docs/song_load.md leaves [G] for the conversion: the 4.0 mixer-track converter sub_437380, the 4.0 MIXR converter (0x4352C0, file layout), the version tests (CVersion compare: equality vs >=, revision byte), the 808 old-track tom tune order against the range check (sub_436F5A).
2. rbs crate: expose HEAD revision (Head.revision) and the MIXR body bytes (Mixer::bytes) the 4.0 converter needs; nothing else changes.
3. rb-engine: module song (the engine-side model: Globals = msg 36, Mixer, Delay, Pcf, Distortion, Compressor, Tb303 with 98-byte patterns/6-byte steps, Tr808/Tr909 with 194-byte patterns, track events in engine ticks, mod name, vintage byte), each with its msg payload encoder; module load (rbs::Song -> model, the msg-37 batches with the exe's validation, and the full message stream of song_load.md section 2 step 5-6).
4. Unit tests per mapping of docs/song_load.md (globals bytes, vintage by version, mixer 4.0/4.1/4.2, effect tails, 303 step flags, 808 tom tune and 0/1 steps, 909 MidTomDecay quirk and raw steps, x24 ticks, tick validation not advancing, ranges per track, 808 old-track tune before range check, 4.0 mixer track converter, batching 1284 with first/last and empty batches).
5. Corpus test: the Rust message stream equals tools/rbhost/rbsmsg.py load_messages for every song in tests/songs/diag/songs and tests/songs/shipped (8 mod songs included).
6. docs/song_load.md: the traced facts with addresses.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a46f7ae008756d9e1

IDA findings (Rebirth.exe.i64, read-only copy in the scratchpad, closed with save=false):
- sub_437380, the 4.0 mixer-track converter, checked in the disassembly (switch at 0x43764E, jump table 0x437C73). It remaps the controller before the validation. Old 4+8c -> 5+6c; old 5..8+8c -> 6..9+6c (level, pan, send, dist); old 9+8c (PCF flag) -> ctrl 2 with value c+1; old 10+8c (comp flag) -> ctrl 1 with value c+2; old 1 (master comp flag) -> ctrl 1 with value 1; old 0 -> 0; 2, 3, 11, 19, 27 and >34 are unchanged. A flag of value 0 becomes value 0 only if its tick is > the tick of the last comp event (var_18), resp. PCF event (var_14), both 0 at the start, compared signed (jg); otherwise the event is dropped. The trackers are updated in the switch, before the tick validation sub_42FD60 (0x437A32), so an event dropped later still updates them. Then the checks of sub_436B6F follow (0x437A28..0x437A8D); the running position advances on a valid tick even if the event is dropped.
- sub_436F5A, the 808 track before 4.2: v/2+32 (sub_432E1B, called at 0x437139) on ctrls 10, 13, 16 is applied BEFORE the range check (sub_42FD00 at 0x4371A3 is passed the tuned byte var_2377). So a file value 190 becomes 127 and is accepted, 192 becomes 128 and is dropped.
- The 4.0 MIXR converter (0x4352C0..0x43561B): sub_433B86 copies raw[0], raw[1] and four 7-byte channels at raw[16+12c]. The defaults of sub_433BEA are master 100, comp 0, PCF 0. Then master = raw[0], comp id = (raw[1] != 0); per channel, enabled = 1, level/pan/send/dist = ch[1..4], ch[5] != 0 -> PCF id c+1, ch[6] != 0 -> comp id c+2 (the last flagged channel wins). The result is packed by sub_438D43.
- The version tests: CVersion (sub_47D768) compares the key major<<16 | minor<<8 | revision (sub_47D9ED; HEAD bytes 5..7). sub_47DA82 tests EQUALITY: the 4.0 MIXR converter and sub_437380 run only for exactly 4.0.0 (0x4DB938 = CVersion(4,0), built by sub_43295F); the 4.1 enable forcing (0x435708) runs only for exactly 4.1.0 (0x4DB928, sub_4329A2). sub_47DAB5 tests key >= 4.2.0 and major <= 4 (0x4DB918, sub_4329E5); the GLOB vintage byte (0x43405E), the 808 chunk (0x432D95) and the 808 track (0x4366F6) use it. So a 4.1.1 file gets neither the 4.1 enable forcing nor the 4.2 paths. tools/rbhost/rbsmsg.py compares (major, minor) only; it agrees on every corpus song, whose revisions are 0.

Implementation:
- crates/rbs: Head.revision (HEAD byte 7, written back) and Mixer::bytes() (the 64-byte MIXR body, needed for the 4.0 channel flag bytes). No other change; rbs tests still pass.
- crates/rb-engine/src/song.rs is the engine-side model. Globals (msg 36, every byte named, payload()); Mixer/MixerChannel; Delay, Pcf, Distortion, Compressor as params blocks with index consts (controller c sets params[c]; constant tail bytes kept, comp 127, 0); Tb303 (12-byte params, 32 x Pattern303 of 98 bytes with 6-byte Step303); DrumMachine<P> = Tr808 (P=32) / Tr909 (P=33) with 194-byte DrumPattern; TrackEvent {tick, ctrl, value}; Song {mod_name, globals, devices, tracks: [Vec<TrackEvent>; 9]}; Device enum (id, msg-41 sub, data length).
- crates/rb-engine/src/load.rs: load(&rbs::Song) -> Loaded {song, batches}, convert(), load_messages(song, slot), track_batches(track, events, TrackReader::{Plain, Old808, Mixer40}), control_range(track, ctrl), the CVersion semantics (equality for 4.0.0 / 4.1.0, >= for 4.2).
- The track deltas are recomputed from the rbs model's absolute positions (pos_i - pos_(i-1)); the exe's u32 arithmetic is kept (abs wrapping_add, (abs*192 mod 2^32) >> 3). The rbs reader rejects a position sum above u32 (checked_add), where the exe would wrap; no real file gets near that.

Coverage of the corpus (scratchpad cov.py over rbsmsg.load_messages): 12 songs have tracks with more than 1284 events (several msg-37 batches); 1 song drops events (Alloy Cowboy Resurrection, 19 events at or beyond position 32000).

Tests: cargo test 34 pass (24 rb-engine unit tests, 1 rb-engine corpus test, 5 + 4 rbs tests). cargo clippy --all-targets -D warnings and cargo fmt --check are clean.
Corpus comparison (crates/rb-engine/tests/rbsmsg_corpus.rs: /usr/bin/python3 -I runs tools/rbhost/rbsmsg.py on every song): "163 songs equal (155 .rbhm files, 8 mod songs' message lists); 3292 messages, 97899 track events". For the 155 songs with the built-in mod the whole .rbhm file of rbsmsg.message_file is compared byte for byte; for the 8 mod songs (rbsmsg refuses them in message_file) it is the list of rbsmsg.load_messages. Every song has revision 0.
Reproduce: cargo test -p rb-engine --test rbsmsg_corpus -- --nocapture

Acceptance criteria:
#1 Each mapping of docs/song_load.md is implemented in crates/rb-engine/src/load.rs, with a unit test in crates/rb-engine/src/load/tests.rs or song.rs. The msg 36 globals (globals_from_glob, globals_payload_layout); the vintage byte by version; the major-4 check (version_major_must_be_4); the mixer for 4.2, 4.1.0 and 4.0.0 and the revision semantics (mixer_4_2_as_stored, mixer_4_1_enables_forced, mixer_4_0_converter); the delay/PCF/dist/comp tails (effect_blocks); the 303 steps and params (tb303_block); the 808 tom tune and 0/1 steps (tr808_block); the 909 MidTomDecay quirk and raw steps (tr909_block); the range table of section 5 (ranges); tick = pos*24 (ticks_are_pos_times_24); invalid ticks not advancing (invalid_tick_dropped_without_advancing); the u32 wrap (tick_wraps_in_u32); controller and range drops (controller_and_range_checks); the old 808 track (old_808_tom_tune_before_range_check); the 4.0 mixer track (mixer_4_0_track_converter, mixer_4_0_trackers_see_dropped_ticks); the 1284-event batches (batches_of_1284_file_events, batch_payload); the message order (message_stream, payload_lengths).
#2 The conversion equals the validated message stream of tools/rbhost/rbsmsg.py for all 163 songs, not just three: 155 whole .rbhm files plus the message lists of the 8 mod songs (crates/rb-engine/tests/rbsmsg_corpus.rs).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The song-to-engine conversion is in rb-engine, on branch worktree-agent-a46f7ae008756d9e1 (one commit).
- crates/rb-engine/src/song.rs is the engine-side model the later tasks build on: the globals of msg 36 (with the Vintage byte), the mixer, the four effect parameter blocks, two TB-303s with 98-byte patterns and 6-byte steps, the TR-808 and TR-909 with 194-byte patterns, and the track events in engine ticks (192 PPQ). Each block keeps the DLL's own layout and has its message payload.
- crates/rb-engine/src/load.rs converts an rbs::Song the way Rebirth.exe does and gives the whole load message stream (msg 36, the nine msg 41 blocks, the msg-37 batches, msg 32).
- It also covers the paths of 4.0 files: the 4.0 mixer chunk and the 4.0 mixer track (sub_437380). These were traced in IDA in this task.
- crates/rbs gains Head.revision and Mixer::bytes().
- docs/song_load.md has the new facts: the exe's version tests (equality for 4.0.0/4.1.0, >= for 4.2), the 4.0 MIXR parse, the 808 old-track tune order, and section 5.1 on sub_437380.
Result: the Rust stream equals tools/rbhost/rbsmsg.py's for all 163 shipped and diagnostic songs (155 whole .rbhm files, and the message lists of the 8 mod songs). That is 3292 messages and 97899 track events.
Tests: cargo test 34 pass (24 new unit tests and 1 new corpus test); clippy and fmt are clean. There is no audio change, so no render comparison applies.
Not tested: no 4.0 song exists, so the 4.0 paths are unit-tested against the disassembly only.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-055
title: 'RE + engine: the panel''s live device messages (msg 41 sub ids)'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 07:34'
updated_date: '2026-10-27 13:28'
labels:
  - re
  - engine
milestone: m-3
dependencies:
  - TASK-043
references:
  - docs/ui_spec.md
  - docs/song_load.md
  - crates/rb-player/src/lib.rs
priority: high
type: feature
ordinal: 50000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the panel (docs/ui_spec.md) changes the engine with device messages (song msg 41 with a device's own sub ids), not only with the controllers of slot 11 and the msg-41 data block. rb models only those two (Engine::set_param, Engine::device_data) plus the mixer's master and mute (MixerDevice::set_master, set_enabled). The real-time player (task-043, crates/rb-player) exposes Command::Param, Master, ChannelEnabled and DeviceData; a pattern edit can be sent only as a whole data block, which in pattern mode also resets the device's live parameters to the block's. task-045 (pattern editing) needs the original's edit messages.

What is already read (task-043 notes, headless ReBirth.dll.i64):
- TB-303 handler sub_100256E0, jump table flt_10153BD8 + 4*sub (subs 31..46, entries from 0x10153C54). Sub 40 0x10025CFC {ctrl, v}: ctrl 0..8 store the live byte +0xC7C+ctrl, then the record hook sub_10012BC0 -> sub_10021340 (acts only with song mode and record on), ctrl 9 nothing, ctrl 10 stores +0xC86; then clears the changed flag +0xC88+ctrl. Sub 45 0x10025FC4 {pattern, step, field 0..5, v}: byte +0x3E + 98*pattern + 6*step + field. Sub 42 0x10025F25 {pattern, 0|1, v}: +0x3C or +0x3D + 98*pattern (the pattern's first two bytes). Not read: subs 43 (sub_100191E0), 44 (sub_10018DC0), 46 (sub_10019B00), and the queries 31..38.
- Mixer handler sub_10015520, table at 0x10151ECC + 4*sub (subs 137..148 from 0x101520F0): 144 the block into the pattern copy only; 145 {id, v}: 0 master, 1 comp id, 2 PCF id; 147 {c, k, v}: k 0 enabled, 1..4 level, pan, send, dist (the record hook on 5+6c+k).
- To read: the TR-808 (data sub 56, handler around 0x1002884C), TR-909 (73), delay (86, 0x1000FB40), distortion (94), PCF (116), compressor (104) handlers; the exe's senders (ui_spec.md: sub_43EC83 303 knobs, sub_43ED1D 303 steps, sub_441C82 808, and the 909, effect and pattern-section polls).

Do: list every device message the panel sends (exe side) with its DLL handler and effect, with addresses, in docs/song_load.md (a new section on the live messages) or a new docs file; implement each in rb-engine as an Engine entry point (numerics untouched: these are byte stores); add the matching rb_player::Command variants; unit tests per message. Messages that only query or reply need no command.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every device message the panel sends (docs/ui_spec.md) is listed with its exe sender and DLL handler, with addresses, in docs/
- [x] #2 Each message that changes engine state has an rb-engine entry point and an rb_player::Command, with unit tests
- [x] #3 cargo test --release passes, including the corpus golden test and crates/rb-player/tests/no_alloc.rs
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE (headless copies of both .i64): the exe's senders of msg 41 (sub_40B28C -> sub_423C98 builds {78, slot, 41, dev, sub, payload}; the queries go through sub_40B23A -> sub_423C51), each DLL handler's sub ids (jump tables), every store with its offset, bounds checks and record-hook calls. Disassembly first.
2. docs/song_load.md: replace 4.10 with the full list (exe sender, DLL handler, effect, record hook, queries and their side effect); fix ui_spec.md where it is wrong.
3. rb-engine: Engine entry points per message kind: device_param ({ctrl,v}: 303 40, 808 57, 909 74, delay 84, dist 92, comp 102, PCF 114), mixer_global (145), mixer_channel (147), pattern_field (42/59/76), pattern_header (43/60/77), pattern (44/61/78), step_field (45/62/79), step (46/63/80). The pattern messages write the device's pattern array through a new ChannelDevice method; byte stores only, numerics untouched. Unit tests per message.
4. rb-player: Command variants for each (Pattern carries a Box returned through the garbage queue; Step an inline [u8;12]); length checks in send; no_alloc test extended with every new command.
5. Evidence: rbhost --live FILE (scheduled messages before 1024-frame calls) driven on the test machine with a scripted session over a corpus song; the same session through rb-player; sha256 of both WAVs equal; the hash goes into an rb-player test.
6. cargo test --release (golden, no_alloc), clippy, fmt; rebase; one commit.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-affa29aa41ac27c0f

Branch: worktree-agent-affa29aa41ac27c0f

RE (headless copies, read-only, closed save=false):
- Exe: every panel edit goes through sub_40B28C(slot, dev, sub, data, n) -> sub_423C98, which builds {78, slot, 41, dev, sub, data...}; the queries through sub_40B23A -> sub_423C51 ({78, slot, 41, dev, sub}). Senders: 303 sub_43EC83 (40 {ctrl,v}), sub_43ECCA (42 {pat,field,v}), sub_43F617 (44 {pat, 98 bytes}, from Clear sub_43F5F5 and the edit functions sub_42B319/sub_42B928/sub_42C204), sub_43ED1D (45 {pat,step,field,v}); 808 sub_441C82 (57), sub_441CC9 (59), sub_443257 (61, 194 bytes), sub_441D1C (62 {pat,step,inst,v}, from sub_44305C); 909 sub_445C41 (74), sub_445C88 (76), sub_4471C5 (78), sub_445CDB (79, from sub_447153); comp sub_403FE5 (102), delay sub_407AD3 (84), dist sub_408976 (92), PCF sub_41B82B (114), mixer sub_4131BC (145 {id,v}), sub_413206 (147 {c,k,v}). Queries: 303 31 (sub_43EAC5) / 33 (sub_43EA26), 808 sub_442978/sub_4428D9, 909 sub_446AD7/sub_446A38, from sub_42D92C / sub_42D9CF; the effects' and mixer's views sub_403DC1, sub_407944, sub_408736, sub_41B6AA, sub_412F04 (137); the save path sub_438EAA (delay 83), sub_43920E (303 31) and siblings.
- DLL 303 sub_100256E0, table 0x10153BD8+4*sub (31 0x10025751 .. 46 0x10026112): 40 0x10025CFC ctrl 0..8 -> +0xC7C+ctrl + record hook sub_10012BC0, 9 nothing, 10 -> +0xC86, all: clear +0xC88+ctrl; 41 nothing; 42 0x10025F25 field 0 -> +0x3C+98p (0x10026162), 1 -> +0x3D+98p, else nothing; 43 0x10025F62 sub_100191E0 (2 bytes) at +0x3C+98p; 44 0x10025F93 sub_10018DC0 (98 bytes) at +0x3C+98p; 45 0x10025FC4 field 0..5 (switch, ja default) -> +0x3E+98p+6s+field; 46 0x10026112 sub_10019B00 (6 bytes) at +0x3E+98p+6s. No bound on p or s.
- 808 sub_100282B0, table 0x10153F74+4*sub (48 0x10028321 .. 63 0x10028FE2): 57 0x100288C5 ctrl 0..28 -> +0x1890+ctrl + record hook, 29 nothing, 30 -> +0x18AE, clear +0x18B0+ctrl; 58 nothing; 59 0x10028EFE 0 -> +0x50+194p, 1 -> +0x51+194p; 60 0x10028F3B sub_100191E0 at +0x50+194p; 61 0x10028F6C sub_100192C0 (194 bytes); 62 0x10028F9D +0x52+194p+12s+inst (inst unchecked); 63 0x10028FE2 sub_10019840 (12 bytes) at +0x52+194p+12s.
- 909 sub_1002B8E0, table 0x101542F0+4*sub (65 0x1002B951 .. 80 0x1002C663): 74 0x1002BF12 ctrl 0..29 -> +0x1891+ctrl with 20,21,22 -> +0x18A6,+0x18A7,+0x18A5 + record hook, 30 nothing, 31 -> +0x18B0, clear +0x18B2+ctrl; 75 nothing; 76 0x1002C57F +0x51/+0x52+194p; 77 0x1002C5BC; 78 0x1002C5ED; 79 0x1002C61E +0x53+194p+12s+inst; 80 0x1002C663.
- Delay sub_1000FB40 (81 0x1000FBA9, 82 0x1000FC1F, 83 0x1000FC66, 84 0x1000FCE5, 85 0x1000FD7B nothing, 86 0x1000FD7D): 84 k 0..4 -> +0x34+k, 5 nothing; clear +0x40+k; record hook for every k. Dist sub_10010D60 table 0x10151A34 (92 0x10010EF1: k 0..2 -> +0x39+k, clear +0x4C+k, hook). PCF sub_10016C50 table 0x101520F4 (114 0x10016DFB: k 0..6 -> +0x38+k, hook, clear +0x48+k). Comp sub_1000EFD0 table 0x1015151C (102 0x1000F178: k 0..2 -> +0x38+k, clear +0x42+k, hook). The messages store raw (slot 11 of delay/dist is the same store; rb's clamp to 127 is its own guard, equal for 0..127).
- Mixer sub_10015520 table 0x10151ECC (137 0x10015589 .. 148 0x10015B8B): 145 0x10015900 id 0 -> +0x30 (no hook), 1 -> dword +0x34 (clear +0x81, hook ctrl 1), 2 -> dword +0x38 (clear +0x82, hook ctrl 2), 3,4 nothing; 147 0x100159DB k 0 -> +0x40+6c (no hook), k 1..4 -> +0x40+6c+k, hook ctrl 5+6c+k, k 5 nothing, c unchecked; 146 0x100159C2 header into the live +0x30 (sub_10019D10), 148 0x10015B8B {c, 6 bytes} into the live channel (sub_10019B00): not sent by the exe.
- Queries: the whole-block query of each device (303 31 0x10025751, 808 48 0x10028321, 909 65 0x1002B951, delay 83 0x1000FC66, dist 91 0x10010E7C, PCF 113 0x10016D80, comp 101 0x1000F0FD) first copies the live block into the pattern-mode copy when in pattern mode (sub_1001FCB0 == 0), then replies. Unobservable in the output: in pattern mode nothing reads the pattern copy's parameters (slot 9 overwrites it from live on entering song mode; msg 41 overwrites it), so the queries need no command. Mixer 137 replies the live set without copying.

Evidence against the original DLL (2026-10-27): rbhost got --live FILE (tools/rbhost/rbhost.c live_session: before 1024-frame call i it writes call i's messages to the port and pump()s them, as SafeSoundCallback drains the panel's messages before it renders). crates/rb-player/tests/live_messages.rs builds a session over diag/full_mix (pattern mode, play, 135 messages: 303 40/42/43-less/44/45/46, 808 57/59/61/62, 909 74/76/77/79/80, delay 84, dist 92, PCF 114, comp 102, mixer 145/147, song mode while playing with more 40/74/145/147, stop, play), 600 calls, idle 0; RB_LIVE_OUT writes the .rblv. rbhost.exe (build sha256 111187a2...) --private-port --live live.rblv full_mix.rbhm (rbsmsg.py) on the test machine, one permit, no Rebirth.exe running (tasklist), folder C:\re\rb\wt055 deleted after: sha256 960e1218589577b8841ac65e77764d4950ad6614b4ce692c3cf04e392559071d, the player's WAV the same: byte-identical (2457644 bytes).
Failed first attempt: the session also returned to pattern mode at call 400; output identical up to call 400, then differed (first frame 409606). Bisected with variants (each through rbhost): no mode switches with all messages: equal; mode switches without any message: equal; per device: only the PCF; per PCF controller: only the wave (ctrl 4 = 30); the same wave sent as a data block (sub 116) instead of msg 114: the same difference (first frame 409604). So it is not a message: rb's PCF step after a return to pattern mode while playing differs from the DLL (pre-existing). Filed task-069 (bug, @agent-complex), with the ignored repro test pcf_wave_after_returning_to_pattern_mode (rbhost sha256 4a50f82a..., rb ce9fdb76..., first frame 409602 449 vs 448). The committed session stays in song mode after call 300.

AC evidence:
#1 docs/song_load.md 4.10 rewritten: every msg 41 the panel sends (303 40/42/44/45, 808 57/59/61/62, 909 74/76/78/79, delay 84, dist 92, comp 102, PCF 114, mixer 145/147) with its exe sender, callers, DLL handler, table and address, the effect with offsets, the record hook, the bounds; the unsent state-changing subs (43/46, 60/63, 77/80, mixer 146/148) and the queries with their side effect. docs/ui_spec.md sections 4, 5, 6, 7, 9 point to the message ids.
#2 rb-engine crates/rb-engine/src/frame/edit.rs: Engine::device_param, mixer_global, mixer_channel, pattern_field, pattern_header, pattern, step_field, step (+ SongDevice::param_message, ChannelDevice::patterns_mut, PatternShape); unit tests frame::tests::device_param_messages, mixer_messages, pattern_messages_303, pattern_messages_drums, a_step_edit_is_heard. rb_player::Command DeviceParam, MixerGlobal, MixerChannel, PatternField, PatternHeader, Pattern (box via garbage, length checked in send), StepField, Step; tests live_messages::the_session_equals_rbhost (byte-identical to the DLL) and no_alloc extended with all of them.
#3 cargo test --release after the rebase onto main 2cd859e: 286 passed, 0 failed, 1 ignored (task-069's repro); corpus_golden 4 passed, no_alloc 3 passed, live_messages 1 passed. rb compare --set diag,app,shipped: 164 of 164 equal. cargo clippy --all-targets -D warnings and cargo fmt clean. The diff against main lists only this task's files.

Review round 1 (send back: docs and one tool fix), fixed:
(1) docs/original_bugs.md OB-17 (section 2): the unbounded stores of 303 subs 42/45 (0x10025F25, 0x10025FC4), 808 59/62 (0x10028EFE, 0x10028F9D), 909 76/79 (0x1002C57F, 0x1002C61E), mixer 147 (0x100159DB, channel 4 at +0x58); linked from the Bounds bullet of song_load.md 4.10.
(2) song_load.md 4.10 'rb:' paragraph states rb's divergences: delay pan and dist amount/shape clamped to 127 (delay.rs:74-76, dist.rs:116-118) where subs 84/92 store raw, and mixer channel above 3 dropped (edit.rs). The clamp is also on the load path, and a song file can carry such a byte: re-read in the headless Rebirth.exe copy, the DELY/DIST parses sub_43465C (called at 0x43589A in sub_434D05) and sub_4345A6 (0x435B28) are plain byte copies (disassembly), and the packers sub_422EB7 / sub_423048 copy the bytes unchanged; no check between the chunk read sub_49F9AE and the send sub_438F22. OB-9 and OB-10 updated with it. That is a render divergence for hand-made files only, already a deliberate deviation: filed Decide task-071 (@pingw33n, recommendation keep the clamp).
(3) tools/rbhost/rbhost.c live_session checks p + 8 <= size before reading a record header (and the length check in 64 bits). Rebuilt (sha256 0507a6f0...), reran both sessions on the test machine (one permit, folder deleted): live 960e1218... byte-identical to rb-player as before, pcf 4a50f82a... as before.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The panel's live device messages are read and modelled (branch worktree-agent-affa29aa41ac27c0f, one commit).

RE (headless IDA copies, disassembly): the exe sends every edit through sub_40B28C -> sub_423C98 ({78, song, 41, dev, sub, data}); the senders are 303 40/42/44/45, 808 57/59/61/62, 909 74/76/78/79, delay 84, dist 92, comp 102, PCF 114 and mixer 145/147. Each DLL handler (tables 0x10153BD8, 0x10153F74, 0x101542F0, 0x101516A8, 0x10151A34, 0x1015151C, 0x101520F4, 0x10151ECC) does byte stores: the {ctrl,v} subs are slot 11's store plus one unused live byte (303 10, 808 30, 909 31) and the record hook; the pattern subs write the pattern array at base + len*p + 2 + step*s + field, read live by the step functions; only the 303's step field and the header field are bounded, a new defect OB-17. The whole-block queries copy live into the pattern copy in pattern mode, which nothing observes. All in docs/song_load.md 4.10 with rb's divergences (delay pan / dist clamps, also reachable from a song file: Decide task-071; mixer channel above 3 dropped); ui_spec.md points to it.

rb: rb-engine frame/edit.rs has one Engine entry point per message shape (device_param, mixer_global, mixer_channel, pattern_field, pattern_header, pattern, step_field, step), with no numerics; rb-player has the matching Commands (the whole pattern's box returns through the garbage queue). Recording stays task-057: the messages act as with record off.

Evidence: rbhost --live (new) ran a 600-call session over full_mix with 135 of these messages, in pattern mode and in song mode while playing, through ReBirth.dll on the test machine. Its WAV is byte-identical to rb-player's (sha256 960e1218...071d, pinned in crates/rb-player/tests/live_messages.rs). A session returning to pattern mode differed from call 400 on; bisecting through rbhost traced it to rb's PCF wave step (also with a data block), filed as task-069 with an ignored repro test.

Tests: cargo test --release 286 passed, 1 ignored (task-069); corpus golden green, rb compare 164/164 equal; no_alloc passes with the new commands; clippy and fmt clean. Review round 1 fixed (OB-17, divergences, rbhost bounds check).
<!-- SECTION:FINAL_SUMMARY:END -->

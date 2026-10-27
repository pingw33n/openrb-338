---
id: TASK-045
title: 'UI: wire the panel to the real-time player, pattern editing'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 16:23'
updated_date: '2026-10-27 14:01'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-043
  - TASK-044
  - TASK-054
  - TASK-055
priority: high
type: feature
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Wire the panel to the real-time player: every knob, slider and switch drives its engine parameter live; transport and tempo/shuffle; pattern and bank selection with the original's timing (pattern changes at the bar line); the 303 step editor and the 808/909 step buttons and instrument selector edit patterns live; the pattern/song mode switch, song position and loop displays follow playback; LEDs show the current step. Behavior per docs/ui_spec.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every control in docs/ui_spec.md drives its engine parameter live, checked against a list in the notes
- [x] #2 Pattern editing of the 303s and drums works as the spec says, and edits are heard on the next pass
- [x] #3 Pattern and song mode play shipped songs; positions and step LEDs follow playback
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rb-ui link.rs: model events -> the original panel's messages (Msg: msg 35 {id,v}, msg 41 device subs of song_load 4.10) -> rb_player Commands; the player snapshot -> panel (transport, live values, patterns, running lights, meters). In-flight rule: values from a snapshot only if it is newer than the last commands sent.
2. Model: Clear303 event, stop-when-stopped locate event, loop_start/loop_end events, adopt() for the engine's values, held (dragged) control.
3. session.rs + main.rs: open a song and the standard-mod kit; App::with_link.
4. Headless tests (tests/wired.rs): real Player, the test drives the audio callback; messages compared with section 4.10; engine live block and pattern arrays compared; shipped songs played.
5. docs/ui_spec.md section 16.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-27, per task-054 (decided): keys by physical US position (scancode) per the manual's pictures, Note/Pause and drum AC on the key right of '0', '=' as an alias; numpad 4/5 rewind/fast forward as the program does (docs/ui_spec.md section 10).

Branch: worktree-agent-a94cd3c62858c4a95

Design: crates/rb-ui/src/link.rs. Out: Panel::events -> Msg (the original's wire messages, logged in Link::log) -> Command. In: after each engine call the Snapshot moves the panel (transport, every controller's live value, mixer, selected pattern per section, drum running lights, meters). The engine's values are taken only from a snapshot newer than the last commands sent (Link::mark), not while a control is dragged (Panel::held), not while a loaded song waits for Play (snapshot.pending): the panel shows the newest song, as the original's does. The pattern arrays are kept in Link (from the Song's device payloads and the edits) to fill the 303 editor, the drum strips and the length/shuffle displays when a pattern is selected (the original's views keep a current pattern; edits go to the selected pattern, not the playing one).

Model changes (crates/rb-ui/src/model.rs): Event::Clear303 replaces 96 StepField events (the original sends the whole pattern, sub 44); Stop while stopped emits the locate (Set transport.position) instead of Press; loop start up/down emits Set loop_start then Set loop_end ({12} then {16}); loop length emits Set loop_end; Panel::adopt()/adopt_pattern() take the engine's values without sending them back; Panel::held. rb-player: Audio::current_mut() (tests, and Link::new reads the new song's pattern arrays). Existing rb-ui tests unchanged and pass.

Control list checked (AC #1), tests/wired.rs every_knob_fader_and_switch_sends_its_device_message_and_the_engine_takes_it drives each control by a pointer gesture, compares Link::log with the section 4.10 message, then runs engine calls and reads the live block of the snapshot (909 through tr909::internal_index). Every one of these passed (id -> dev sub [ctrl, value] of the run): 303 #1 and #2 (dev 1, 2; sub 40): wave 8, tune 2, cutoff 3, reso 4, envmod 5, decay 6, accent 7. 808 (dev 3; sub 57): ac_level 2, bd_level 3, bd_tone 4, bd_decay 5, sd_level 6, sd_tone 7, sd_snappy 8, lt_level 9, lt_tuning 10, slot3_switch 11, mt_level 12, mt_tuning 13, slot4_switch 14, ht_level 15, ht_tuning 16, slot5_switch 17, rs_level 18, slot6_switch 19, cp_level 20, slot7_switch 21, cb_level 22, cy_level 23, cy_tone 24, cy_decay 25, oh_level 26, oh_decay 27, ch_level 28. 909 (dev 4; sub 74): ac_level 2, bd_level 3, bd_tune 4, bd_attack 5, bd_decay 6, sd_level 7, sd_tune 8, sd_tone 9, sd_snappy 10, lt_level 11, lt_tune 12, lt_decay 13, mt_level 14, mt_tune 15, mt_decay 16, ht_level 17, ht_tune 18, ht_decay 19, rs_level 20, cp_level 21, hh_level 22, ch_decay 23, oh_decay 24, cc_level 25, cc_tune 26, rc_level 27, rc_tune 28, flam 29 (flam in the_flam_knob_... test). Delay (dev 5; sub 84): on 0, steps 1 (arrows), triplet 2, feedback 3, pan 4. Dist (6; 92): on 0, amount 1, shape 2. PCF (7; 114): on 0, freq 1, q 2, amt 3, pattern 4 (arrows), dec 5, mode 6. Comp (8; 102): on 0, ratio 1, threshold 2. Mixer (dev 0): master fader 145 {0,v}; Comp/PCF radios 145 {1,c+2|1|0}, {2,c+1|0}; per channel 147 {c,k,v}: mute 0, level 1, pan 2, delay 3, dist 4. Pattern sections (dev 1..4; subs 40 / 42,59,76): on {0,v}, pattern button {1,p}, shuffle {p,0,v}, steps {p,1,v}. Steps/records: 303 sub 45 {p,s,field,v}, Clear sub 44 {p,98 bytes}; 808 sub 62 and 909 sub 79 {p,s,inst,v} (909 hi-hat exclusion sends the second message). Transport (msg 35): mode {0}, play {1,1}, stop {1,0}, loop {3}, tempo {4}, locate {8}, loop start {12}, loop end {16}, shuffle {20}, record {2} (logged; no player command, task-057). UI-only, nothing sent: tr808.selector, tr909.flam_button, bank buttons, 303 edit step/pitch mode, sync/midi LEDs. Not done: logo click (msg 58), keyboard beyond Space and focus arrows (task-070), the song's own mod (task-047).

Facts found while testing: (1) a new empty song has loop 0..1 bar, PCF wave 24, 303 patterns of 16 rests; the panel takes these from the first snapshot. (2) A fader press off its handle jumps then drags, so a drag sends two values (the last counts). (3) Pattern change timing: pressing a pattern button mid-bar, the engine's playing pattern changed 42 calls later at tick 769 (bar line), the strip showed the new pattern at once (a_new_pattern_starts_at_the_bar_line_...). (4) Meter bytes of Fast Forward's first 14 s are small (master 6/127, 303 channel 30), so a linear meter scale with rounding up is used [G]. (5) Mode switch song->pattern->song while playing was tested on Calm (no PCF) with no faulted calls; a song with a PCF is not used for it (task-069).

Run on a real song: cargo run --release -p rb-cli -- standard-mod --rebirth-dll original/ReBirth.dll -o target/Standard.rbm; target/release/rb-ui 'tests/songs/shipped/Fast Forward.rbs' --standard-mod target/Standard.rbm --screenshot OUT.png opens the window on this machine with the audio output started, loads the song (song mode, loop 1 / 250 bars, the patterns, knobs, mixer and effects as the song's data blocks) and exits 0 (screenshot looked right). Playing needs a person at the window; the headless tests play shipped songs instead (Fast Forward in song mode 600 calls: position, bar display, tempo, the four pattern selections, the 808 running light, the master meter and the cutoff knob follow the snapshot every call; Calm in pattern mode).

Review round 1 (coordinator): (1) snapshot-feedback race fixed: Link::flush_marked reads the snapshot's frames before the push and sets mark = before + CALL_FRAMES (the first call that can have drained the commands; the call already running when they are pushed does not), used by forward, load and sync; the > test is kept. Test the_panel_ignores_snapshots_older_than_its_commands (a real race needs the callback thread; the test checks the mark and that the panel keeps the user's value through the following calls). (2) rewind, fast forward, bar down and Stop while recording now emit Set transport.record 0 first ({2,0}, logged until task-057); test msg_35_widths_and_record_off_messages. (3) Audio::current_mut doc corrected. (4) Msg::encode gives the wire bytes (ids 4, 8, 12, 16 a big-endian u32, others one byte); the test asserts them. (5) Link::load forwards the panel's pending events first (test load_sends_the_events_the_panel_had_first). Workspace after the round: see the commit message.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The panel drives the real-time player: every knob, fader, switch, toggle, arrow, pattern button, 303 key and flag, step button and transport button sends the message the original's panel sends (msg 35 and the msg 41 subs of docs/song_load.md 4.10), and the panel follows the engine's snapshot after each call (position, play, mode, loop, tempo, every controller incl. the song's automation, mixer, the selected pattern per section, drum running lights, meters).

Code (branch worktree-agent-a94cd3c62858c4a95, one commit): crates/rb-ui/src/link.rs (new: Link, Msg, the control table), session.rs (new: song and standard-mod loading), model.rs (events: Clear303, stop locate, loop start/end; adopt, held), app.rs (App::with_link), main.rs (rb-ui SONG.rbs [--standard-mod F]), crates/rb-player/src/lib.rs (Audio::current_mut), tests/wired.rs (16 headless tests with a real Player and the test driving the audio callback), docs/ui_spec.md section 16.

Tests: cargo test --release 345 pass, 0 fail (16 new in crates/rb-ui/tests/wired.rs; the 28 unit and 15 interaction tests of task-044 unchanged); cargo clippy --all-targets -D warnings and cargo fmt --check clean. Real run: rb-ui on Fast Forward with the standard kit and the audio output opens, loads and exits 0 (screenshot checked).

Not done, by design or by the notes: logo click (msg 58), record (no player command, task-057; the message is logged), the song's own mod and the menu (task-047), the keyboard beyond Space and the focus arrows (task-070). Guesses [G] documented in ui_spec section 16: meter scale, comp gain-reduction LEDs, the bank following an engine-selected pattern. No engine DSP change.
<!-- SECTION:FINAL_SUMMARY:END -->

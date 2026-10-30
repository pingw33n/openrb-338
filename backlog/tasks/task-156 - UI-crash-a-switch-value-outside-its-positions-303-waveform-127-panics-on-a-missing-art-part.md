---
id: TASK-156
title: >-
  UI crash: a switch value outside its positions (303 waveform 127) panics on a
  missing art part
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-30 09:45'
updated_date: '2026-10-30 10:31'
labels:
  - ui
  - re
milestone: m-5
dependencies: []
references:
  - crates/orb-ui/src/model.rs
  - crates/orb-ui/src/app.rs
  - crates/orb-engine/src/tb303.rs
  - docs/song_load.md
  - docs/ui_spec.md
priority: high
type: bug
ordinal: 174000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on 2026-10-30 OpenRB-338 closed with no dialog. The user had chosen ~/rb-stuff-peff/mods_recrypted_aig/RB-AM.rbm (the user's file, read only) with Mods > Choose file..., then clicked Mods > Demo Song. Run from Terminal, the app printed:

    thread 'main' panicked at crates/orb-ui/src/app.rs:376:50:
    missing art part switch_wave_127

The cause: the demo song (title "ReSurrection 338", RBS 4.2) stores a 303 waveform byte of 127. song_load.md section 8 says that byte reaches the engine unchanged, and the engine plays any value other than 0 as square (tb303.rs:475, 508: `live[WAVEFORM] != 0`). The panel takes the engine value as it is: model.rs about line 578, `*s = v as u8` for a switch. The part name is then built as format!("{fam}{}", self.switch(id)) (model.rs:1037), giving switch_wave_127, and paint_part_with panics on the missing part (app.rs:376).

What:
1. RE, on a copy of the user's Rebirth.exe database: /Users/pingw33n/dev/openrb-338/original/Rebirth.exe.i64. It is read only and currently open in the user's IDA GUI, so do not use or adopt that GUI session. Copy the .i64 to the scratchpad, open the copy headless, close it without saving, and delete it. Find how the exe draws a 2-position switch from a controller value outside 0..1, from a song and from the DLL's poll (sub_43ED76 and the switch drawing; ui_spec.md section 5, "Sound controls"). Candidates: != 0, >= 64, or a clamp. Do the same for every switch kind the panel has (hswitch, vswitch and switch: the 303 waveform, the mode switch, the 808/909 and mixer switches, the PCF mode, and so on), including ones with more than 2 positions. Record the rule with addresses in docs/ui_spec.md, and in docs/song_load.md section 8 if relevant.
2. Fix: the switch's drawn position follows that rule. The stored value stays what the song holds unless the original changes it, so that a save writes the original's bytes. Check what the original saves after loading such a song, if it is known or cheap to learn; otherwise keep the raw value and note it.
3. Robustness: no song value may panic the UI. A missing art part must not abort the app: draw nothing, or a fallback, and debug_assert or log it. Review other format!-built part names (knob frames, LEDs, digits, rotary positions) for the same out-of-range input.
4. Tests:
   - a synthetic song (build it in the test, or edit a copy of a committed test song through the rbs crate) with waveform 127, and out-of-range values for the other switches, opens and draws frames without a panic, and shows the position the rule gives;
   - an orb-ui test that every switch value 0..=255 resolves to an existing part.
   Do not commit the user's mod or its demo song.
5. Verify with the user's file, read only: in a scratch test or the real app (ORB_UI_CONFIG on a temp dir), Mods > Choose file with RB-AM.rbm, then Demo Song, no longer panics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The original's rule for drawing a switch from an out-of-range value is found in Rebirth.exe, with addresses, and recorded in docs
- [x] #2 orb-ui draws switches by that rule; no song value can panic the UI (a missing part is not fatal); a test covers every switch with values 0..=255
- [x] #3 RB-AM.rbm's demo song opens and plays in the UI without a panic
- [x] #4 fmt, clippy, debug cargo test -p orb-ui and cargo test --release pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE on a headless copy of the exe database: the CBitmapSwitch value path (polls, sub_424548, update/draw, CBitmapSequence frame rect) for the 303 waveform, delay mode, PCF mode, 808 selectors, transport mode.
2. orb-ui: behavior::switch_position(id, v) gives the drawn position (the engine's reading of the byte, since the original's frame index out of the strip cannot be drawn); Panel::switch keeps the raw value for saving; part_name, the click flip and the 808 legend frame use the position.
3. Robustness: paint_part_with draws nothing for a missing part and reports it once (art::report_missing / art::missing); 909 step part clamps to frame 3; review the other format!-built names (meters, GR LEDs, digits, knobs, 808 steps are already bounded).
4. Tests: model unit tests (every switch 0..=255, every 909 step value 0..=255 resolves to a part); tests/switch_values.rs opens a synthetic song (303_accent0.rbs edited through rbs) with out-of-range switches and 909 steps, plays, draws, clicks, and checks no part is missing.
5. Verify with RB-AM.rbm (recrypted to scratch from the user's original) in a scratch test; docs ui_spec 2, song_load 8, original_bugs OB-23.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-aff204ad96c43d13d (rebased on main 722805e).
IDA: original/Rebirth.exe.i64 does not exist as a packed file (the GUI holds it unpacked as .id0/.id1/.nam), so the read-only copy is of /Users/pingw33n/dev/openrb-338/ida/Rebirth.exe.i64 (2026-10-26, same exe), opened force_headless from the scratchpad.

RE (IDA disassembly of the headless copy; Hex-Rays only for orientation):
- CBitmapSwitch ctor 0x4247D3 derives from CABCPositionView (ctor 0x4242B6; value word +0x22, drawn word +0x26, strip +0x1C). vtable 0x4BBA7C.
- Setter sub_424548 (0x424548): mov [eax+22h], cx, no clamp. Callers pass the device byte after movzx: 303 waveform +0x504 at 0x43F3B6 (poll sub_43ED76), delay mode +0x1B6 at 0x4081DA (sub_407F7A), PCF mode +0x23A at 0x41BC25 (sub_41B914), 808 selectors +0x65B/+0x65E/+0x661/+0x663/+0x665 at 0x4426F5..0x4427ED (sub_441E79). Transport mode switch is 1/0 from the mode flag (0x42A6B3..0x42A6D9).
- Update 0x4244E8 copies +0x22 to +0x26 and redraws; draw 0x4243C2 -> sub_46F84D(strip, frame=+0x26) -> CBitmapSequence (vtable 0x4BD664) slot 1 0x46DC43: fw = (w+1)/n - 1, x = frame*(fw+1), unbounded; blit sub_488B1E with SRCCOPY (0xCC0020).
- The byte arrives raw from the DLL's msg-40 echo sub_43EB64 (+0x504 = v at 0x43EC2F). Only MIDI CC 23/30 normalizes: sub_4405B5 stores v >= 64 (0x4405F9).
- Candidates refuted: neither != 0, >= 64 (only MIDI) nor a clamp; the original draws an out-of-strip frame (OB-23, effect [G], not observed on the test machine).
Rule orb uses: the position the DLL plays (303 wave, delay mode, 808 selectors: != 0; PCF mode: == 1, cmp dl,1 at 0x10016921). Raw value kept (Panel::value) so a save writes the song's byte; what the original saves after such a load was not observed [G: its panel keeps the raw byte].
Other bugs fixed in passing: the click flip did 1 - switch (u8 underflow panic in debug for 127); the 808 legend frame compared the raw byte with 0; 909 step parts were format!-built from the raw step byte (step909_a_7 missing) -> clamped to 3. Reviewed: meters (.min(10)), GR LEDs (.min(3)), digits (clamped), 808 steps (bool), LEDs/toggles (on/off), knob bodies (static): bounded. Knob positions from engine values > 127 give p > 1000 (angle past +135, no panic) as the original's sub_4A637D (no clamp); left as is.
Observation: in song mode before play, the panel follows the engine's live bytes, so RB-AM's demo shows 303 #2 wave 127 right after open (Link::load from the device chunk) and 1 after the first follow (the track's tick-0 value).
Verification with the user's file: ~/rb-stuff-peff/mods_recrypted_aig/ was empty at run time; the original ~/rb-stuff-peff/ReBirth Mods/AI Generated/RB-AM.rbm was recrypted to the scratchpad with orb mod recrypt --iso (read only), orb info on its Demo.rbs: RBS 4.2 'ReSurrection 338', song mode. A scratch test (not committed) ran App::open_demo_now on it, pressed Play and drew 3000 frames: after open tb303_2.wave = 127 at position 1, no panic, art::missing() empty.
Note: the plan was recorded after the RE and the first code edits, not before them.

Review round 1 (send back on 4d9e324), fixes:
1. OB-23 moved under the existing '## 4. Panel (Rebirth.exe)' after OB-13; the new '## 6' heading dropped.
2. 909 step frame: the reviewer read 4 as a plain accent; the code says otherwise. tr909.rs: the flam pass (0x1002B320) retriggers every value > 2, so 4 = accent + flam and 5..255 = the flam hit alone (the step pass plays 1|3 at x0.8, 2|4 at x2.0, others nothing); the flam is armed on every new step (0x1002ABB0). The original exe (re-opened headless copy of ida/Rebirth.exe.i64, closed save=false, deleted) bounds the 909 LED itself: sub_444E30 0x444F0C cmp eax,2 / 0x444F18 mov byte [edx+ecx],4, then frame v-1 (0x444F28), so every value > 2 shows the flam frame 3; the copy is local (sub_44535E fills it from sub_444D74), the pattern keeps its byte. So orb's min(v,3) is the original's drawing and what plays; kept, with the comment corrected. Recorded in docs/ui_spec.md section 7 (Display) and noted in OB-23. The unit test (renamed every_909_step_value_draws_the_frame_of_what_plays) asserts the frame for 0..=255, playing step inverted.
3. Save round-trip test out_of_range_switch_values_save_back_unchanged: pattern mode after 4 drawn frames, and song mode before the first frame, write the file back byte for byte. Song mode after frames does NOT round-trip, and that is not switch-specific: a probe with 303_accent0.rbs (song mode) and chunk cutoff 10 / waveform 1 against tick-0 events 50 / 0 saved 50 / 0. The followed song-mode panel writes the engine's live values into the device chunks (Link::follow + document::compose). Filed as task-158 (bug, m-5), not fixed here.
Checks: fmt clean, clippy clean, debug cargo test -p orb-ui 232 passed, cargo test --release -q --no-fail-fast 637 passed 0 failed 3 ignored.

Review round 2 (send back on 914bf83): the 909 reading and min(v,3) were accepted. The task-158 file was removed from this branch (one commit per task); the supervisor creates task-158 on main from its text: the follow-up for the song-mode save (a followed song-mode panel writes live values over the device chunks) is task-158 on main. Wrapped the new ui_spec.md section 7 line to 100 columns and shortened the OB-23 heading under 100.

Correction: the follow-up was created on main as task-159 (Backlog.md assigned the next free id), not task-158. Every mention of task-158 above means task-159; the test comment in crates/orb-ui/tests/switch_values.rs names task-159.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Outcome: the original draws a switch's controller byte as the frame index of its two-frame strip with no clamp (sub_424548 0x424548, draw 0x4243C2, CBitmapSequence frame rect 0x46DC43), so a value above 1 shows pixels from outside the strip (OB-23). orb-ui now draws the position the engine plays (behavior::switch_position: PCF mode == 1, every other switch != 0), keeps the raw value for saving, flips from the drawn position, and a missing art part draws nothing and is reported once instead of panicking. The 909 step LEDs draw min(v, 3), which matches the original (0x444F0C..0x444F18 draws every value above 2 as the flam frame) and the engine (values above 2 flam).
Docs: ui_spec.md section 2 (A value out of 0..1) and section 7 (Display), song_load.md section 8, original_bugs.md OB-23 (under section 4).
Tests: model unit tests over every switch and every 909 step value 0..=255 (expected position and frame); crates/orb-ui/tests/switch_values.rs opens a synthetic song (303_accent0.rbs edited through rbs: waveforms 127 and 2, 808 selectors 255/64/0/200/9, PCF mode 2, delay mode 127, 909 steps 4/7/255), plays, draws, clicks, checks no part is missing, and saves it back byte for byte (pattern mode after frames, song mode before the first frame). RB-AM.rbm's demo opened, played and drew 3000 frames in a scratch test with no panic.
Follow-up: task-159 on main (a followed song-mode panel writes live values over the device chunks on save; pre-existing, not switch-specific).
Checks: nightly fmt clean, clippy -D warnings clean, debug cargo test -p orb-ui 232 passed, cargo test --release -q --no-fail-fast 637 passed, 0 failed, 3 ignored. Review rounds 1 and 2 fixed.
Branch worktree-agent-aff204ad96c43d13d, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

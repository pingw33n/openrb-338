---
id: TASK-156
title: >-
  UI crash: a switch value outside its positions (303 waveform 127) panics on a
  missing art part
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-30 09:45'
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
- [ ] #1 The original's rule for drawing a switch from an out-of-range value is found in Rebirth.exe, with addresses, and recorded in docs
- [ ] #2 orb-ui draws switches by that rule; no song value can panic the UI (a missing part is not fatal); a test covers every switch with values 0..=255
- [ ] #3 RB-AM.rbm's demo song opens and plays in the UI without a panic
- [ ] #4 fmt, clippy, debug cargo test -p orb-ui and cargo test --release pass
<!-- AC:END -->

---
id: TASK-182
title: 'MIDI remote control: CCs and pattern selection drive the panel''s controls'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-11-13T18:19:00Z'
labels:
  - re
  - player
  - ui
  - docs
milestone: m-7
dependencies: []
references:
  - docs/initial_investigation.md
  - docs/song_load.md
  - docs/ui_spec.md
  - crates/orb-player/src/lib.rs
priority: medium
type: feature
ordinal: 198000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user decided (2026-11-13) to bring MIDI into scope, until now excluded by AGENTS.md ("MIDI and ReWire are out of scope for now"), in the new milestone m-7, which will also hold the VST3/CLAP plugin. This task is the first half of MIDI input: remote control, where MIDI messages do what the mouse and keyboard do on the panel (turn knobs, flip switches, select patterns, mute channels). Live playing of the devices from notes is a separate task that builds on this one.

What is known: the original has a MIDI implementation (the MIDI LED of `docs/ui_spec.md`, the parameters `docs/song_load.md` marks as probably MIDI remote control around line 413, the 303 waveform CC 23 mentioned in `docs/ui_spec.md`, and the controller table of the ReBirth 1.x MIDI-file import in `docs/song_load.md` section 12). The manual (`/Volumes/downloads/rebirth_2.01_manual.pdf`, or the copy on the CD) likely has a MIDI implementation chart. In the DLL, every panel controller message calls the record hook `sub_10012BC0` after its store (`docs/initial_investigation.md` section 5.3), so a MIDI controller that reaches the same messages records in song mode like a mouse move.

Reverse engineering is deliberately minimal: confirm the mapping (which channel addresses which device, which CC drives which controller and how its value is scaled, how patterns are selected, what other messages the original reacts to, and the MIDI settings the original offers), with addresses checked against the disassembly. MIDI-driven output does NOT have to match the original sample for sample; it should behave as a user of the original would expect. Where the original is unclear or awkward, write down the choice made.

The MIDI handling must be a layer that takes timed MIDI bytes and produces the player's commands, independent of where the bytes come from: the standalone app reads them from a MIDI port, and the future VST3/CLAP plugin will feed it the host's events with sample offsets. Development and testing happen on a Mac with no MIDI hardware (virtual CoreMIDI ports, the IAC Driver, `sendmidi`, MIDI Monitor). The user's IDA databases are read only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A new docs/midi.md documents the original's MIDI remote control (channels per device, the CC-to-controller map with value scaling, pattern selection, the other messages it handles, its MIDI settings), each fact marked [C] with addresses or [G], and lists where orb deliberately differs
- [ ] #2 The MIDI-to-command layer is independent of the MIDI source, and cargo tests drive it with timed byte streams (no MIDI port) covering every mapped controller kind, pattern selection, channel filtering and out-of-range or unmapped messages
- [ ] #3 The standalone app lists the MIDI inputs (including a virtual port it creates on macOS), lets the user choose one, remembers the choice, and lights the panel's MIDI LED on input
- [ ] #4 A knob moved by MIDI moves on the panel too, and in song mode with record on it is recorded like a mouse move
- [ ] #5 A manual check with a virtual port and sendmidi on macOS is recorded in the notes with the commands used
- [ ] #6 AGENTS.md's scope sentence and docs/ui_spec.md's out-of-scope notes on the MIDI LED are updated to say MIDI is in scope in m-7
<!-- AC:END -->

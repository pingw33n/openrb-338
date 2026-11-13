---
id: TASK-183
title: 'MIDI live play: notes play the 303s (and the drums if the original did)'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-11-13T18:20:00Z'
labels:
  - re
  - engine
  - player
  - docs
milestone: m-7
dependencies:
  - TASK-182
references:
  - docs/initial_investigation.md
  - docs/midi.md
  - crates/orb-engine/src/tb303.rs
priority: medium
type: feature
ordinal: 199000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user decided (2026-11-13) to bring MIDI into scope in milestone m-7. task-182 adds MIDI remote control (CCs and pattern selection doing what the panel does) and the source-independent layer that turns timed MIDI bytes into player commands. This task adds the other half: playing the devices live from MIDI notes, which the panel cannot do. It matters for the standalone app with a keyboard and for the future VST3/CLAP plugin, where a DAW's MIDI clip will feed it.

What is known: the DLL has a MIDI input for the 303, `sub_100261F0`, which also calls the record hook (`docs/initial_investigation.md` section 5.3). Whether the 808 and 909 also played their instruments from MIDI notes is not known.

Reverse engineering is deliberately minimal: find out whether and how live play worked in the original. For the 303: the channel, how a note sets pitch, how velocity or other data sets accent and slide, what happens to a running pattern while notes are played (override, mix, or only when stopped), and whether played notes are recorded. For the 808 and 909: whether notes trigger instruments and with which note map. Addresses are checked against the disassembly. The output does NOT have to match the original sample for sample; it should sound and behave as a user would expect (correct pitch, accent, slide and envelopes, no stuck notes, correct timing). If the original had no live play for a device, record that and do not invent it without a Decide task.

Testing happens on a Mac with no MIDI hardware (a virtual CoreMIDI port, VMPK or sendmidi). The user's IDA databases are read only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/midi.md gains a live-play section: what the original's 303 MIDI input (sub_100261F0) does with notes, velocity, slide and a running pattern, and whether the 808 and 909 play from notes, each fact [C] with addresses or [G], plus where orb deliberately differs
- [ ] #2 MIDI notes play each device the original let play live, through the MIDI layer of task-182, from the standalone app's MIDI input
- [ ] #3 cargo tests feed timed note streams and check pitch, accent and slide, note-off, overlapping notes, all-notes-off and notes during a running pattern; no test needs a MIDI port
- [ ] #4 No note hangs after its note-off, a stop, a song load or a mod change
- [ ] #5 A manual check playing the 303 from VMPK or sendmidi through a virtual port on macOS is recorded in the notes
<!-- AC:END -->

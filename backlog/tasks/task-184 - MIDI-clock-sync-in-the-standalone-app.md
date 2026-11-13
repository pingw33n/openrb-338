---
id: TASK-184
title: MIDI clock sync in the standalone app
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-11-13T18:20:00Z'
labels:
  - re
  - player
  - ui
  - docs
milestone: m-7
dependencies:
  - TASK-182
references:
  - docs/initial_investigation.md
  - docs/ui_spec.md
  - docs/midi.md
priority: low
type: feature
ordinal: 200000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user decided (2026-11-13) to bring MIDI into scope in milestone m-7, with MIDI sync at low priority. MIDI sync lets an external sequencer, drum machine or app own the tempo and transport of the standalone app through MIDI real-time messages (clock at 24 per quarter note, Start, Stop, Continue, Song Position Pointer). The future VST3/CLAP plugin does not need it: there the host's transport does the job.

What is known: the DLL export `SetMIDISyncActive` sets `transport+0x160`; while it is set, the sequencer takes its tempo from the MIDI clock (`sub_1001BCC0`) and Stop does not locate to the bar start (`docs/initial_investigation.md` section 5, transport messages). The panel's tempo control is ignored while synced (`sub_4094D4`) and has a Sync LED (`docs/ui_spec.md`). How the original turns clock pulses into a tempo (smoothing, rounding) and how it handles Start, Continue and Song Position Pointer is not known.

Reverse engineering is deliberately minimal: enough to follow the original's behavior (what the clock drives, what Start/Stop/Continue/SPP do in song and pattern mode, what the panel shows). The output does not have to match the original sample for sample, since it depends on when the clock pulses arrive; it should stay in time with the clock source without drift and react to transport messages as a user would expect. Testing happens on a Mac with no MIDI hardware: a DAW (for example a Reaper trial) sending clock to the IAC Driver or to orb's virtual port. It uses the MIDI input and port selection of task-182. The user's IDA databases are read only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/midi.md gains a sync section: how the original derives tempo from MIDI clock and what Start, Stop, Continue and Song Position Pointer do, each fact [C] with addresses or [G], plus where orb deliberately differs
- [ ] #2 With sync on, the app follows an external clock's tempo and Start/Stop/Continue/SPP; the panel's tempo control is locked and the Sync LED lights as in the original
- [ ] #3 cargo tests feed timed real-time message streams (no MIDI port) and check the tempo followed at a steady and a changing clock, a jittered clock, and each transport message
- [ ] #4 After 5 minutes against a steady clock the position does not drift from the clock source by more than one tick, checked in a test
- [ ] #5 A manual check against a DAW sending clock through a virtual port on macOS is recorded in the notes
<!-- AC:END -->

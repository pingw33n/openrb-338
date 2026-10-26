---
id: TASK-004
title: 'RE: engine state at export start and the host''s realtime behavior'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
labels:
  - re
  - docs
milestone: m-0
dependencies: []
references:
  - docs/initial_investigation.md
priority: high
type: spike
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The export does not reset the engine (docs/initial_investigation.md section 9): the 303 oscillator and filter states, the leftover sub-block buffer in sub_100139E0, the mixer's 16-block counter, the smoothed gains, the delay buffer and write pointer, the compressor/PCF states and the global snare/bass-drum counters carry over from whatever the app did before. To make the original's output deterministic we need the complete list of that state and of every event that changes it on the host side.

Trace in Rebirth.exe and ReBirth.dll (copies of ida/*.i64, headless, read-only) and document in `docs/export_state.md`:
- how the exe drives the engine in realtime (the DirectSound/MME audio thread, SyncSoundCallback vs SafeSoundCallback, buffer sizes) and whether the engine is called while stopped (does the 303 keep running when idle);
- what the exe does around an export (sub_427988: the realtime audio object's vfunc +8/+4 around the render), whether a realtime callback can run during the export, and what is in the leftover buffer when the export starts;
- every place msg 33 (full reset, sub_10014140) is sent and what it resets; what song load, New, Open and app start do to engine state;
- the full list of engine state that survives into an export, each with its address, its reset points (if any) and its effect on the output;
- the startup sound (preview player at +1064 in SafeSoundCallback) and anything else mixed into the output.
End with a proposed protocol (a sequence of user or host actions) that should make an export start from a known state, and the residual state it cannot control.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/export_state.md lists every piece of engine state that survives into an export, with addresses, reset points and output effect
- [ ] #2 The exe's realtime driving of the engine while playing, stopped and exporting is described with addresses
- [ ] #3 The document ends with a proposed deterministic capture protocol and the state it leaves uncontrolled
<!-- AC:END -->

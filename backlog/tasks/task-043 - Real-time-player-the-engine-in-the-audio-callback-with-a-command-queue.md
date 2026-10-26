---
id: TASK-043
title: 'Real-time player: the engine in the audio callback with a command queue'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 16:23'
labels:
  - engine
  - ui
milestone: m-3
dependencies:
  - TASK-040
  - TASK-029
priority: high
type: feature
ordinal: 43000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The m-1 engine renders an export offline. The UI needs it live: a real-time player that runs the engine in the audio callback of the chosen output library and takes commands from the UI thread without locks or allocation in the callback: transport (play, stop, locate, loop), pattern mode and song mode, every device parameter change (as the original applies them: docs/song_load.md, docs/export_state.md), pattern edits, mute, tempo, shuffle, the Vintage flag, loading a song and a mod; plus state the UI reads back (song position, the current step of each device, meter levels if the UI shows them). Output at 44.1 kHz (resample if the device does not offer it, and say how). The offline render (and so the corpus tests) must stay bit-identical; add a test that the player, fed the same song with no commands, produces the export's samples after the same idle count.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A player crate or module plays a song through the chosen audio library with transport, pattern and song mode, and live parameter changes from another thread
- [ ] #2 No locks or allocations happen in the audio callback (checked by a test or a debug allocator)
- [ ] #3 A test shows the player's output equals the offline render for the same song and idle count
<!-- AC:END -->

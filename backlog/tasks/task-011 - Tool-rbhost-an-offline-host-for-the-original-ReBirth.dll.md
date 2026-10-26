---
id: TASK-011
title: 'Tool: rbhost, an offline host for the original ReBirth.dll'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
labels:
  - tools
  - re
  - testmachine
milestone: m-0
dependencies:
  - TASK-003
  - TASK-004
  - TASK-002
priority: high
type: feature
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
An exported WAV from the GUI depends on uncontrolled state (docs/initial_investigation.md section 9). A small Win32 program that loads the original ReBirth.dll itself and drives it offline removes that: a fresh process, a known block alignment, no realtime thread. It also gives us the engine's run-time tables (all computed at init into BSS) and its state for the Rust port.

Write `rbhost` (C, i686 Windows, cross-built on the Mac with llvm-mingw: see ~/dev/dcre/common/tools/fetch_llvm_mingw.sh; sources under tools/rbhost/): it loads ReBirth.dll, emulates whatever the panel side of the shared-memory port needs (EngineOpen, the "ReBirth Panel->DLL"/"DLL->Panel" mappings, semaphores and events; docs/export_state.md), sends the song-load messages of docs/song_load.md for a given .rbs, then the export sequence (stop, locate to loop start, play, the 2048-frame SafeSoundCallback loop with the exe's double-precision chunking, stop) and writes a WAV. Options: `--dump-tables DIR` writes the BSS tables listed in docs/initial_investigation.md section 10 item 4 (with their addresses) after init; `--dump-state` writes the engine objects at export start. It must not need the GUI or a registration; if the DLL refuses to run without the panel, document why and what the minimal emulation is.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rbhost renders a given .rbs to WAV on the test machine without Rebirth.exe running, from a script that builds it on the Mac, uploads and runs it
- [ ] #2 Two renders of the same song by rbhost are byte-identical
- [ ] #3 --dump-tables writes every table of section 10 item 4 with a manifest of addresses and sizes
<!-- AC:END -->

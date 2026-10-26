---
id: TASK-011
title: 'Tool: rbhost, an offline host for the original ReBirth.dll'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 10:38'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-004 (2026-10-26, docs/export_state.md): use the shared-memory port names and protocol of section 2.7 and the export sequence of section 4. Besides the plain offline render, rbhost needs an idle-block option (--idle-blocks N, N a multiple of 16) that renders N 64-frame blocks of the active, stopped song before the export, as the app's realtime stream does while stopped (sections 2.4, 7, 9): the 303 oscillator phase and the delay write pointer depend on N, so the app's exports can be reproduced by fitting N. Also an option to preset the four process-global drum counters (section 7.5) and to mix the startup sound through the preview player (section 8) for the comparison with app exports.
<!-- SECTION:NOTES:END -->

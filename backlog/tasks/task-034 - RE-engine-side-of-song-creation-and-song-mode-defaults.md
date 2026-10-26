---
id: TASK-034
title: 'RE: engine side of song creation and song-mode defaults'
status: To Do
assignee: []
created_date: '2026-10-26 10:05'
updated_date: '2026-10-26 10:38'
labels:
  - re
  - docs
milestone: m-1
dependencies:
  - TASK-003
references:
  - docs/song_load.md
  - docs/initial_investigation.md
priority: low
type: spike
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
task-003 traced how Rebirth.exe turns an .rbs into engine messages (docs/song_load.md). Four engine-side points were left open (docs/song_load.md section 11) and matter for the song model (task-018) and the export start state:
1. Top-level msg 52 {u16 mod} opens a new song; the DLL queues it (sub_1001D694 -> object at +0x414, sub_10009520 / sub_10026EA0). Find where it is processed, whether it creates new device objects (so fresh 303, effect and mixer state at every load), and the constructor defaults of every device parameter.
2. In song mode the DLL does not copy a device's msg-41 parameters to the live values (303 at 0x10025CB0, mixer at 0x1001588F); a controller with no TRAK event at or before the locate keeps its constructor value. Confirm the same rule in the delay (0x1000FB40), PCF (0x10016C50), distortion (0x10010D60), compressor (0x1000EFD0), 808 (0x100282B0) and 909 (0x1002B8E0) host-message handlers, and list the defaults that apply.
3. The 4.0 mixer-track converter sub_437380 in the exe (4.0 files only; RBS42 is 4.2): its controller remapping.
4. Global byte 2 (msg 35 id 2, setter sub_10021190, a thunk to 0x10021194, used by locate 0x1001F984) and byte 22 (song +0x16E): their meaning.
Read-only IDA work on headless copies (doc-001); write the results into docs/song_load.md with addresses.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/song_load.md states, with addresses, whether msg 52 creates new device objects and lists the constructor default of every automatable parameter
- [ ] #2 The song-mode msg-41 rule is confirmed or refuted for each of the six other devices
- [ ] #3 sub_437380's controller mapping and the meaning of global bytes 2 and 22 are documented, or marked unresolved with what was tried
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-004 (2026-10-26): item 1 is answered in part: docs/export_state.md section 3 shows every load (msg 52) creates new device objects [C], so no per-song state survives a load. The constructor default of every automatable parameter (the rest of item 1) and items 2-4 remain.
<!-- SECTION:NOTES:END -->

---
id: TASK-047
title: 'UI menu: load/save song, export, choose RBM, Vintage flag'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 16:23'
updated_date: '2026-10-27 07:55'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-045
  - TASK-046
  - TASK-028
  - TASK-033
  - TASK-056
priority: medium
type: feature
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The menu, reduced to what the user asked for (2026-10-26): Load song, Save song (and Save as), Export (the song loop to .wav/.flac/.ogg through the m-1 export path, so the file equals `rb render`'s), Choose RBM (the mods found in the mods folder or a file picked in the in-app browser; "Standard ReBirth" for the built-in kit), and the Vintage ReBirth Synth Sound flag (GLOB byte 482, docs/song_load.md section 3). Unsaved-changes prompt inside the app. Nothing else from the original menu.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Load, Save and Save as round-trip a song (the saved file re-opens identically)
- [ ] #2 Export writes the same file as rb render for the same song and options
- [ ] #3 Choose RBM switches the kit, including back to the standard kit; the Vintage flag toggles the sound model and is saved with the song
<!-- AC:END -->

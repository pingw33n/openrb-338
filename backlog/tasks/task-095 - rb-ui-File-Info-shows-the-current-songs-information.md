---
id: TASK-095
title: 'rb-ui: File > Info shows the current song''s information'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:51'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/menu.rs
  - docs/RBS42.txt
  - docs/ui_spec.md
priority: low
type: feature
ordinal: 115000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for File > Info, which shows information about the current song: its metadata, the mod it uses and so on. The song format has a Song info chunk (docs/RBS42.txt: window title ZSTRING(41), more text ZSTRING(201) with 0x0d as newline, web page ZSTRING(101), show song info on open) and a mod reference chunk (name of mod ZSTRING(65), ftp and www addresses), plus the version info and copyright. The original shows the song info in its Song Information dialog.

What: add File > Info, which opens a window (egui Window, closable, Escape closes it) with: the file name and path (or Untitled), the song's window title, the more text (0x0d shown as line breaks), the web page (a clickable link), the mod's name and whether it is loaded from the mod folder or missing, the format version, the tempo and other global facts that are cheap to show. Read-only; editing the song info is out of scope. Do not implement 'show song info on open' (note it in docs/ui_spec.md as not done).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 File > Info opens a window with the song's file name, song info texts, web page link and mod name (with loaded/missing)
- [ ] #2 The window updates when another song is opened and closes with Escape or its close button
- [ ] #3 A test checks the info built from a song (the strings and mod name) without rendering
- [ ] #4 docs/ui_spec.md section 18 lists the item; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

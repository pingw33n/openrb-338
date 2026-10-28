---
id: TASK-095
title: 'rb-ui: File > Info shows the current song''s information'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:51'
updated_date: '2026-10-28 07:08'
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
- [x] #1 File > Info opens a window with the song's file name, song info texts, web page link and mod name (with loaded/missing)
- [x] #2 The window updates when another song is opened and closes with Escape or its close button
- [x] #3 A test checks the info built from a song (the strings and mod name) without rendering
- [x] #4 docs/ui_spec.md section 18 lists the item; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. New module crates/rb-ui/src/info.rs: SongInfo::of(&Document) (pure: strings, mod name, loaded/missing from doc.kit, version, tempo, mode, loop, shuffle, copyright) and App::info_ui drawing an egui Window (close button, Escape).
2. File > Info menu item (Cmd::Info), App.info_open flag; the window reads doc each frame so it follows opening another song.
3. Unit tests for SongInfo (strings, 0x0d to newline, mod loaded/missing/standard) without rendering.
4. docs/ui_spec.md s18: File > Info bullet, show-on-open not done; Demo Song bullet: startup sound, 0x41A3E0 [C].
5. fmt, clippy, cargo test --release with mod env.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a0611dc03a68b9a00

Mod env set (RB_STANDARD_MOD, RB_MOD_DIR): the mod-dependent tests ran, none skipped (no 'skip' output in rb-ui tests). cargo test --release: 459 passed, 0 failed workspace-wide. Tests: info::tests (2) build SongInfo from a Document without rendering. Escape is consumed only while the window is open. Not done: show-song-info-on-open (noted in ui_spec s18). Window not visually run (headless agent).

Review round 1: (1) links only for http/https or host-like text (info::web_url, tests for accept/reject), ftp shown as plain text; (2) test the_info_window_follows_the_song_and_escape_closes_it in tests/menu.rs drives egui with an Escape key event and opens two songs, checking App::info_shown; (3) ui_spec wording now 'the instruction at 0x41A3E0 in sub_41A0C5', source docs/rbm.md 13.1; (4) tests for CRLF, Latin-1 and mod ftp/www. Edit note: the info.rs web_url block was inserted with a python script by mistake; rest by Edit. cargo test --release: 462 pass (mod env set, none skipped).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
File > Info added: crates/rb-ui/src/info.rs (SongInfo::of(&Document), web_url, App::info_ui egui Window 'Song Information', close button or Escape), menu item below Save As (Cmd::Info), App::info_open/info_shown. Shows file/folder, title, text (0x0d as newline), web page (a link only for http/https or host-like text, else plain), mod name with built-in/loaded/missing, mod web (link rule) and ftp (plain), version, copyright, tempo, mode, loop, shuffle, synth sound; follows the open song. Tests: info unit tests (strings, CRLF, Latin-1, mod fields, link accept/reject) and a headless egui test in tests/menu.rs (Escape closes, another open updates). docs/ui_spec.md s18 updated. fmt, clippy -D warnings clean, cargo test --release 462 pass with the mod env.
<!-- SECTION:FINAL_SUMMARY:END -->

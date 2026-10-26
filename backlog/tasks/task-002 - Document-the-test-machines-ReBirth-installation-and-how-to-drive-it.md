---
id: TASK-002
title: Document the test machine's ReBirth installation and how to drive it
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 09:36'
labels:
  - testmachine
  - docs
milestone: m-0
dependencies:
  - TASK-001
priority: high
type: docs
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Agents need one written place that says how to use the installed ReBirth on the test machine before any runner tool exists. Find out and write `docs/test_machine.md`:
- rb's folder layout on the machine: the installation, the songs shipped with it, where the preferences file ("ReBirth RB-338 2.prf") lives, and a working folder for uploads and renders.
- How to start and stop the app through rcs_client.py (`exec`, `ps`, `kill`), its main window title, a screenshot of the main window.
- The menu path and keyboard route to File > "Export Loop as Audio File" and its dialogs (format pop-up WAV/AIFF, file name), with screenshots.
- The preferences that change what the engine does before an export: the startup sound (Startup.aif, played by the engine's preview player), the default song loaded at start (Demo.rbs or "Use Current Song"), the audio driver settings. Record their current values.
- The permit rule of doc-001 applied to this app (a ReBirth session holds all the permits of `redc-win-run`).
Copy the .rbs songs that ship with the installation into the scratchpad and list them with their hashes in the document (they are input for the RBS tools; where they are committed is decided by the storage Decide task).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/test_machine.md exists and covers the folder layout, start/stop through rcs, the window title, the export dialog route with screenshots, and the engine-affecting preferences with their current values
- [x] #2 The shipped .rbs songs are listed with their SHA-256 in docs/test_machine.md
- [x] #3 The agent brief doc-001 points to docs/test_machine.md (already does; verify the paths it names are right)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Drive the app over rcs: layout, start/stop, menus, Open/Export dialogs, Preferences; copy shipped songs; write docs/test_machine.md
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-001 (2026-10-26): the app is in C:\ReBirth RB-338, outside the rcs root C:\re, so rcs ls/get/put cannot reach it; use exec with absolute paths (cmd.exe /c dir|copy, certutil). Copy the shipped songs and mods into C:\re\rb\ with exec cmd.exe /c copy (or robocopy), then rcs get them. Startup shows a modal 'Failed opening MIDI input' box (title 'ReBirth RB-338', not foreground); find the preference that stops it (MIDI input port: none) so a start is unattended. The main window title is the song name ('Untitled song').

Branch: worktree-agent-af2257992fc5e8a26

Done on the machine under all 6 permits (released). Evidence in docs/test_machine.md. Findings: prf is in VirtualStore, not the install dir; no pref removes the MIDI-input box (no MIDI device; Input list disabled); no default-song pref exists (new-song template lives at prf 0x4000); Startup.aif behavior unconfirmed (Play Mod Startup Sound checked); export to C:\re\rb works (Calm WAV 56,013,828 B, 16-bit stereo 44.1k), first attempt wrote a stray Calm.AIF (deleted); task-001's claim that the window title is the song name is wrong (frame title is fixed, the child window carries the song name). Unconfirmed/skipped: what Startup.aif does at start/export, whether the audio driver affects export samples, prf slot order. doc-001 paths verified correct.

Review round 1: fixed the wrong section cite (preview-player mix is section 3, step 4, not 7.3) in docs/test_machine.md; commit amended.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
docs/test_machine.md written with screenshots in docs/test_machine/; 18 shipped .rbs committed in tests/songs/shipped/ with SHA-256 in the doc; 4 mods listed with hashes, not committed. Tested live on the test machine (app started and stopped by PID 9544, permits held and released). Branch worktree-agent-af2257992fc5e8a26.
<!-- SECTION:FINAL_SUMMARY:END -->

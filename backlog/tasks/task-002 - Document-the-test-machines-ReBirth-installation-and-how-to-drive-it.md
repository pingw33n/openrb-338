---
id: TASK-002
title: Document the test machine's ReBirth installation and how to drive it
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
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
- [ ] #1 docs/test_machine.md exists and covers the folder layout, start/stop through rcs, the window title, the export dialog route with screenshots, and the engine-affecting preferences with their current values
- [ ] #2 The shipped .rbs songs are listed with their SHA-256 in docs/test_machine.md
- [ ] #3 The agent brief doc-001 points to docs/test_machine.md (already does; verify the paths it names are right)
<!-- AC:END -->

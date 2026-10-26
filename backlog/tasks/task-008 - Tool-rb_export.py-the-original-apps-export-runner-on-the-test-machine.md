---
id: TASK-008
title: 'Tool: rb_export.py, the original app''s export runner on the test machine'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
labels:
  - tools
  - testmachine
milestone: m-0
dependencies:
  - TASK-002
priority: high
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
References come from the original app's File > Export Loop as Audio File (docs/initial_investigation.md section 3). Write `tools/rb_export.py`, a runner that, holding all the permits of `redc-win-run` (doc-001), uploads an .rbs to the test machine, starts ReBirth fresh (no leftover process), opens the song, exports the loop as WAV through the menu and dialogs (rcs_client.py input/screenshot, or a script run on the machine if the machine has a UI automation library; say which), waits for the file to be complete, downloads it, quits the app, and releases the permits. Options for the capture protocol variants the determinism experiment needs: fresh process per song or several exports in one session, play for N seconds before exporting, export twice in a row. It records what it did (protocol, timings) in a JSON sidecar next to the WAV. Use docs/test_machine.md for the UI route.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb_export.py exports a given .rbs to a local WAV and JSON sidecar unattended, taking and releasing the permits itself (run on two songs, output listed in the notes)
- [ ] #2 The protocol variants (fresh process, repeated export, play before export) are options and recorded in the sidecar
- [ ] #3 A failed step (dialog not found, timeout) stops the app it started, releases the permits and exits non-zero with a screenshot saved
<!-- AC:END -->

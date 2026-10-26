---
id: TASK-014
title: Capture and commit the reference corpus
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
labels:
  - tests
  - testmachine
milestone: m-0
dependencies:
  - TASK-009
  - TASK-013
  - TASK-006
  - TASK-007
priority: high
type: task
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Capture the reference corpus: every diagnostic song (and the shipped songs, as the storage decision allows) rendered by the decided protocol, stored per the storage decision, with a MANIFEST.md listing for each reference: the song file and its SHA-256, the protocol and tool (rbhost or rb_export.py, with versions or commits), the capture date, the WAV's SHA-256 and length in frames. A script re-captures the whole corpus from the manifest; rbcmp.py checks a fresh capture of three songs against the committed references.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The corpus and its MANIFEST.md are committed per the storage decision
- [ ] #2 The re-capture script exists and a fresh capture of three songs equals the committed references (rbcmp output in the notes)
<!-- AC:END -->

---
id: TASK-046
title: In-app file browser (no native dialogs)
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 16:23'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-044
priority: medium
type: feature
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user wants no native dialogs. Build an in-app file browser (a panel or overlay drawn with the toolkit) used for: opening and saving songs (.rbs), choosing the export target (path and format .wav/.flac/.ogg, as the rb CLI writes them), and choosing an RBM mod. Navigation by mouse and keyboard, recent folders, a filter by extension, overwrite confirmation inside the app, and sane defaults (the user's documents folder, a configurable songs folder and mods folder).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An in-app browser opens and saves files with filters, keyboard navigation and recent folders, without any native dialog
- [ ] #2 Overwrite confirmation and errors are shown inside the app
<!-- AC:END -->

---
id: TASK-035
title: 'tools: fail when the Rebirth.exe check cannot run tasklist'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 13:05'
labels:
  - tools
  - testmachine
milestone: m-0
dependencies: []
priority: low
type: bug
ordinal: 11500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
tools/rb_export.py Machine.rebirth_pids() (around line 196) parses tasklist output but ignores its return code: if tasklist fails, the list reads as empty and both rb_export.py's 'refuse to start if Rebirth.exe runs' check and tools/rbhost/run.py's 'no Rebirth.exe during an rbhost run' check (task-011) pass silently. Raise a Failure when tasklist exits non-zero or its output does not look like a tasklist listing, and add a unit test with a fake client. Found in the task-011 review.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Machine.rebirth_pids() raises Failure on a non-zero tasklist exit or unparseable output
- [ ] #2 A unit test covers both cases; the existing tests still pass
<!-- AC:END -->

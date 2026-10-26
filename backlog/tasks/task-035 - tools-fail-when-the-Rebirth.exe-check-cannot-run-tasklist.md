---
id: TASK-035
title: 'tools: fail when the Rebirth.exe check cannot run tasklist'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 13:05'
updated_date: '2026-10-26 16:07'
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
- [x] #1 Machine.rebirth_pids() raises Failure on a non-zero tasklist exit or unparseable output
- [x] #2 A unit test covers both cases; the existing tests still pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Raise Failure in rebirth_pids on non-zero rc or output that is neither CSV rows nor the INFO: no-match line; add fake-client tests.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a8d262daef86b90f3

Both callers (start_app/stop_app, rbhost/run.py) go through rebirth_pids so both get the check. Tests: tools 36 pass, tools/rbhost 15 pass (discover needs -p '*_test.py').
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rebirth_pids() raises Failure on non-zero tasklist exit, unparseable lines or empty output; accepts the 'INFO: No tasks...' line as an empty list. TasklistTest (4 tests, fake client) added. Python tests pass.
<!-- SECTION:FINAL_SUMMARY:END -->

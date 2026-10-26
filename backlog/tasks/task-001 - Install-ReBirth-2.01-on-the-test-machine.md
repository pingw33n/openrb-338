---
id: TASK-001
title: Install ReBirth 2.01 on the test machine
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-10-26 09:06'
labels:
  - testmachine
milestone: m-0
dependencies: []
priority: high
type: chore
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The reference renders come from the original ReBirth RB-338 2.01 running on the Windows test machine (reached with ~/dev/dcre/common/tools/rcs_client.py, see the agent brief doc-001). Install it there under `rb/` on the rcs server's root so that the agents can drive it. The app checks its registration ("The ReBirth Engine is online but not registred" in Rebirth.exe), so this needs the user's installer and registration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ReBirth 2.01 is installed under rb/ on the rcs server's root, registered, and starts to its main window
- [ ] #2 Rebirth.exe and ReBirth.dll on the machine have the same SHA-256 as original/Rebirth.exe and original/ReBirth.dll (or the difference is noted in the task)
<!-- AC:END -->

---
id: TASK-062
title: 'rb_export.py: restore the user''s ReBirth prefs after every app run'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-27 08:58'
labels:
  - tools
  - testmachine
milestone: m-2
dependencies: []
references:
  - docs/test_machine.md
priority: high
type: bug
ordinal: 33600
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user's ReBirth preferences on the test machine live in the VirtualStore file C:/Users/pingw33n/AppData/Local/VirtualStore/Windows/ReBirth RB-338 2.prf (docs/test_machine.md section 5). tools/rb_export.py with --restore-prf restores the protocol's saved copy (C:/re/rb/prf_before.prf, SHA-256 6309526e1574414dd3c093f97fedb5e7978428c789db94099c3af81ee8cbd51e) only BEFORE starting the app; when the app quits it writes its prefs back, so every run leaves the user's file changed (with --startup-sound off, byte 0x20A "Play Mod Startup Sound" = 0, and the recent-file list rotated to the uploaded test song). Found in task-058; the supervisor restored the file on 2026-10-27 at the user's request, and the user asked that the tool restore it after every run too.

Work: make rb_export.py (and any other tool that starts the app, e.g. tools/rbhost/fit.py if it does) restore the saved prefs file after the app has exited, on every exit path (normal, Failure, timeout, KeyboardInterrupt; the same finally that releases the permits), and verify by hash. If the saved copy is missing, fail before starting the app. Document it in docs/test_machine.md section 5. Test with a fake rcs client (the existing tools tests' pattern) that the restore runs after a normal run and after a failure. One short real app session to show the file's hash equals 6309526e... after the run (all redc-win-run permits only when free; if Rebirth.exe is already running — the user sometimes uses it — wait).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After every app run (normal or failed) the VirtualStore prefs file equals the saved copy by SHA-256, checked by the tool
- [ ] #2 A fake-client test covers the restore after a normal run and after a failure; docs/test_machine.md section 5 describes it
- [ ] #3 One real app session shows the prefs file hash equal to the saved copy afterwards
<!-- AC:END -->

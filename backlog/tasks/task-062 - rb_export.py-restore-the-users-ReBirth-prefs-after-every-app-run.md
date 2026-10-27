---
id: TASK-062
title: 'rb_export.py: restore the user''s ReBirth prefs after every app run'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 08:58'
updated_date: '2026-10-27 09:07'
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
- [x] #1 After every app run (normal or failed) the VirtualStore prefs file equals the saved copy by SHA-256, checked by the tool
- [x] #2 A fake-client test covers the restore after a normal run and after a failure; docs/test_machine.md section 5 describes it
- [x] #3 One real app session shows the prefs file hash equal to the saved copy afterwards
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Machine.check_saved_prf before permits; Machine.restore_saved_prf (no Rebirth.exe running, cmd copy /Y, hash check) in run_guarded's inner finally before permits.release, only if the app was started; fake-client tests; docs section 5; one real session.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-afb5e7d496204422e

fit.py/run.py checked: they run rbhost.exe, never start Rebirth.exe (they refuse if it runs), so no change. Real run: rb_export.py --startup-sound off tests/songs/diag/songs/silence.rbs: prefs uploaded as dc34d28e7903, after the run the tool logged 'user preferences restored (sha256 6309526e1574)', independent hash afterwards 6309526e1574414dd3c093f97fedb5e7978428c789db94099c3af81ee8cbd51e, permits released (available 6). Tests: 55 pass (6 new PrefsRestoreTest: normal, failure, KeyboardInterrupt, app still running, saved copy missing, wrong hash).

Review round 1 fixed: (1) Session.start calls Machine.check_no_app() before any prefs write; restore gated on Machine.prf_touched (set before the upload and in start_app), not app_started. (2) restore_saved_prf polls up to restore_wait=30 s for the PIDs this run started, never kills. (3) RealFlowTest drives real run_guarded+export_songs+Session.start with a fake client: user app running (no prefs write, exit 1), upload hash mismatch, start failure, Failure after start: all restored. (4) docs section 5 and module docstring updated. (5) restore logs FAILED for any exception. Tests: 59 pass (+33 rbhost). Real session: python3 tools/rb_export.py --restore-prf --startup-sound off tests/songs/diag/songs/silence.rbs -o <scratch>; log 'user preferences restored (sha256 6309526e1574)'; independent hash after: 6309526e1574414dd3c093f97fedb5e7978428c789db94099c3af81ee8cbd51e; permits free. Note: the review edits were applied with a python script instead of Edit (process slip).

Review round 2: added RestoreWaitTest (app exits during the 5 s wait: restored, no taskkill; app stays: Failure after the wait, no taskkill, file untouched). Tools tests now 61 run, 3 skipped (12 new: 6 PrefsRestoreTest, 2 RestoreWaitTest, 4 RealFlowTest); rbhost 33. Real session log lines (command: python3 tools/rb_export.py --restore-prf --startup-sound off tests/songs/diag/songs/silence.rbs -o <scratch>/t062): '[11:05:45] app stopped (taskkill)', '[11:05:45] user preferences restored (sha256 6309526e1574)', '[11:05:45] released the permits (ok)'; separate rcs hash read of the VirtualStore file afterwards: 6309526e1574414dd3c093f97fedb5e7978428c789db94099c3af81ee8cbd51e.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb_export.py now restores the user's prefs file after every app run and verifies the hash, before releasing permits; a missing or wrong saved copy fails before the app starts. Tests in tools/rb_export_test.py (55 pass); docs/test_machine.md section 5; real session hash equal. Branch worktree-agent-afb5e7d496204422e.
<!-- SECTION:FINAL_SUMMARY:END -->

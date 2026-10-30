---
id: TASK-157
title: >-
  UI: a panic writes a crash log and shows an error box instead of closing
  silently
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-30 09:48'
labels:
  - ui
milestone: m-5
dependencies: []
references:
  - crates/orb-ui/src/main.rs
  - crates/orb-ui/src/error_box.rs
  - docs/ui_spec.md
  - docs/building.md
priority: high
type: feature
ordinal: 175000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on 2026-10-30 OpenRB-338 panicked (task-156, "missing art part switch_wave_127") and the window just vanished. A Rust panic exits with code 101 and prints to stderr, but an app started from Finder, the Start menu or a desktop launcher has no visible stderr, and macOS writes no crash report for it. The cause was found only by running the app from Terminal. Users need a log they can send, and a message telling them what happened.

What:
1. Install a panic hook at startup, in crates/orb-ui/src/main.rs, before eframe runs. It keeps the default hook's stderr output and also writes a crash log containing:
   - the UTC time;
   - the version string (crates/orb-ui/src/version.rs, the same text the About window shows);
   - the OS and architecture;
   - the thread name;
   - the panic message and its location;
   - a backtrace, std::backtrace::Backtrace::force_capture(), so it works without RUST_BACKTRACE.

   The log goes to the platform's log folder:
   - macOS: ~/Library/Logs/OpenRB-338/;
   - Windows: %LOCALAPPDATA%\OpenRB-338\logs\;
   - Linux: $XDG_STATE_HOME/OpenRB-338/ or ~/.local/state/OpenRB-338/.

   Name each log crash-YYYYMMDD-HHMMSS.log and keep only the newest 20. Writing the log must not panic in turn; ignore its errors after trying stderr.
2. Tell the user. After the log is written, show a native message box: "OpenRB-338 stopped because of an internal error. A crash log was saved to <path>." with the panic message, then exit with a non-zero code. The egui error box cannot be used, because the egui frame that panicked may be unusable. Pick a small, maintained crate that does a blocking native dialog on all three systems: rfd's MessageDialog (rfd is a common eframe companion; check whether it is already a dependency), or native-dialog / tinyfiledialogs. Check the license (GPL-3.0-only compatible) and that it cross-builds for the six targets (scripts/build-all.sh and the CI matrix). If no dialog can be shown, such as on Linux without a dialog backend, the log and stderr still happen.
3. Panics on other threads: the audio thread, the export thread and the link. Make sure each goes through the same hook. If such a thread panics without ending the process, either recover as today and log only, or end the app with the box, and say which. Never let it hang silently.
4. Unsaved work: nothing is saved automatically. Say in the box that unsaved changes were lost, unless an autosave exists.
5. docs: docs/ui_spec.md (the error message box section) and docs/building.md (the log folder for bug reports, next to the ORB_* table if a variable is added). If useful, add an environment variable that turns the box off for tests (ORB_UI_NO_CRASH_DIALOG), and list it in the table.
6. Tests:
   - a unit test of the log writer: a temp folder, the file content, the 20-file rotation;
   - an integration test that runs the OpenRB-338 binary with a hidden debug switch that panics on purpose, for example `--debug-panic` among the hidden developer options (docs/building.md), with the dialog off. It checks that the exit code is non-zero and that a log file appears in the folder the test sets, for example through ORB_UI_LOG_DIR.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A panic on any thread writes a crash log with the version, the OS, the message, the location and a backtrace to the platform's log folder, keeping the newest 20
- [ ] #2 The user sees a native message box naming the log file, then the app exits non-zero; with no dialog available, the log and stderr still happen
- [ ] #3 Tests cover the log writer and an end-to-end deliberate panic (exit code and log file); docs/ui_spec.md and docs/building.md describe it
- [ ] #4 The dialog crate's license is GPL-compatible and it builds for the six targets; fmt, clippy and the tests pass
<!-- AC:END -->

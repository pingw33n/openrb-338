---
id: TASK-157
title: >-
  UI: a panic writes a crash log and shows an error box instead of closing
  silently
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-30 09:48'
updated_date: '2026-10-30 10:32'
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
- [x] #1 A panic on any thread writes a crash log with the version, the OS, the message, the location and a backtrace to the platform's log folder, keeping the newest 20
- [x] #2 The user sees a native message box naming the log file, then the app exits non-zero; with no dialog available, the log and stderr still happen
- [x] #3 Tests cover the log writer and an end-to-end deliberate panic (exit code and log file); docs/ui_spec.md and docs/building.md describe it
- [x] #4 The dialog crate's license is GPL-compatible and it builds for the six targets; fmt, clippy and the tests pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. crates/orb-ui/src/crash.rs: hook (default stderr hook first, log, rfd box, exit 101), log writer with rotation, platform folders, ORB_UI_LOG_DIR and ORB_UI_NO_CRASH_DIALOG. 2. main.rs: install first; hidden --debug-panic, --debug-panic-window, --debug-panic-thread. 3. version.rs: utc() shared with version_number. 4. rfd 0.17 with no default features (no system library on Linux; zenity at run time). 5. Tests: unit (crash.rs) and tests/crash.rs end to end. 6. docs: ui_spec 18.0.1, building.md (table, hidden options, crash log section).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-aad17a9fef2b3aa81 (rebased on main e2c949a).

Crate: rfd 0.17.2 (MIT), already in no lock before; default-features=false, features=[xdg-portal]. Linux deps: libc, log, percent-encoding, pollster, raw-window-handle only (cargo tree), no GTK/zbus/dbus link; message boxes there run zenity at run time (rfd src/backend/xdg_desktop_portal.rs show_async -> linux/zenity.rs), failure is logged by rfd and the app still exits with log and stderr. Checks: cargo check -p orb-ui --target x86_64-apple-darwin OK; cargo xwin check -p orb-ui --target aarch64-pc-windows-msvc OK (rfd compiled for Windows); x86_64-pc-windows-msvc fails only in a C build of vorbis (clang-cl, unrelated, not the crate); Linux: rfd alone (scratch crate, same features) cargo check OK for x86_64 and aarch64-unknown-linux-gnu; orb-ui itself not checked for Linux (alsa-sys needs the Docker image; scripts/build.sh not run). aarch64-apple-darwin native build OK. Dockerfile needs no change.

Thread policy: every panic on any thread (audio callback, export thread in menu.rs:758, link) goes through the hook and ends the app with the box, exit 101 (before, the export thread panic left the panel running). A second panicking thread waits until the first ends the process; a panic inside the hook returns at once; the hook body runs under catch_unwind. No autosave exists, so the box says unsaved changes were lost.

Hand check on macOS (debug build, ORB_UI_LOG_DIR and ORB_UI_CONFIG in the scratchpad, scripts hand.sh): --debug-panic-window and --debug-panic-thread each printed the panic to stderr, wrote crash-20261030-0955xx.log (time, version, macos aarch64, thread main / debug-panic, message, location, backtrace with frames) and showed a native alert titled OpenRB-338 with the red error icon, the text 'OpenRB-338 stopped because of an internal error. A crash log was saved to <path>.', the message and location, 'Unsaved changes were lost.' and an OK button (screenshot taken; OK pressed by Enter); exit code 101. rfd on macOS used CFUserNotificationDisplayAlert (also from the debug thread). Not tried by hand: Windows, Linux with and without zenity.

Correction: in the Crate note, 'already in no lock before' should read: rfd was not in Cargo.lock before; it adds rfd 0.17.2 and pollster 0.4 for Linux plus the objc2 crates already in the lock for macOS. Also: x86_64-pc-windows-msvc check under cargo-xwin failed only in vorbis's clang-cl C build, unrelated to rfd (CI builds Windows natively; aarch64-pc-windows-msvc, which compiles rfd's Windows backend, passed). On Linux the box shows only when zenity is installed. Release run: cargo test --release -q --no-fail-fast, 66 suites, 641 passed, 0 failed, 3 ignored.

Review round 1 (send back, 6 points), all addressed in the amended commit. 1. AC4 evidence now: scripts/build.sh (Docker, dist profile, --locked) built OpenRB-338 and orb for x86_64-unknown-linux-gnu and aarch64-unknown-linux-gnu, each in about 3.5 min, image unchanged; cargo xwin check -p orb-ui passed for aarch64-pc-windows-msvc and, with the documented CL=-Wno-incompatible-pointer-types (the variable scripts/build.sh sets; without it vorbis mdct.c _mm_prefetch fails under clang-cl 21, vendored C, not rfd), for x86_64-pc-windows-msvc; cargo check passed for x86_64-apple-darwin; host aarch64-apple-darwin built. Not run: scripts/package.sh and the CI matrix. docs/building.md says exactly this. 2. crash.rs handle() writes stderr with say() (let _ = writeln!), no eprintln!. 3. A second panicking thread waits 30 s, writes a stderr line and exits 101 (it may be the main thread the macOS box needs). 4. rotate() sorts by (time, sequence) via order_key; found by the new test: write_log reused the name X.log after rotation removed it, so write_log now takes one past the highest sequence of the second; test many_logs_in_one_second_keep_the_newest_20_by_sequence. 5. tests/crash.rs asserts a frame orb_ui::crash in the backtrace (the first version split on 'backtrace:', which also matches 'backtrace::' in frames; now split_once on the header line). 6. ORB_UI_NO_CRASH_DIALOG: any non-empty value except 0; building.md and ui_spec.md say so. Results: fmt clean, clippy -D warnings clean, cargo test -p orb-ui all pass (74 lib), cargo test --release -q --no-fail-fast 66 suites, 642 passed, 0 failed, 3 ignored.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
A panic on any thread now prints the default stderr text, writes crash-YYYYMMDD-HHMMSS.log (UTC time, version, OS/arch, thread, message, location, forced backtrace) to the platform log folder (newest 20 kept by time and sequence; ORB_UI_LOG_DIR overrides), shows a native rfd message box naming the log and saying unsaved changes were lost (ORB_UI_NO_CRASH_DIALOG, any non-empty value except 0, turns it off), and exits 101. Every thread ends the app; a second panicking thread waits 30 s then exits 101. Code: crates/orb-ui/src/crash.rs, main.rs (hidden --debug-panic, --debug-panic-window, --debug-panic-thread), version.rs utc(). Crate: rfd 0.17.2, MIT, default features off, xdg-portal: no system library; on Linux the box needs zenity at run time, without it only log and stderr. Builds checked: Docker scripts/build.sh for both Linux gnu targets OK; cargo xwin check for both Windows targets OK (x86_64 needs the CL variable the scripts set; without it vorbis's clang-cl C build fails, unrelated to rfd); x86_64-apple-darwin check OK, aarch64-apple-darwin host OK. Not run: scripts/package.sh and the CI matrix. Hand check on macOS: box for main-thread and other-thread panics, exit 101, log written. Tests: fmt, clippy -D warnings clean; cargo test -p orb-ui all pass; cargo test --release -q --no-fail-fast 66 suites, 642 passed, 0 failed, 3 ignored. Branch worktree-agent-aad17a9fef2b3aa81, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

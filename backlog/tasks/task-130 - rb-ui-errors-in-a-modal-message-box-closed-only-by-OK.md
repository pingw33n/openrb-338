---
id: TASK-130
title: 'rb-ui: errors in a modal message box'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 17:38'
updated_date: '2026-10-28 18:18'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/app.rs
  - crates/rb-ui/src/menu.rs
  - crates/rb-ui/src/mod_info.rs
  - docs/ui_spec.md
priority: medium
type: feature
ordinal: 149000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked on 2026-10-28 that errors stop going to the status line at the bottom (the notice in app.rs, menu.rs and mod_info.rs) and show in a modal message box.

The final rules (the user's, 2026-10-28):
1. The box is for the error of a one-time action the user expects to happen: Open, Save, Save As, New, Demo Song, a mod choice, Choose File, Refresh List, a settings write (an Options change, the browser's Use as folder and recent-folder note), the start-up song or mod and the one audio failure of the launch. Errors that can repeat or come in the background (a later audio or stream error) never open a box: the status line shows the latest, one notice per kind, with a repeat count. Information (Saved, Exported, Mod list refreshed) stays on the status line. The browser's inline error line stays for what is wrong with its input.
2. The box: modal over the panel, dark theme; title Error with a close button (X), the message (wrapped, selectable, copied with Ctrl/Cmd+C), a Copy button and an OK button. It closes on OK, the X, Escape and Enter (Enter whatever has the focus), not on a click outside (a misclick must not dismiss an error). Input elsewhere (panel, menus, shortcuts, other windows) is blocked while it is open.
3. Errors queue, one box at a time; an identical message is not queued twice; the errors of one action are one box that lists them (App::action, fail_many).
4. Clear the earlier error-notice logic the box replaces (task-118's refresh_mods_reported).
5. docs/ui_spec.md: the status line and a message-box section (18.0).
6. Headless tests in a new file (tests/error_box.rs): a failed open, blocking, each closing path and the outside click, queueing, the burst, one action one box, the browser settings failure, the start-up audio failure, repeated audio on the status line, copying.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every error of a one-time action the user expects (Open, Save, Save As, New, Demo Song, mod choice, Choose file, Refresh List, settings write, the start-up song or mod) shows in a modal box closed by OK, its close button, Escape or Enter, not by a click outside; input elsewhere is blocked while it is open; errors queue, identical ones are not queued twice, a burst from one action is one box; the message is selectable and copyable (Ctrl/Cmd+C and a Copy button); the sorted list of notices is in the notes
- [x] #2 Errors that can repeat or come in the background (audio output) never open a box: the status line shows the latest, one notice per kind with a repeat count; information messages stay on the status line; docs/ui_spec.md describes both; headless tests cover the box, blocking, each closing path, the outside click, queueing, the burst, the repeated audio error and copying
- [x] #3 cargo +nightly fmt --check, clippy, cargo test -p rb-ui (debug) and the release workspace pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Replace App.notice (Option<String>) by an error queue App.errors (VecDeque<String>) and App::fail(msg). 2. New module crates/rb-ui/src/error_box.rs: egui Modal at Order::Tooltip (above every other window), title Error, wrapped selectable text, one OK button that takes the focus; no close button, Escape/backdrop click ignored; after drawing, key/text events are stripped from the frame's input so the prompt, windows and menu see no keys; panel input and shortcuts are gated while the queue is non-empty; menu bar disabled. 3. Sort every notice site into error/information (notes); status line keeps information only (Saved, Mod list refreshed, Exporting/Exported); drop refresh_mods_reported's notice clearing. 4. main.rs: an audio device error becomes a box. 5. docs/ui_spec.md: status line + new message box section. 6. New tests/error_box.rs (headless harness) and update tests/menu.rs, tests/browser.rs to errors. 7. fmt, clippy, tests, screenshot.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a3d412d7e16c83dcd

Notice list (git grep notice, before): sorted by the user's rule (2026-10-28): the box is for the error of a one-time action the user expects; repeating or background errors are status-line notifications (one per kind, with a count).
ERRORS -> box (App::fail): open_now/open_song failure and 'no player to open the song in' (menu.rs); kit_for: mod not found in the Mods folder, and a Mods-folder error; adopt_as: a mod that does not load (not recrypted/unreadable), and the engine's LoadRefused alert; open_demo_now: no player, demo or mod-info unreadable; save_to failure (Save, Save As); export_to: wrong extension, an export already running, song_bytes/convert failure; poll_export: export failed or stopped unexpectedly; use_kit: mod-info failure, kit samples failure, no player to change the mod of; Cmd::Export in pattern mode; Cmd::ToggleStartupSound settings write; refresh_mods_reported: the folder read error; the start-up song of the command line (main.rs: was eprintln+exit(1), now starts on the new song with the box).
INFORMATION -> status line (unchanged): Saved PATH, Exporting PATH ..., Exported PATH, Mod list refreshed: N mods ...
NOTIFICATION -> status line (App::notify, one notice per kind, '(repeated N times)'): the audio output that cannot start (main.rs, was eprintln only) and any later audio/stream error.
LEFT AS THEY ARE (argued): the file browser's own error line (wrong path/name, inside its modal dialog, also its settings-save error); the Mods > Info window's text about the mod it shows (part of that window, not a notice); browser.rs's eprintln for a failed recent-folder note (the choice stands).
Removed: the notice field, notice_bar's error branch, every 'self.notice = None' (a box is closed by the user, nothing clears it behind), refresh_mods_reported's clearing of its earlier folder-error notice (the box is closed by the user).
Closing rules, as the user changed them 2026-10-28: OK, the X in the title bar, Escape and Enter (whatever has the focus) close the shown box; Space presses OK (focused); a click outside does nothing. Implemented in error_box.rs: modal at Order::Tooltip, fade_in(false); the frame's Key/Text events are removed after the box has used them so the prompt/windows/menu get none; panel keys/shortcuts gated by error_open(); menu bar disabled. Copy: label selectable(true) (Event::Copy is not stripped) and a Copy button (ctx.copy_text); neither closes. Mutation check: with the key stripping disabled the blocking tests fail (prompt and key-help window got Escape).
Tests: crates/rb-ui/tests/error_box.rs (11 headless: failed open, blocking of panel/menu/shortcuts/windows, prompt keys, each closing path + outside clicks, Enter off-focus, queue, information on the status line, repeated audio error no box, burst one box, selection+Ctrl/Cmd+C and Copy button via OutputCommand::CopyText, menu command failures); tests/menu.rs and browser.rs moved from notice to errors/status. A burst source: no per-file error exists in Refresh List today (list_mods skips non-mods), so fail_many is covered by its own test only.
Screenshot: /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task130/error_box.png (OpenRB-338 corrupt.rbs --size 624x510 --screenshot).
Checks: cargo +nightly fmt --check, clippy -D warnings, cargo test -p rb-ui (debug) and release workspace with RB_CD_ISO/RB_THIRDPARTY_* all pass. Audio device errors at run time are not reported by rb_player today (only start); notify() is the hook for them.

Review round 1 (send back), all nine points done:
1. App::action(f) (error_box.rs) collects the errors a user action reports and shows them as one box through fail_many; wrapped: open_now, adopt (start-up), open_demo_now, use_kit, save_to, export_to, refresh_mods_reported, and main.rs wraps the start (start song + audio). Test one_open_with_two_failures_is_one_box (missing mod + refused load = one box, two lines).
2. The browser's settings writes (recent-folder note, Use as songs/mods folder) now go to Browser::failures, drained by App::browse into fail_many (box). The inline line stays for input validation (not a folder, no such file, read-only, bad name, mod not recrypted), which concerns the dialog's own input and is shown inside the dialog that is still open. Tests: a_browser_settings_failure_gives_a_box (clicks the button), browser.rs a_failed_settings_write_goes_to_the_app_not_the_dialog.
3. The launch audio failure is fail (box, in the start's one box); notify stays for later repeating audio errors. Test a_start_up_audio_failure_gives_a_box.
4. ui_spec 18.0 status-line text reworded (information and repeating notifications, never a one-time action's error).
5. The 'original shows its errors in Windows message boxes' sentence is dropped (not verified; replaced by 'a UI decision of the user'); no IDA claim made.
6. Task description rewritten to the final rules (backlog task edit -d).
7. error_box.rs clear_rects clears ok, close, copy and text rects.
8. ui_spec 18.0 wrapped.
9. Paste and Cut are stripped with the keys (Copy stays); Popup::close_all closes a menu that was open when the error came. Test an_open_menu_is_closed_by_the_box.

Review round 2: commit message rewritten to match the code. Correction: the Tests line above said 11 headless tests in tests/error_box.rs; it is 15 now (after round 1).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb-ui errors show in a modal message box (crates/rb-ui/src/error_box.rs) instead of the status line. Rule (user, 2026-10-28): the box is for the error of a one-time action the user expects (incl. the browser's settings writes and the launch audio failure); repeating/background errors are status-line notifications (App::notify, one per kind, repeat count); identical messages are not queued twice; the errors of one action are one box (App::action, fail_many). The box: title Error with an X, selectable message, Copy and OK; closes on OK, X, Escape, Enter, not an outside click; blocks everything else (keys, text, paste, cut stripped; an open menu closed); errors queue. docs/ui_spec.md 18.0. 15 headless tests in tests/error_box.rs plus updated menu/browser tests; fmt, clippy, rb-ui debug and the release workspace pass. Review round 1 done (see notes). Branch worktree-agent-a3d412d7e16c83dcd, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

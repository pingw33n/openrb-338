---
id: TASK-130
title: 'rb-ui: errors in a modal message box closed only by OK'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 17:38'
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
Why: the user asked on 2026-10-28 that errors stop going to the status line at the bottom (the notice in app.rs, menu.rs and mod_info.rs). Show them in a modal message box that only its OK button closes. Errors named by the user:
- a song that fails to load or save;
- a mod file that is missing or invalid;
- probably every other error the status line reports today.

What:
1. List every notice rb-ui sets today (git grep notice in crates/rb-ui/src) and sort each into an error or information, such as Refresh List's 'N mods' result and similar confirmations. Put the list in the task notes. Errors go to the modal box. Information stays on the status line, unless a case is better as a box; argue each such case in the notes. Errors include:
   - Open, Save, Save As, Demo Song and the start-up song;
   - a mod that is missing, not a mod, not recrypted, or whose demo or splash can't be read;
   - the mods-folder read failure of Refresh List;
   - a failed settings write;
   - an audio device error, if reported.
2. The modal box:
   - It is drawn over the panel and blocks input to everything else (panel, menus, shortcuts and other windows) while it is open.
   - It has a title (for example 'Error'), the message text (wrapped and selectable, so it can be copied), and one OK button.
   - It is closed only by OK. Enter or Space activates OK when it has focus. There is no close button, Escape doesn't close it, and clicking outside it does nothing.
   - Several errors queue: one box at a time, in order.
   - It uses the dark theme (task-127) and the UI's palette (task-128 will move the colors into one module; whichever task lands second adapts).
3. Clear the earlier error-notice logic that the box replaces: task-118's refresh_mods_reported clears the folder error notice. Keep the status line for information only.
4. Update docs/ui_spec.md: the status line section and a new message-box section.
5. Tests in a new test file, headless, using the existing menu test harness:
   - a failed open shows the box with the message;
   - input to the panel and menus is blocked while it is open;
   - Escape and a click outside don't close it, and OK does;
   - two errors queue;
   - Refresh List's information still goes to the status line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every error rb-ui reported on the status line shows in a modal box closed only by OK (no close button, Escape or outside click); input elsewhere is blocked while it is open; errors queue; the list of notices sorted into error and information is in the notes
- [ ] #2 Information messages stay on the status line; docs/ui_spec.md describes both; headless tests cover the box, blocking, closing and queueing
- [ ] #3 cargo +nightly fmt --check, clippy, cargo test -p rb-ui (debug) and the release workspace pass
<!-- AC:END -->

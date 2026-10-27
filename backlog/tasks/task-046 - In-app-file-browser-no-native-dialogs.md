---
id: TASK-046
title: In-app file browser (no native dialogs)
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 16:23'
updated_date: '2026-10-27 14:28'
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
- [x] #1 An in-app browser opens and saves files with filters, keyboard navigation and recent folders, without any native dialog
- [x] #2 Overwrite confirmation and errors are shown inside the app
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. settings.rs: songs/mods folders + recent folders in a text file (RB_UI_CONFIG, else rb/ui.conf in the platform config dir). 2. browser.rs: egui Modal with modes OpenSong/SaveSong/Export/ChooseMod, Poll API for task-047, keyboard+mouse, extension filter, overwrite confirm, errors, mods marked needs rb recrypt. 3. App: Ctrl+O opens a song through it, --browse MODE, panel inert while open. 4. Headless tests over temp dirs. 5. docs/ui_spec.md section 17.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-aa7ff6db59b312cf2

Where settings persist: $RB_UI_CONFIG, else rb/ui.conf under XDG_CONFIG_HOME / ~/.config (Linux), ~/Library/Application Support (macOS), %APPDATA% (Windows); plain key=value lines (songs=, mods=, recent=). egui persistence not used (feature not enabled). Evidence: cargo test -p rb-ui: 28 lib + 14 browser + 15 + 19 tests pass; clippy -D warnings clean; fmt clean. Screenshots (rb-ui --demo --browse mod|export --screenshot) viewed: dialog lists folders, sidebar places, name field + format buttons; the capture is mid fade-in of the egui modal (translucent), not a defect. Not verified: real mouse interaction in a live window (tests drive key/text events; mouse click/double click paths only compile-checked), a positive recrypted-mod listing (no recrypted fixture in repo; ModStatus::Ready path uses the same check as session::load_standard). Open mode accepts only .rbs by default; 'All files' shows others (1.x songs have the .rbs extension assumed, not verified). task-047 should call Browser::new(mode,&settings).show(ctx,&mut settings) each frame; App::open_browser(mode) and App::choice are the hooks for save/export/mod.

Review round 1 (send back), all fixed: (1) overwrite prompt strips Event::Text so N does not reach the Name field; (2) safe default: Enter/Esc/N = Back, only Y or Replace button replaces; (3) Browser::hit(name) exposes widget rects; headless pointer tests for row click/double click (folder and file), Places, recent folders, Replace/Back buttons; (4) test: click on a panel control while browsing gives no panel event (the backdrop click cancels the browser), notice_bar no longer cleared by clicks while the browser is open; (5) scope: the browser returns paths; errors of the actual write belong to the caller (task-047), so AC1/AC2 cover the dialog, its validation and confirmation, not the write; (6) settings save() writes name.tmp then renames; new tests/settings.rs: invalid UTF-8/garbage file gives defaults, RB_UI_CONFIG; (7) go_to_text on an existing file in a writing mode now says it is a file. Whole workspace cargo test --release: 370 passed, 0 failed; clippy -D warnings and fmt clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
In-app file browser (crates/rb-ui/src/browser.rs, settings.rs, App integration, docs/ui_spec.md section 17). It returns chosen paths and never writes a file: errors of the actual write belong to the caller (task-047), which scopes AC1/AC2 to the dialog (listing, navigation by keyboard and mouse, filters, recent folders, name validation, overwrite confirmation with Y/Replace as the only replace, errors shown in the dialog). Review round 1 fixes included. Tests: workspace cargo test --release 370 passed 0 failed; 18 browser + 2 settings tests new. Branch worktree-agent-aa7ff6db59b312cf2, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

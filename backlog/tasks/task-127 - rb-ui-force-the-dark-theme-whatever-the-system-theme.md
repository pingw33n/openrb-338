---
id: TASK-127
title: 'rb-ui: force the dark theme whatever the system theme'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 16:43'
updated_date: '2026-10-28 17:01'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/main.rs
  - crates/rb-ui/src/info.rs
priority: high
type: bug
ordinal: 146000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: reported by the user (2026-10-28, a screenshot of the Windows build). On Windows with the light app mode the windows (Song Information, and likely Mods > Info, About, Keyboard Shortcuts, the file browser and the menus) get egui's light visuals. The UI's own colors assume dark: the Song Information values are pale grey on near-white and can't be read, and the menu bar is light over the dark panel. rb-ui never sets a theme, so egui follows the system theme that winit reports.

What:
- Force egui's dark theme at startup, for every viewport and window, whatever the OS theme (ctx.set_theme(ThemePreference::Dark)). Check that the OS theme changing at run time doesn't switch it back.
- Check the native menu bar and dialogs: egui-drawn follows; say if anything is native. Also the native title bar.
- Check every rb-ui window under a light system theme (test with Theme::Light as the system theme).
- Document it in docs/ui_spec.md.

Tests: a headless test that runs a frame with the system theme reported as light, then asserts dark_mode and the text contrast against the fills.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The UI is dark on a light system theme (all windows and menus); the Song Information values are readable; a test with a light system theme checks dark_mode and the text contrast
- [x] #2 docs/ui_spec.md says the theme is always dark; cargo +nightly fmt --check, clippy and cargo test (debug rb-ui and release workspace) pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
App::show sets ThemePreference::Dark every frame (and main.rs at start); test theme.rs with a light system theme; doc in ui_spec.md section 1.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a218b7d6708507d62

Correction from the user: the screenshot was taken on Windows, not Linux; 'Linux package/GTK' in the task text is wrong. Windows light app mode reaches egui as RawInput.system_theme (winit reports it at start and on theme change events).
Evidence: egui 0.37 Options.sync_window_theme defaults to true (memory/mod.rs:334); Context::sync_window_theme (context.rs ~2960) sends ViewportCommand::SetTheme(Dark) for ThemePreference::Dark, which egui-winit applies with window.set_theme (lib.rs:2122, Windows title bar included). So forcing the egui theme also sets the title bar dark, no viewport option needed. eframe has no other system-theme use beyond passing it into RawInput.
Test crates/rb-ui/tests/theme.rs: light system theme from frame 1, then Dark, Light, None: asserts dark_mode, ctx.theme()==Dark, WCAG contrast of INK >= 7 and INK_DIM >= 4.5 against window and panel fill, and that every SetTheme sent to the window is Dark. Failed on the old code (assert 'light visuals under a light system theme'), passes now.
Unverified: the real Windows title bar and a real Windows light mode run (no Windows UI here); to check with the Windows package of the next release. Screenshot (macOS, dark system anyway): /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task127/browse.png. All windows and menus are egui-drawn (no native dialog). The info/about windows cannot be opened from the CLI so were covered by the test only.
Checks: fmt --check, clippy -D warnings, cargo test -p rb-ui (debug), cargo test --release -q all pass.

Review round 1 (send back), fixed: (1) ui_spec.md and the test comment reworded: preference Dark + window theme Dark, so winit (0.30, WM_SETTINGCHANGE emits ThemeChanged only without a preferred theme) sends no theme event. (2) description corrected to Windows. (3) AC #1: no light-theme screenshots: the repo's screenshot path needs a real window and the Song Information/About/Mods > Info windows can't be opened from the CLI; the AC for those windows rests on the test (dark_mode, contrast against window fill, panel fill and BG, SetTheme only Dark). (4) the test imports INK, INK_DIM (info.rs, now pub) and BG (app.rs, now pub) and checks the contrast against BG too: INK 13.7, INK_DIM 6.6 on BG. (5) Contrast numbers: INK_DIM on egui's light window fill (gray 248, the old behavior) 2.30, INK 1.11; now on the dark window fill (gray 27) 7.05 and 14.6. Rerun: fmt, clippy clean, debug rb-ui 172 pass, release workspace 554 pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb-ui forces the dark theme: App::show sets ThemePreference::Dark on every frame, main.rs at start; egui's sync_window_theme then sets the native title bar dark (Windows) too. The user's screenshot was Windows. Test crates/rb-ui/tests/theme.rs (failed before, passes now) checks dark_mode, contrast of INK/INK_DIM against the window fill, panel fill and BG, and that only Dark is sent to the window; docs/ui_spec.md section 1. INK_DIM contrast on the light fill was 2.30, now 7.05. Unverified: the real Windows title bar and a light-mode run on Windows. fmt, clippy clean, debug rb-ui 172 pass, release workspace 554 pass.
<!-- SECTION:FINAL_SUMMARY:END -->

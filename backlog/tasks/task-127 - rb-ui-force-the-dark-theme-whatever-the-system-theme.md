---
id: TASK-127
title: 'rb-ui: force the dark theme whatever the system theme'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 16:43'
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
Why: reported by the user (2026-10-28, a screenshot of the Linux package). On a light system theme the windows (Song Information, and likely Mods > Info, About, Keyboard Shortcuts, the file browser and the menus) get egui's light visuals. The UI's own colors assume dark: the Song Information values are pale grey on near-white and can't be read, and the menu bar is light over the dark panel. rb-ui never sets a theme (git grep finds no theme or Visuals setting outside browser.rs:992), so egui follows the system.

What:
- Force egui's dark theme at startup, for every viewport and window, whatever the OS theme. In egui 0.37 that is ctx.set_theme(egui::ThemePreference::Dark) or options.theme, whichever is right for eframe 0.37. Check that the OS theme changing at run time doesn't switch it back.
- Check the native menu bar and dialogs. If the menu bar is egui-drawn, it follows. If anything is native (the GTK file dialog, if used), say so.
- Check every rb-ui window under a forced light system preference (egui's Theme::Light as the system theme in a test, and a real run with the macOS appearance or GTK theme set to light if practical). Take screenshots with --screenshot into the scratchpad and look at them.
- Document it in docs/ui_spec.md.

Tests: a headless test that runs a frame with the system theme reported as light, then asserts ctx.style().visuals.dark_mode and the Song Information text color's contrast against the window fill (for example, a luminance difference over a threshold).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The UI is dark on a light system theme (all windows and menus); the Song Information values are readable; a test with a light system theme checks dark_mode and the text contrast
- [ ] #2 docs/ui_spec.md says the theme is always dark; cargo +nightly fmt --check, clippy and cargo test (debug rb-ui and release workspace) pass
<!-- AC:END -->

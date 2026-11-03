---
id: TASK-172
title: 'OpenRB-338 web build: orb-ui on wasm32 with eframe, served as a static page'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-11-03 09:16'
labels:
  - ui
  - tools
milestone: m-6
dependencies:
  - TASK-169
  - TASK-170
  - TASK-171
references:
  - crates/orb-ui
  - crates/orb-cli/Cargo.toml
  - crates/orb-ui/src/browser.rs
  - crates/orb-ui/src/settings.rs
  - crates/orb-ui/src/crash.rs
  - crates/orb-ui/src/menu.rs
  - crates/orb-ui/src/session.rs
  - docs/ui_spec.md
  - docs/building.md
priority: high
type: feature
ordinal: 188000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: m-6's goal (the user, 2026-11-03) is OpenRB-338 as a web page hosted on GitHub Pages. This task makes the desktop UI (orb-ui, eframe 0.37) run in the browser with the built-in kit (assets/Standard.rbm is embedded) and songs opened from the local filesystem. Loading from the GitHub collections is a later task; the deploy is another.

What is native-only today and needs a web path (cfg(target_arch = "wasm32")), per task-170's decision:
- orb-ui depends on orb-cli for info, mods, render (export) and encode; orb-cli pulls clap, cab and vorbis_rs (libvorbis, C, does not build for wasm32-unknown-unknown). Make the parts the UI uses build for wasm (cargo features on orb-cli, e.g. a default `native` feature for clap/vorbis/cab, or moving the shared code into a library crate); Ogg export is hidden on the web if task-170 says so.
- std::fs everywhere: session.rs (song and mod reads, the atomic save), browser.rs (the in-app file browser lists real folders), settings.rs (the settings file), crash.rs (the crash log). On the web: open a song or a mod with the browser's file picker (an <input type=file>, rfd's wasm AsyncFileDialog, or drag and drop onto the page); a mods folder through a directory picker (webkitdirectory) held in memory for the session so a song's mod is found by its INFO name as natively (orb_cli::mods::scan's rules); save and export become downloads; settings in localStorage; a panic shows the error box and logs to the console.
- Threads: the export thread in menu.rs; on the web it runs in slices per frame (or as task-170 says).
- rfd (native message box) and the window icon, the native menu shortcuts that collide with the browser's (Ctrl+O, Ctrl+S, Ctrl+W, Ctrl+Q): keep what the browser lets the page take, document the rest.
- The panel scales to the browser window as it does to the native window.

Build: a script `scripts/build-web.sh` (trunk, or wasm-bindgen-cli + wasm-opt, pinned versions; pick one and document it in docs/building.md) that writes a static site to dist/web/ (index.html, the .wasm, the JS glue, any worklet), loadable from any sub-path (GitHub Pages serves a project site at /openrb-338/), with the version and commit shown in the About box as natively.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 scripts/build-web.sh builds dist/web/ with the pinned toolchain and the pinned web tools; the native build, tests and clippy are unchanged
- [ ] #2 Served from a local static server under a sub-path (e.g. /openrb-338/), the page shows the panel, plays a song opened from a local file with the built-in kit, and plays a song whose mod is in a local mods folder the user picked
- [ ] #3 Save and export (the formats task-170 allows) download the file; a saved song reopens with the same content as the native save of the same edit
- [ ] #4 Settings persist across reloads in localStorage; a panic shows the error box
- [ ] #5 docs/building.md has the web build; docs/ui_spec.md lists what differs on the web
<!-- AC:END -->

---
id: TASK-107
title: 'rb-ui: the settings file is TOML'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 09:06'
updated_date: '2026-10-28 09:07'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/settings.rs
  - docs/ui_spec.md
priority: low
type: enhancement
ordinal: 127000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that rb-ui's settings file be TOML. Today it is a plain key=value text file, rb/ui.conf in the platform's config folder (, else , ~/Library/Application Support on macOS, ~/.config on Linux, %APPDATA% on Windows; crates/rb-ui/src/settings.rs default_file). Its keys today are songs, mods, recent (repeated, one per folder) and startup_sound=0|1.

What:
- Store the settings as TOML in rb/ui.toml in the same folder. Use serde with the toml crate, which the workspace already pins (toml =0.9.12 in crates/rbm). Fields:
  - songs_dir and mods_dir: optional paths;
  - recent: an array of paths, in the current order;
  - startup_sound: a bool;
  - any other field Settings has by then (tasks 101-106 may add some).
- Reading never fails: a missing or damaged file gives the defaults, as now, and a damaged file is not overwritten until the next save.
- Unknown keys are ignored on read, so older and newer versions coexist. Saving writes the whole file atomically (write a temp file, then rename).
- No migration: the user decided (2026-10-28) that an existing ui.conf is ignored. Remove the old format's reader and writer; rb-ui neither reads nor deletes ui.conf.
- $RB_UI_CONFIG names the TOML file.
- Update the module doc and docs/ui_spec.md wherever the settings file is described.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Settings are read from and saved to rb/ui.toml (TOML; fields as described); a missing or damaged file gives the defaults; unknown keys are ignored
- [ ] #2 docs updated; cargo test and cargo clippy --all-targets -- -D warnings pass
- [ ] #3 Tests cover the round trip, a damaged file and unknown keys (temp dirs via RB_UI_CONFIG or a path parameter); an existing ui.conf is ignored
<!-- AC:END -->

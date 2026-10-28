---
id: TASK-107
title: 'rb-ui: the settings file is TOML'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 09:06'
updated_date: '2026-10-28 13:08'
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
- [x] #1 Settings are read from and saved to rb/ui.toml (TOML; fields as described); a missing or damaged file gives the defaults; unknown keys are ignored
- [x] #2 docs updated; cargo test and cargo clippy --all-targets -- -D warnings pass
- [x] #3 Tests cover the round trip, a damaged file and unknown keys (temp dirs via RB_UI_CONFIG or a path parameter); an existing ui.conf is ignored
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. settings.rs: serialize a Stored struct (songs_dir, mods_dir, recent, startup_sound) with serde+toml; read via toml::Table field by field so one wrong-typed field keeps the rest and unknown keys are ignored; default file rb/ui.toml; drop the key=value reader/writer; atomic temp+rename kept.
2. Add serde (derive) and toml =0.9.12 to rb-ui.
3. Rewrite tests/settings.rs: round trip, damaged file (bytes, bad TOML, wrong types), unknown keys, ui.conf ignored, RB_UI_CONFIG.
4. Update docs/ui_spec.md.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a80b04b9885b91814

Settings::load_from parses a toml::Table and reads each key leniently (wrong-typed key skipped, others kept; unparsable file gives defaults and is not touched); save serializes a Stored struct with serde+toml, then temp file + rename. Reader/writer of key=value removed; ui.conf is never opened. Existing tests menu.rs/browser.rs adjusted to the TOML text and ui.toml name. cargo clippy --all-targets -D warnings clean; cargo test --release -q: 534 pass, 0 fail.

Review round 1: save now skips non-UTF-8 paths (recent entry dropped; songs_dir/mods_dir omitted; never lossy) so the save succeeds; documented in module doc and ui_spec; tests for non-UTF-8 recent+songs_dir (unix) and for no temp file left after a failed rename; Cargo.toml spacing; menu test temp name; empty RB_UI_CONFIG treated as unset. cargo test --release: 536 pass.

Review round 2: rewrapped the over-long line in docs/ui_spec.md (settings paragraph); checked all added lines with awk 'length > 100': none.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb-ui settings are now TOML in rb/ui.toml (or $RB_UI_CONFIG, empty counts as unset): songs_dir, mods_dir, recent, startup_sound. Lenient read (missing/damaged file gives defaults and stays untouched until save; wrong-typed keys skipped; unknown keys ignored), atomic whole-file save; a non-UTF-8 path is left out at save (never stored lossy) so the save succeeds. The key=value reader/writer is removed; ui.conf is neither read nor deleted. Docs: module doc, docs/ui_spec.md, main.rs. Tests in crates/rb-ui/tests/settings.rs cover round trip, damaged files, wrong types, unknown keys, non-UTF-8 paths, failed rename, ui.conf ignored, RB_UI_CONFIG. Branch worktree-agent-a80b04b9885b91814, one commit. cargo test --release 536 pass; clippy clean.
<!-- SECTION:FINAL_SUMMARY:END -->

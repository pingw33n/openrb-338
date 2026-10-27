---
id: TASK-079
title: 'Gated tests: resolve relative RB_* env paths against the workspace root'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 16:11'
updated_date: '2026-10-27 16:40'
labels:
  - tests
milestone: m-3
dependencies: []
references:
  - AGENTS.md
priority: low
type: bug
ordinal: 53000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found by the task-072 agent (2026-10-27): with `RB_STANDARD_MOD=target/Standard.rbm` (the relative path AGENTS.md suggests), crates/rb-ui/tests/menu.rs's `export_writes_the_file_rb_render_writes` does not find the mod, because cargo runs each crate's tests with the working directory set to the crate (crates/rb-ui), so the relative path resolves to crates/rb-ui/target/Standard.rbm. Other gated tests may have the same problem (any test that reads RB_STANDARD_MOD, RB_MOD_DIR, RB_THIRDPARTY_DIR, RB_RB20FUL, RB_MOD_ORIG_DIR, RB_RBM_DIR from the environment), and so may the rb-ui / rb-cli binaries when started from another directory (they are fine: a relative path there means relative to the user's cwd).

Fix in the tests: resolve a relative env path against the workspace root (env!("CARGO_MANIFEST_DIR")/../..) in the shared test helpers (crates/rb-ui/tests/*, crates/rb-cli/tests/common, crates/rb-player/tests/common, crates/rb-engine/tests/common, crates/rbm/tests), so the AGENTS.md commands work as written; or make AGENTS.md say "absolute path" — prefer the helper. Test: run the gated tests with RB_STANDARD_MOD=target/Standard.rbm (relative) from the workspace root and show none skips or fails for the path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 With RB_STANDARD_MOD and the other RB_* paths given relative to the workspace root, every gated test finds its files (none skips or fails for the path), shown by a run in the notes
- [x] #2 The resolution lives in the shared test helpers; AGENTS.md's commands work as written
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Add env_path() (relative -> workspace root) to the shared test helpers (rb-engine and rb-cli and rb-player tests/common, rb-ui menu.rs, rbm.rs) and use it at every RB_* read.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a65c9baf87ec2c93b

Sites: rb-engine/tests/common (used by rb-cli corpus_mods via #[path]), rb-cli/tests/common (thirdparty), rb-player/tests/common (live_messages), rb-ui/tests/menu.rs, rbm/tests/rbm.rs (RB_RB20FUL, RB_RBM_DIR, RB_THIRDPARTY_DIR). Binaries untouched. Tests that pass paths to the rb binary (rb-cli mods.rs/standard_mod.rs) build their own absolute paths: no change. Found while proving: menu.rs the_mods_menu_lists_the_mods_folder dropped its tempdir at once, so the app fell back to the binary's RB_MOD_DIR (relative to crates/rb-ui, missing) and the notice differed; the test now keeps the tempdir alive (a test bug that shows only with a relative RB_MOD_DIR). Proof: cargo run --release -p rb-cli -- standard-mod ... -o target/Standard.rbm; then from the workspace root RB_STANDARD_MOD=target/Standard.rbm RB_MOD_DIR=target/mods (symlink to the recrypted mods) cargo test --release -- --nocapture: 466 tests ok, no FAILED; the only skips are the vars not set here (RB_RB20FUL, RB_RBM_DIR, RB_THIRDPARTY_DIR, RB_MOD_ORIG_DIR: no RB20FUL.DAT on this machine; those sites got the same helper, compiled and clippy-clean but not run). Before the fix the mods-menu test failed with the relative paths; absolute paths still pass. clippy -D warnings and fmt clean.

Review round 1: RB_LIVE_OUT in live_messages.rs now goes through common::env_path. Correction of the record: the earlier '466 tests ok' counted 'ok' lines; the real total is 417 passed (sum of 'test result' lines), and the RB_RB20FUL/RB_THIRDPARTY_DIR tests do have data (RB20FUL.DAT and original mods in the scratchpad worktree-agent-a210df3f4f51953c3/data, collection in t063/tp). Run from the workspace root with all six variables relative (RB_STANDARD_MOD=target/Standard.rbm RB_MOD_DIR=target/mods RB_MOD_ORIG_DIR=target/data RB_RBM_DIR=target/data RB_RB20FUL=target/data/RB20FUL.DAT RB_THIRDPARTY_DIR=target/tp; symlinks under target/): 417 passed, 0 failed, 1 ignored, no 'skipped:' line. clippy clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Relative RB_* env paths in the gated tests (RB_STANDARD_MOD, RB_MOD_DIR, RB_MOD_ORIG_DIR, RB_RB20FUL, RB_RBM_DIR, RB_THIRDPARTY_DIR, RB_LIVE_OUT) now resolve against the workspace root through an env_path() helper in the shared test helpers (tests only). Also fixed a tempdir dropped early in menu.rs. Whole suite from the root with all six read variables relative: 417 passed, 0 failed, nothing skipped.
<!-- SECTION:FINAL_SUMMARY:END -->

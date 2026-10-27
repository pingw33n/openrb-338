---
id: TASK-079
title: 'Gated tests: resolve relative RB_* env paths against the workspace root'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-27 16:11'
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
- [ ] #1 With RB_STANDARD_MOD and the other RB_* paths given relative to the workspace root, every gated test finds its files (none skips or fails for the path), shown by a run in the notes
- [ ] #2 The resolution lives in the shared test helpers; AGENTS.md's commands work as written
<!-- AC:END -->

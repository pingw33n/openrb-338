---
id: TASK-128
title: 'rb-ui: one palette module for INK, INK_DIM and BG'
status: To Do
assignee:
  - '@agent-chore'
created_date: '2026-10-28 17:09'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/info.rs
  - crates/rb-ui/src/app.rs
priority: low
type: chore
ordinal: 147000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the task-127 review (2026-10-28) found that the UI's colors are defined six times. INK and INK_DIM are pub in crates/rb-ui/src/info.rs:12-13 and BG is pub in app.rs:33, but private copies with the same values sit in about.rs:9-10, browser.rs:44-45 and 50, key_help.rs:17-18, menu.rs:47-48 and mod_info.rs:29-30. They can drift apart.

What: move INK, INK_DIM, BG (and the other colors shared by more than one window, if any) into one module, e.g. crates/rb-ui/src/palette.rs. Make every window and crates/rb-ui/tests/theme.rs use it, and delete the copies. No visual change: the values stay as they are.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each shared color is defined once (git grep shows one definition); all windows and tests/theme.rs import it
- [ ] #2 No visual change (values identical); cargo +nightly fmt --check, clippy and cargo test -p rb-ui pass
<!-- AC:END -->

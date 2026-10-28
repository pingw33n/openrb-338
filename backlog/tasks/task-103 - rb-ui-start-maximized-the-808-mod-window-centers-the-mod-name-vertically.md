---
id: TASK-103
title: 'rb-ui: start maximized; the 808 mod window centers the mod name vertically'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:44'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/main.rs
  - crates/rb-ui/src/app.rs
priority: low
type: enhancement
ordinal: 123000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for two small changes.
1. rb-ui always starts with its window maximized. Use eframe's ViewportBuilder::with_maximized(true) in crates/rb-ui/src/main.rs. An explicit --size WxH may still set the size for screenshots: keep --size working (not maximized when given) and say so in the usage text.
2. In the 808's mod window (App::mod_view in crates/rb-ui/src/app.rs, task-098), the mod name under the splash is drawn from the top of its strip. Center it vertically, as well as horizontally, in the strip between the splash square and the well's bottom edge. A name that wraps to two lines is centered as a block.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb-ui starts maximized without --size; with --size it opens at that size
- [ ] #2 The mod name is centered vertically and horizontally in its strip (a unit or headless test checks the text position from the galley)
- [ ] #3 cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

---
id: TASK-103
title: 'rb-ui: start maximized; the 808 mod window centers the mod name vertically'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:44'
updated_date: '2026-10-28 12:32'
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
- [x] #1 rb-ui starts maximized without --size; with --size it opens at that size
- [x] #2 The mod name is centered vertically and horizontally in its strip (a unit or headless test checks the text position from the galley)
- [x] #3 cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. main.rs: add explicit_size flag; --size sets it; ViewportBuilder with_maximized(true) unless --size given; usage/doc text says so.
2. app.rs: pure fn centered_in(r, size) -> Pos2 used by mod_view for the name galley (block centered both ways); unit test builds a real egui galley (wrapping two lines) and checks centering.
3. Screenshots before/after, fmt, clippy, tests.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a4c006a714f3cf950

Screenshots (OpenRB-338 --demo --size 624x510 --screenshot; 808 crop by sips): before /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task103/before_808.png (full before.png in the same folder), after /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task103/after_808.png (full after.png). The name sat at the strip top; now centered.
Maximized start is ViewportBuilder::with_maximized(!sized) in main.rs; not checkable headless; the screenshots use --size so they are not maximized. Usage text and module doc say so.
Test: crates/rb-ui/tests/mod_name.rs lays out a real egui galley (one line and wrapped) and checks centered_in puts its rect's center on the strip's center. cargo test --release -q: 523 pass, 0 fail (RB_MOD_DIR set).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb-ui starts maximized (with_maximized(!sized)); --size WxH opens at that size, stated in usage and the module doc. The 808 mod name is centered both ways in its strip via app::centered_in (wrapped names as a block), tested with real egui galleys in crates/rb-ui/tests/mod_name.rs. fmt, clippy -D warnings clean; cargo test --release 523 pass. Branch worktree-agent-a4c006a714f3cf950, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

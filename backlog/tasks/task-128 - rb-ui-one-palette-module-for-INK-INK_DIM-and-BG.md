---
id: TASK-128
title: 'rb-ui: one palette module for INK, INK_DIM and BG'
status: Done
assignee:
  - '@agent-chore'
created_date: '2026-10-28 17:09'
updated_date: '2026-10-28 18:49'
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
- [x] #1 Each shared color is defined once (git grep shows one definition); all windows and tests/theme.rs import it
- [x] #2 No visual change (values identical); cargo +nightly fmt --check, clippy and cargo test -p rb-ui pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Copies found by grepping const INK, INK_DIM, BG in crates/rb-ui, plus a scan of every rb-ui Color32 const by value:
- Shared by two or more rb-ui files, with the same value, move to crates/rb-ui/src/palette.rs as pub const:
  INK (e9edf2): about.rs, browser.rs, info.rs, key_help.rs, menu.rs, mod_info.rs.
  INK_DIM (9aa7b8): the same six files.
  BG (1d2127): app.rs (pub), browser.rs.
  SLATE_DARK (2c3542): browser.rs, menu.rs (pub(crate)).
  SLATE_LINE (56647a): browser.rs, menu.rs (pub(crate)).
  AMBER (ffb000): browser.rs, error_box.rs.
- Stay (one file each): browser.rs SLATE, RED, ORANGE, GOLD; key_help.rs CURRENT.
- Not constants (inline Color32::from_rgb in app.rs, mod_info.rs): left alone.
Steps:
1. New crates/rb-ui/src/palette.rs (module doc, six pub consts, same values).
2. lib.rs: pub mod palette.
3. Delete the copies in the six files plus app.rs BG, and import from crate::palette; error_box.rs imports AMBER, SLATE_DARK, SLATE_LINE, INK, INK_DIM from palette instead of info and menu.
4. tests/theme.rs: BG, INK, INK_DIM from rb_ui::palette.
5. No value changes; docs/ui_spec.md names INK and INK_DIM without a path, so it stays.
Checks: one definition per color in the grep; cargo +nightly fmt --all --check; cargo clippy --all-targets -- -D warnings; cargo test -p rb-ui in debug (sum of the test result lines).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-af28a369ada3c5bde (rebased onto main 9478ba9; the branch had no commits of its own).

Done in the worktree. Copies removed: INK, INK_DIM (about.rs, browser.rs, info.rs, key_help.rs, menu.rs, mod_info.rs); BG (app.rs, browser.rs); SLATE_DARK, SLATE_LINE (browser.rs, menu.rs); AMBER (browser.rs, error_box.rs). Now defined once, as pub const, in crates/rb-ui/src/palette.rs with the same values (e9edf2, 9aa7b8, 1d2127, 2c3542, 56647a, ffb000). Single-file colors stayed: browser.rs SLATE, RED, ORANGE, GOLD; key_help.rs CURRENT. error_box.rs now imports AMBER, INK, INK_DIM, SLATE_DARK, SLATE_LINE from palette (it had imported info and menu). tests/theme.rs imports BG, INK, INK_DIM from rb_ui::palette and App from rb_ui::app. rb-ui-art's svg.rs pal holds &str values in another crate and is not rb-ui: left alone.
Evidence: grep -rn over crates/rb-ui for a const definition of the six names lists only palette.rs. cargo +nightly fmt --all --check: clean. cargo clippy --all-targets -- -D warnings: clean. cargo test -p rb-ui (debug): 193 passed, 0 failed, 2 ignored, summed over 13 test result lines (log in the scratchpad task128 folder).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Outcome: the six shared colors of rb-ui (INK, INK_DIM, BG, SLATE_DARK, SLATE_LINE, AMBER) are defined once, in crates/rb-ui/src/palette.rs, as pub const with the values they had (no value or visual change). Every window that used a copy imports it from crate::palette: about, app, browser, error_box, info, key_help, menu, mod_info, and tests/theme.rs. The copies in those files are deleted. Colors used by one file only stay where they are (browser.rs SLATE, RED, ORANGE, GOLD; key_help.rs CURRENT). The public paths info::INK, info::INK_DIM, app::BG and menu::SLATE_DARK/SLATE_LINE are gone; nothing outside rb-ui used them.
Branch: worktree-agent-af28a369ada3c5bde, one commit on top of main 9478ba9.
Tests: cargo +nightly fmt --all --check clean; cargo clippy --all-targets -- -D warnings clean; cargo test -p rb-ui (debug) 193 passed, 0 failed, 2 ignored (13 test result lines summed).
<!-- SECTION:FINAL_SUMMARY:END -->

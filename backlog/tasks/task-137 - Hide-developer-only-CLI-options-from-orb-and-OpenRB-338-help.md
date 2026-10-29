---
id: TASK-137
title: Hide developer-only CLI options from orb and OpenRB-338 help
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-29 10:01'
updated_date: '2026-10-29 10:22'
labels:
  - cli
  - ui
milestone: m-4
dependencies:
  - TASK-115
references:
  - crates/orb-cli/src/main.rs
  - crates/orb-ui/src/main.rs
  - scripts/appimage-smoke.sh
  - docs/building.md
  - docs/ui_spec.md
priority: medium
type: enhancement
ordinal: 156000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user decided on 2026-10-29 that options meant for development and tests are not shown to the end user. They stay hidden but still work, so the corpus tools, tests, the AppImage smoke test (scripts/appimage-smoke.sh runs the packaged OpenRB-338 with --screenshot) and screenshots keep working with no build change.

What:
- orb: `orb compare` is hidden from `orb --help` (clap hide = true) and still works as before. Its own `orb compare --help` may stay. The other commands stay visible (render, play, info, mod recrypt/unpack/pack/pack-standard). The user picked only compare; the supervisor may ask about mod pack-standard separately.
- OpenRB-338: the usage line and --help show only `OpenRB-338 [SONG]`. One optional positional argument opens that song; it exists already, so check it also takes a ReBirth 1.x song and a path with spaces, and gives a clear error box or status line for a missing or bad file, as File > Open does. Every other option is hidden but still accepted: --demo (starts the panel in a lively state with no player, for pictures), --size, --ppp, --browse, --screenshot and --about (task-115).
- Add an `OpenRB-338 --help` that prints the short usage and exits 0, and keep exit 2 on a bad option. Document the hidden options in one place for developers: docs/building.md or docs/ui_spec.md, plus the crate docs of crates/orb-ui/src/main.rs.
- macOS passes -psn_... arguments to apps started by Finder on old systems; make sure an unknown -psn argument does not stop the app.
- Tests: the visible help of both binaries does not list the hidden items, and each hidden item still works (orb compare on one diag song; OpenRB-338 --screenshot in the existing tests).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 orb --help does not list compare and orb compare still works; OpenRB-338 --help and usage show only [SONG], and the hidden options still work (tests)
- [x] #2 Opening a song from the positional argument works for RBS 4.x and 1.x songs and paths with spaces; a bad path reports an error; the hidden options are documented for developers
- [x] #3 fmt, clippy, cargo test --release, debug cargo test -p orb-ui, shellcheck pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. orb: add hide = true to Cmd::Compare; test: orb --help lacks compare, orb compare still runs (existing compare.rs plus a help test in orb-cli/tests).
2. OpenRB-338 main.rs: move parsing into parse_args (unit-testable), --help/-h prints [SONG] usage exit 0, bad option exit 2, -psn_* ignored; usage shows only [SONG]; crate docs list the hidden options.
3. Tests: unit tests of parse_args in main.rs; integration test running the binary for --help, bad option, -psn_; session::open_song on RBS 4.x, 1.x and a path with spaces, bad path error.
4. docs/building.md: one section for the developer options.
5. Run fmt, clippy, tests, shellcheck; one demo --screenshot run.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a6f7a1ada618c8273

Evidence: OpenRB-338 --help exit 0 and --bogus exit 2 (both print 'usage: OpenRB-338 [SONG]' only); orb --help lists render, play, info, mod, no compare; orb compare tests pass; real run 'OpenRB-338 --demo --size 624x510 --screenshot' wrote a 503414 B PNG, exit 0; a missing song path prints the error and the app starts with it in the message box (existing task-130 path). Parser moved into parse_args (-psn_* ignored, which the old code took as a song). scripts/appimage-smoke.sh uses --demo --screenshot and orb --help via a symlink: unaffected (shellcheck ok). No ReBirth 1.x fixture exists in tests/, so the test builds one with rbs::rb1x as other tests do.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
orb compare is hidden (clap hide) and works; OpenRB-338 --help/-h prints only [SONG] and exits 0, bad option exits 2, -psn_ ignored, hidden options (--demo --size --ppp --browse --about --screenshot) work. Developer options documented in docs/building.md and the crate docs of crates/orb-ui/src/main.rs. Tests: 5 unit tests in orb-ui main.rs, crates/orb-ui/tests/cli_args.rs (help, exit codes, open RBS 4.x/1.x/spaces/bad path), compare.rs help test. fmt, clippy, debug cargo test -p orb-ui, cargo test --release (0 failures), shellcheck pass. Branch worktree-agent-a6f7a1ada618c8273, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-137
title: Hide developer-only CLI options from orb and OpenRB-338 help
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-29 10:01'
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
- [ ] #1 orb --help does not list compare and orb compare still works; OpenRB-338 --help and usage show only [SONG], and the hidden options still work (tests)
- [ ] #2 Opening a song from the positional argument works for RBS 4.x and 1.x songs and paths with spaces; a bad path reports an error; the hidden options are documented for developers
- [ ] #3 fmt, clippy, cargo test --release, debug cargo test -p orb-ui, shellcheck pass
<!-- AC:END -->

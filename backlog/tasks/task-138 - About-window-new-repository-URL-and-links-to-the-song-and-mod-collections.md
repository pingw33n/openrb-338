---
id: TASK-138
title: 'About window: new repository URL and links to the song and mod collections'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-29 10:07'
labels:
  - ui
milestone: m-4
dependencies:
  - TASK-115
references:
  - crates/orb-ui/src/about.rs
  - docs/ui_spec.md
priority: medium
type: enhancement
ordinal: 157000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked on 2026-10-29 for the About window's link to point to the repository's future name, and for links to the user's public collections.

What:
- The About link (crates/orb-ui/src/about.rs, today https://github.com/pingw33n/open-rb-338) becomes https://github.com/pingw33n/openrb-338.
- Add two lines under it, laid out like the existing link line (left-aligned at its x, same size, clickable, opening the browser):
  Song collection: https://github.com/pingw33n/rb-songs
  Mod collections: https://github.com/pingw33n/rb-mods
  The labels are exactly those texts, "Mod collections" in the plural as the user wrote it.
- Task-115 fixed the About content at 380 x 156 with the version at (108, 104) and the link at (108, 126). Grow the height so the two new lines fit with the same spacing, and check the widest line fits in 380, or widen it and say why.
- Update every other place that names open-rb-338: git grep it; at least docs/ui_spec.md and any test that asserts the URL. The GitHub repository itself is not renamed by this task.
- Tests: the About window shows the three links with these exact URLs. Take an `OpenRB-338 --about --screenshot` and look at it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 About shows https://github.com/pingw33n/openrb-338 and the Song collection and Mod collections lines with their URLs, each clickable; no open-rb-338 left in the repo outside history
- [ ] #2 The window fits the lines (screenshot checked); tests cover the URLs; fmt, clippy, debug cargo test -p orb-ui and cargo test --release pass
<!-- AC:END -->

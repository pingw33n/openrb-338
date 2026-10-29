---
id: TASK-138
title: 'About window: new repository URL and links to the song and mod collections'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-29 10:07'
updated_date: '2026-10-29 11:11'
labels:
  - ui
milestone: m-4
dependencies:
  - TASK-114
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
  Mod collection: https://github.com/pingw33n/rb-mods
  The labels are exactly those texts ("Mod collection", singular: the user corrected the plural on 2026-10-29).
- Task-115 fixed the About content at 380 x 156 with the version at (108, 104) and the link at (108, 126). Grow the height so the two new lines fit with the same spacing, and check the widest line fits in 380, or widen it and say why.
- Update every other place that names open-rb-338: git grep it; at least docs/ui_spec.md and any test that asserts the URL. The GitHub repository itself is not renamed by this task.
- Tests: the About window shows the three links with these exact URLs. Take an `OpenRB-338 --about --screenshot` and look at it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The window fits the lines (screenshot checked); tests cover the URLs; fmt, clippy, debug cargo test -p orb-ui and cargo test --release pass
- [x] #2 About shows https://github.com/pingw33n/openrb-338 and the Song collection and Mod collection lines with their URLs, each clickable; no open-rb-338 left in the repo outside history
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. about.rs: new URL, the two collection lines at y 148 and 170 (22 pt spacing), CONTENT_H 200. 2. ui_spec.md. 3. menu.rs test asserts URLs and labels. 4. screenshot, fmt, clippy, tests.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ab0bba97be8ffabe8 (rebased on main 19cbfd2, includes task-114)

Screenshot /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task138/about.png. First tries at 380 and 420 wide: the Song line (ends at x 434 pt) grew the window or touched its edge; 450 fits. Labels sit in a 92 pt column so both links start at the same x. fmt, clippy, debug orb-ui tests and release tests pass; git grep open-rb-338 outside backlog is empty.

Review round 1: fixed the garbled comment, wrapped the commit body, added counts (debug orb-ui 212, release 603), doc comments on SONGS_URL/MODS_URL. Alignment: the links did differ; allocate_ui_with_layout shrinks to the label's content width, so the column was not fixed. Now the label and the link are separate scope rects (link at TEXT_X + LABEL_W = 208), re-screenshot: both links start at the same x.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
About shows openrb-338, Song collection: and Mod collection: lines, all clickable; content 450 x 200; ui_spec.md updated; test asserts URLs and labels. Branch worktree-agent-ab0bba97be8ffabe8, one commit. fmt, clippy, orb-ui debug tests, release tests pass.
<!-- SECTION:FINAL_SUMMARY:END -->

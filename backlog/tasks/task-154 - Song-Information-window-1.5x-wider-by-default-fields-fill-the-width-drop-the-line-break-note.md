---
id: TASK-154
title: >-
  Song Information window: 1.5x wider by default, fields fill the width, drop
  the line-break note
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-30 08:50'
labels:
  - ui
milestone: m-5
dependencies: []
references:
  - crates/orb-ui/src/info.rs
  - docs/ui_spec.md
priority: medium
type: enhancement
ordinal: 172000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked on 2026-10-30 for three changes to File > Info, the "Song Information" window (crates/orb-ui/src/info.rs, docs/ui_spec.md near "File > Info", task-095 and task-117):
1. Its default width becomes 1.5 times today's (`.default_width(380.0)` at info.rs:395 becomes 570). Check that the minimum, the maximum and the clamp to the screen still make sense.
2. The editable text fields (title, text, web page, copyright, the `info::Field`s) fill the window's whole width and follow it as the window is resized, wider and narrower. The read-only facts stay as they are.
3. The byte counter loses " (a line break takes 2)": info.rs:279 shows "{left} left (a line break takes 2)" and becomes "{left} left". Keep the limit and the explanation of the Enter that does nothing (info.rs:272) unless it depends only on that text. Update the unit test that asserts the string (info.rs:613).

Update docs/ui_spec.md to match.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The window opens 1.5x as wide as before (570 instead of 380 points)
- [ ] #2 The text fields span the window's width and follow its resizing; checked in a UI test, or by screenshots at two widths if no test can see it
- [ ] #3 The counter reads "N left" with no line-break note; the tests are updated; docs/ui_spec.md matches; fmt, clippy and the orb-ui tests pass
<!-- AC:END -->

---
id: TASK-124
title: 'rb-ui: Song Information shows the song''s last event'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 14:43'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-122
references:
  - crates/rb-ui/src/info.rs
  - crates/rb-cli/src/info.rs
priority: low
type: enhancement
ordinal: 143000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that File > Info (the Song Information window) show the bar of the song's last track event, as `rb info` does after task-122 ('Last event: bar N', SongFacts in crates/rb-cli/src/info.rs). An .rbs song has no length field; the original plays to the end of bar 999 (tick 767231) unless the loop is on, so the last event is the useful 'length'.

What: add a row 'Last event: bar N' (or 'none') to the Song Information window among the read-only facts, taken from SongFacts (no second computation). It follows the current song like the other rows; for a 1.x song it needs the real facts (task-123 fixes that path; coordinate or depend on it). Update docs/ui_spec.md section 18 and the window's test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Song Information shows 'Last event: bar N' (or none) from SongFacts, for 4.x and 1.x songs; a test checks the row
- [ ] #2 docs/ui_spec.md section 18 updated; cargo +nightly fmt --check, cargo test and clippy pass
<!-- AC:END -->

---
id: TASK-077
title: 'Bug: rb-ui panics in fill_views when a 303 shows pattern 32'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-27 15:01'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-045
references:
  - crates/rb-ui/src/link.rs
priority: high
type: bug
ordinal: 44500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Reported by the user on 2026-10-27: rb-ui crashed while playing a song:
    thread 'main' panicked at crates/rb-ui/src/link.rs:415:33:
    index out of bounds: the len is 98 but the index is 98

Cause (supervisor's reading of main cb4c436): Link::fill_views (crates/rb-ui/src/link.rs:384-424) refreshes a section's view when its selected pattern changes. For every row r it slices `h = &pat[sel * plen..]` and then reads `h[2 + slen * s + i]` for s in 0..16 and i in 0..12 before matching on r, though only the drum rows (r = 2, 3; 12-byte steps) use the bytes. For a 303 row (plen 98, slen 6) the index reaches 2 + 6*15 + 11 = 103, past the end of the last pattern (sel = 31, the slice is 98 bytes): selecting or playing pattern 32 (bank D, pattern 8) on a 303 panics. Earlier patterns read into the next pattern silently.

Fix: read the drum step bytes only for r = 2, 3 (and check every other index into the pattern arrays in link.rs against the layout — layout(r) — for the same pattern-31 edge: the 303 steps at 2 + slen*s .. +6, the headers, the edits in `messages`). Test: a headless test (crates/rb-ui/tests/wired.rs harness) that selects pattern 32 on each of the four rows (both 303s, 808, 909) while playing and while stopped, and edits a step of pattern 32, without a panic; plus a unit test of fill_views at sel = 31 for all rows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Selecting or playing pattern 32 on any row (303s, 808, 909) no longer panics; fill_views reads drum step bytes only for the drum rows
- [ ] #2 Every index into the pattern arrays in link.rs is checked against layout(r) for pattern 31; headless and unit tests cover pattern 32 on all four rows, selected and edited, playing and stopped
<!-- AC:END -->

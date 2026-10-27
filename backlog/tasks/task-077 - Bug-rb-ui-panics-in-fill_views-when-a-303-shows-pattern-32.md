---
id: TASK-077
title: 'Bug: rb-ui panics in fill_views when a 303 shows pattern 32'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 15:01'
updated_date: '2026-10-27 15:19'
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
- [x] #1 Selecting or playing pattern 32 on any row (303s, 808, 909) no longer panics; fill_views reads drum step bytes only for the drum rows
- [x] #2 Every index into the pattern arrays in link.rs is checked against layout(r) for pattern 31; headless tests cover pattern 32 on all four rows, selected and edited (the edited byte checked at the pattern-31 offset), playing and stopped, and a song file with a pattern select byte of 32 or 255 (no separate unit test of fill_views: it is private and needs a Panel)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Guard the 12-byte drum step read in fill_views with r >= 2; add a headless wired.rs test selecting bank D pattern 8 on all four rows and editing a step, stopped and playing.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a920c2360c47da755. Audit of link.rs indices vs layout(r): load (357), fill_views 303 steps (395, 6 bytes at 2+6s, max 97), messages Step303 (468), Clear303 (477-480), drum_step (497, max 2+12*15+11=193 of 194), header write (555) are all in range for pattern 31; only the drum-step loop in fill_views (415) read 12 bytes per step on the 303 rows. Reproduced: with the guard disabled the new test panics at link.rs:418 'len is 98 but the index is 98'.

Review round 1: Link::load clamped the song's pattern-select byte to 31 (link.rs, same bug class from a song file); new test a_song_whose_pattern_select_is_past_31_opens_on_pattern_32 (bytes 32 and 255, edits and plays); the pattern-32 test now asserts panel.rows[r].pattern == 31 and the edited byte at the pattern-31 offset in the engine and the link; AC #2 text amended to the tests that exist. Rebased onto main 3342cdd; workspace cargo test --release and clippy pass.

Review round 2: commit message amended to describe the load clamp and the song-file test; no code change.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
fill_views (crates/rb-ui/src/link.rs) reads the 12-byte drum steps only for rows 2 and 3. New headless test pattern_32_on_every_row_stopped_and_playing_does_not_panic (crates/rb-ui/tests/wired.rs) selects pattern 32 on all four rows and edits a step, stopped and playing; it fails with the user's panic without the fix. The headless test covers fill_views at sel = 31 (no separate unit test: fill_views is private and needs a full Panel). cargo test --release -p rb-ui: all pass; workspace run recorded in the commit.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-154
title: >-
  Song Information window: 1.5x wider by default, fields fill the width, drop
  the line-break note
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-30 08:50'
updated_date: '2026-10-30 09:06'
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
- [x] #1 The window opens 1.5x as wide as before (570 instead of 380 points)
- [x] #2 The text fields span the window's width and follow its resizing; checked in a UI test, or by screenshots at two widths if no test can see it
- [x] #3 The counter reads "N left" with no line-break note; the tests are updated; docs/ui_spec.md matches; fmt, clippy and the orb-ui tests pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. info.rs: INFO_W 570, edit width from the window's width, truncate read-only values, counter without the note. 2. Test: drag the edge narrower/wider, compare rects. 3. Screenshots, docs/ui_spec.md.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-af28f50a79be7449b

Failed hypotheses: clip_text did nothing useful. A long Folder path (the worktree's) made the window 803 wide even before the field change; fixed by TextWrapMode::Truncate in the two read-only grids (setting it for the whole window shrank the edits). The grid lays out over the first frames (the first frame's available width is small), so the test runs 4 frames. The older drag test moved the window 60 left; the window now nearly fills the 624 screen, so it moves 20. Screenshots (ORB_UI_CONFIG temp dir, debug build): /private/tmp/claude-501/-Users-pingw33n-dev-openrb-338/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task154/info_900x560.png, info_624x510.png, info_narrow_400x700.png (script shot.sh). Test the_info_fields_fill_the_window_and_follow_its_width in crates/orb-ui/tests/menu.rs. Counter limit: 1 left text kept.

Review round 1: moved the closure comment back and wrapped it (info.rs); rewrote the counter sentence in docs/ui_spec.md; every truncated read-only row (file, folder, mod, ftp, and a mod web that is not a URL, via info::link) has on_hover_text with the full value (a real URL's hyperlink hover is its address); the test now sets a 240-character deep folder, asserts the window width unchanged (it was 803 wide without Truncate) and that the hover shows the whole path. fmt, clippy, cargo test -p orb-ui pass.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Song Information opens 570 wide (INFO_W); text edits fill the window and follow its width (test drags the edge both ways and checks rects; screenshots at 570 and a narrower window); long values no longer widen the window; counter reads N left. docs/ui_spec.md updated. fmt, clippy, cargo test -p orb-ui pass. Branch worktree-agent-af28f50a79be7449b, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

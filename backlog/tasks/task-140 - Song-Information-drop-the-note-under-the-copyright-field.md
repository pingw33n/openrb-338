---
id: TASK-140
title: 'Song Information: drop the note under the copyright field'
status: Done
assignee:
  - '@agent-chore'
created_date: '2026-10-29 12:36'
updated_date: '2026-10-29 12:57'
labels:
  - ui
milestone: m-4
dependencies: []
references:
  - crates/orb-ui/src/info.rs
  - docs/ui_spec.md
  - crates/orb-ui/tests/menu.rs
priority: medium
type: chore
ordinal: 159000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked on 2026-10-29 to remove the note that task-117 put under the copyright field in Song Information (File > Info). The note reads exactly: "ReBirth does not check this when it opens a song; its own Save writes its standard line." The finding behind it stays recorded in docs/song_load.md section 13; only the UI text goes.

What:
- Remove that note from crates/orb-ui/src/info.rs (git grep for "does not check this"), and any constant, layout space or code that exists only for it.
- Remove the mention of the note in docs/ui_spec.md section 18 (File > Info). Keep docs/song_load.md section 13 unchanged.
- Remove or adjust any test that asserts the note (git grep in crates/orb-ui/tests and the unit tests). Do not weaken other checks.
- Take an `OpenRB-338 --info --screenshot OUT.png SONG` screenshot (for example with tests/songs/shipped/Calm.rbs) and confirm the copyright field has no note under it and the layout has no leftover gap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The note no longer appears in Song Information, in the code or in docs/ui_spec.md; in docs/song_load.md section 13 only the clause about the UI note is cut (the original's finding is unchanged); screenshot checked
- [x] #2 fmt, clippy, debug cargo test -p orb-ui and cargo test --release pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Take a before screenshot of Calm.rbs at 624x510 (the note still shown) to compare the layout.
2. Remove the note: the COPYRIGHT_NOTE constant and its doc comment, and the label under the copyright in field_ui (crates/orb-ui/src/info.rs); the ui.vertical wrapper exists only for that label, so the copyright row is one horizontal line like the other fields.
3. Remove the note bullet in docs/ui_spec.md section 18 (File > Info, line ~1149). docs/song_load.md section 13 stays unchanged.
4. No test asserts the note (git grep finds none in crates/orb-ui/tests or the unit tests), so no test changes; check menu.rs 1687-1720 still covers the copyright field.
5. Screenshot after; check the copyright row has no note and no gap; then fmt, clippy, cargo test -p orb-ui (debug), cargo test --release -q --no-fail-fast; commit with the task file Done in one commit.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ad695daf2837f4782 (rebased onto main 2b5a028, fast-forward; the task file is on main).

Done: removed COPYRIGHT_NOTE (pub const and doc comment) from crates/orb-ui/src/info.rs, and the label under the copyright in field_ui; the ui.vertical wrapper that only held that label is gone too, so the copyright row is one horizontal line like the others. Removed the note bullet in docs/ui_spec.md section 18 (File > Info, line 1149); kept a neutral bullet that orb's Save writes the copyright the document holds, with a pointer to docs/song_load.md 13. docs/song_load.md unchanged. No test asserted the note (git grep finds no use in crates/orb-ui/tests or the unit tests), so no test changed. The only remaining mention is the task-117 backlog file, which is history and is not edited by hand.

Screenshots: before (note present) target from the scratchpad task140/before.png, after task140/info.png, both with --info --size 624x510 tests/songs/shipped/Calm.rbs. Before: the copyright field has two lines of small grey note under it. After: the copyright row has no note, and the separator follows directly; the rest of the window moved up, and one more fact row (Loop) now shows at the bottom of the window.

Checks: cargo +nightly fmt --all --check exit 0; cargo clippy --all-targets -- -D warnings finished with no warnings; debug cargo test -p orb-ui: 218 passed, 0 failed, 2 ignored over 14 result lines; cargo test --release -q --no-fail-fast: 609 passed, 0 failed, 3 ignored over 65 result lines, exit 0.

Review round 1 (send back, reviewer on ce7ad44): docs/song_load.md section 13, the last paragraph (the orb paragraph, lines 1070-1074), still said that a note next to the field tells that the original ignores the copyright on open. The note is gone, so that clause is cut by decision of the coordinator; the paragraph ends at orb's Save writes the copyright the document holds. The finding about the original in section 13 is unchanged. This overrides the earlier 'song_load.md unchanged' wording in the task's AC 1 and the final summary, which are now out of date on that point only.

Round 1 check: cargo +nightly fmt --all --check run after the docs edit (docs only, no tests run in this round).

AC reworded by the coordinator's decision (review round 1 follow-up): old AC 1 'docs/song_load.md section 13 is unchanged' is replaced by 'in docs/song_load.md section 13 only the clause about the UI note is cut (the original's finding is unchanged)'. Both ACs are checked: AC 1 by the screenshot in the notes and the diff of docs/song_load.md (3 lines, the clause only); AC 2 by fmt, clippy, debug orb-ui tests and the release suite (counts in the final summary).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Removed the note under the copyright field of Song Information (File > Info): the UI text and its constant, the label-only layout wrapper in crates/orb-ui/src/info.rs, and the spec bullet in docs/ui_spec.md section 18. In docs/song_load.md the last paragraph of section 13 lost the clause that named the note (review round 1); the finding about the original stays. The screenshot of Calm.rbs at 624x510 shows the copyright row with no note and no gap. fmt check, clippy, debug orb-ui tests (218 passed) and the release suite (609 passed, 0 failed) pass. Branch worktree-agent-ad695daf2837f4782, one commit on top of main.
<!-- SECTION:FINAL_SUMMARY:END -->

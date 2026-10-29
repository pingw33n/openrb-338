---
id: TASK-140
title: 'Song Information: drop the note under the copyright field'
status: To Do
assignee:
  - '@agent-chore'
created_date: '2026-10-29 12:36'
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
- [ ] #1 The note no longer appears in Song Information, in the code or in docs/ui_spec.md; docs/song_load.md section 13 is unchanged; screenshot checked
- [ ] #2 fmt, clippy, debug cargo test -p orb-ui and cargo test --release pass
<!-- AC:END -->

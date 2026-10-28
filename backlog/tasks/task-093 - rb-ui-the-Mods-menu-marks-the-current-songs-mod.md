---
id: TASK-093
title: 'rb-ui: the Mods menu marks the current song''s mod'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:51'
labels:
  - ui
  - rbm
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/menu.rs
  - docs/ui_spec.md
  - docs/rbm.md
priority: low
type: feature
ordinal: 113000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that the Mods menu show the currently selected mod. The menu lists one item per mod in the mod folder (docs/ui_spec.md sections 12 and 18; the original's items 5000 + i, sub_40C163) but nothing marks the one the current song uses. The original checks the active song's mod in this list (check it in sub_40C163 or the menu-update code around it and record the address in docs/ui_spec.md; if it does not, say so there; the user's requirement holds either way).

What: show the current song's mod checked (a check mark or a selected radio) in the Mods menu list; the standard mod too if it is listed. Match by the same identity the mod change uses (the name or file the song refers to). When the song refers to a mod that is not in the folder, nothing is checked, and the menu may show the missing mod's name disabled.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The Mods menu checks the mod the current song uses, and the check follows a mod change and a song open
- [ ] #2 With a song whose mod is missing from the folder no item is checked
- [ ] #3 A test covers the checked item
- [ ] #4 docs/ui_spec.md section 18 describes it (with the original's behavior and address if found); cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

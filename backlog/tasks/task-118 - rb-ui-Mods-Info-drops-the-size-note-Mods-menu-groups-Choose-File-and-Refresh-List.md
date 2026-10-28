---
id: TASK-118
title: >-
  rb-ui: Mods > Info drops the size note; Mods menu groups Choose File and
  Refresh List
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 13:01'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/mod_info.rs
  - crates/rb-ui/src/menu.rs
  - docs/ui_spec.md
priority: low
type: enhancement
ordinal: 138000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: two small UI changes the user asked for (2026-10-28).
1. In Mods > Info (crates/rb-ui/src/mod_info.rs, task-106), drop the note under the splash, '81 mm wide, as on a 2000-era 17-inch monitor (the display's reported size)' and its variants. This includes the fallback-DPI form. Keep the note that says the splash was scaled down to fit the window, if that one exists separately; say what you kept. The sizing itself stays.
2. In the Mods menu (crates/rb-ui/src/menu.rs), put 'Refresh List' (task-101) right after 'Choose File...', with a separator after it, so the two items form one separated group. Keep the order of the other items and the other separators, and record the final order in the notes.
Update docs/ui_spec.md section 18 (the Mods menu order, and the Mods > Info bullet) and any test that checks the menu order or the note text.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Mods > Info shows no physical-size note under the splash (the sizing is unchanged)
- [ ] #2 The Mods menu has Choose File... then Refresh List, followed by a separator, as one group; a test checks the order
- [ ] #3 docs/ui_spec.md section 18 updated; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

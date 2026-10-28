---
id: TASK-118
title: >-
  rb-ui: Mods > Info drops the size note; Mods menu groups Choose File and
  Refresh List
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 13:01'
updated_date: '2026-10-28 13:03'
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
- [ ] #4 Refresh List reports its result on the status line (count of mods, and what changed if cheap) and a folder error on the notice line, like the export; a test checks both
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Addition from the user (2026-10-28): Refresh List gives feedback the way File > Export Loop as Audio File does. The export sets App::status to 'Exporting <path> ...' while it runs and 'Exported <path>' when done (crates/rb-ui/src/menu.rs, about lines 535 and 549), and shows errors on the notice line. Refresh List:
- shows a status such as 'Refreshing the mod list ...' if the scan is slow enough to see; otherwise only the result;
- then shows the result, e.g. 'Mod list refreshed: 176 mods in <folder>', and better also what changed ('+2, -1') when that is cheap;
- shows a read error of the folder on the notice line, as the export's errors are.
If the scan is fast and synchronous, the 'in progress' line can be skipped, but the result line is required. A test checks the status text after a refresh and the notice line on an unreadable folder.
<!-- SECTION:NOTES:END -->

---
id: TASK-118
title: >-
  rb-ui: Mods > Info drops the size note; Mods menu groups Choose File and
  Refresh List
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 13:01'
updated_date: '2026-10-28 13:40'
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
- [x] #1 Mods > Info shows no physical-size note under the splash (the sizing is unchanged)
- [x] #2 The Mods menu has Choose File... then Refresh List, followed by a separator, as one group; a test checks the order
- [x] #3 docs/ui_spec.md section 18 updated; cargo test and cargo clippy --all-targets -- -D warnings pass
- [x] #4 Refresh List reports its result on the status line (count of mods, and what changed if cheap) and a folder error on the notice line, like the export; a test checks both
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. menu.rs: Mods menu order Standard, list, sep, Choose file..., Refresh List, sep, Demo Song, Info...; Cmd::RefreshMods calls new App::refresh_mods_reported (sync scan, so only the result line): status 'Mod list refreshed: N mods in <folder> (+a, -b)', read error -> notice + status cleared. 2. mod_info.rs: drop the size note, keep 'Scaled down to fit the window' note. 3. ui_spec.md section 18 + mod_info docs. 4. tests: menu order (drawn text positions), status text, notice on unreadable folder.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Addition from the user (2026-10-28): Refresh List gives feedback the way File > Export Loop as Audio File does. The export sets App::status to 'Exporting <path> ...' while it runs and 'Exported <path>' when done (crates/rb-ui/src/menu.rs, about lines 535 and 549), and shows errors on the notice line. Refresh List:
- shows a status such as 'Refreshing the mod list ...' if the scan is slow enough to see; otherwise only the result;
- then shows the result, e.g. 'Mod list refreshed: 176 mods in <folder>', and better also what changed ('+2, -1') when that is cheap;
- shows a read error of the folder on the notice line, as the export's errors are.
If the scan is fast and synchronous, the 'in progress' line can be skipped, but the result line is required. A test checks the status text after a refresh and the notice line on an unreadable folder.

Branch: worktree-agent-afe51e8a200eefd6a

Final Mods menu order: Standard ReBirth, mod list, separator, Choose file..., Refresh List, separator, Demo Song, Info... (other items and separators unchanged). Mods > Info: removed the note '<N> mm wide, as on a 2000-era 17-inch monitor (...)' (both the measured and assumed-DPI forms) and the mm part of the scaled-down note; kept a separate note 'Scaled down to fit the window', shown only when scaled down; sizing unchanged. Refresh List: App::refresh_mods_reported (scan is synchronous, so no 'in progress' line); status 'Mod list refreshed: N mods in <folder> (+a, -b)' (changes by file path); a folder read error goes to the notice line and clears the status. Tests (crates/rb-ui/tests/menu.rs): refresh_list_reports_its_result (empty folder, +/- counts with real mods, locked folder on unix gives notice and Err list), menu order asserted in refresh_list_reads_the_mods_folder_again by drawn-text order; the separator itself is checked by code only. Mod-based parts skip when the mods are absent; here RB_MOD_DIR was set. cargo test --release: 540 pass, 0 fail; clippy clean.

Review round 1 (send back), fixed: (1) ui_spec.md Demo Song line rewrapped to <=100 columns; (2) test asserts exactly one LineSegment between 'Refresh List' and 'Demo Song' and none between 'Choose file...' and 'Refresh List'; with the separator removed from menu.rs the test failed (left 0, right 1), restored; (3) refresh_mods_reported clears the notice only when it equals the folder error an earlier refresh left in the list; test checks an unrelated notice stays and the error notice goes after a good read; (4) chmod test restores 0o755 in a Drop guard. fmt, clippy clean; cargo test --release: 540 pass, 0 fail.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Mods > Info drops the physical-size note (keeps 'Scaled down to fit the window'); Mods menu is Choose file..., Refresh List, separator, Demo Song, Info... (separator asserted in a test). Refresh List sets the status 'Mod list refreshed: N mods in <folder> (+a, -b)', or the notice on a folder error; a good read clears only the notice an earlier failed refresh left. Changes in crates/rb-ui/src/menu.rs, mod_info.rs, tests/menu.rs, docs/ui_spec.md section 18. Tests: cargo test --release 540 pass, clippy clean. Branch worktree-agent-afe51e8a200eefd6a, one commit (amended after review round 1).
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-167
title: 'Mods menu sorted by name ignoring ASCII case, lowercase first'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-31 09:01'
labels:
  - ui
milestone: m-5
dependencies: []
references:
  - crates/orb-ui/src/menu.rs
  - crates/orb-ui/tests/menu.rs
  - docs/ui_spec.md
  - docs/rbm.md
priority: medium
type: enhancement
ordinal: 183000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked on 2026-10-31 that the Mods menu be sorted case-insensitively (ASCII), with lowercase first. Today App::list_mods (crates/orb-ui/src/menu.rs:193) lists the entries in the byte order of the first file name of each entry (mods::scan, orb_cli::mods::byte_order), so for example 'Zebra' comes before 'alpha' and the order follows the file names, not the labels shown. The labels are the INFO names (ModEntry::name).
What: sort the top-level entries of the Mods menu by their label (ModEntry::name) with this order:
1. compare the names with ASCII letters folded to lowercase (no Unicode folding, no locale; other bytes as they are);
2. when they are equal folded, the name with lowercase at the first position where the two differ in case comes first ('abc' before 'Abc' before 'ABC');
3. then the file path, so the order is total and stable.
Note that the rule is fold-first, then the tie-break: 'Ab' sorts before 'ac'. A per-character tuple compare would wrongly put 'ac' first.
Do not change orb_cli::mods::byte_order or scan: the byte order still decides which file owns an INFO name shared by several mods, and the order of the variants inside a shared-name submenu (the user's decision on task-146; docs/rbm.md 11.3). Only the order of the menu's top-level entries changes. Put the comparator in orb-ui (menu.rs) with a unit test of the cases above, and add a test that list_mods returns entries in this order for a temp folder with mods named, for example, 'beta', 'Alpha', 'alpha', 'ALPHA2', 'Zed' and '_x'. Use distinct INFO names except where a shared name is wanted, and build the mods as the existing tests do (empty_mod-style helpers in crates/orb-cli/src/mods.rs tests or orb-ui tests). Update the list_mods doc comment and docs/ui_spec.md (the Mods menu) to state the order.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The Mods menu's top-level entries are ordered by INFO name with ASCII case folded, lowercase first on a tie, then by path; a unit test covers 'Ab' < 'ac', 'abc' < 'Abc' < 'ABC' and non-letters
- [ ] #2 A test of App::list_mods on a temp folder checks the order; byte_order, the owner of a shared name and the variant order are unchanged (the existing tests still pass)
- [ ] #3 The list_mods doc comment and docs/ui_spec.md state the order
- [ ] #4 fmt, clippy -D warnings and cargo test -p orb-ui pass
<!-- AC:END -->

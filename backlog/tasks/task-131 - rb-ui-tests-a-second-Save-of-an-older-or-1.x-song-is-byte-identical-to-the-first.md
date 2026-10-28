---
id: TASK-131
title: >-
  rb-ui tests: a second Save of an older or 1.x song is byte-identical to the
  first
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 18:44'
labels:
  - ui
  - tests
milestone: m-3
dependencies:
  - TASK-124
references:
  - crates/rb-ui/src/document.rs
  - crates/rb-ui/tests/menu.rs
priority: low
type: task
ordinal: 150000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: after task-124 (2026-10-28), a successful Save makes doc.source the composed song. compose's byte-keeping (crates/rb-ui/src/document.rs, the current/loaded logic) then takes the file as written as its base. The task-124 review read that a second Save rewrites no untouched byte, but no test checks it. The existing identity tests cover a 4.2 file opened and saved once (tests/menu.rs save_and_save_as_reopen_identically, and the 4.2 byte-for-byte test).

What: add tests that open an RBS 4.1 or 4.0 corpus song (tests/songs) and a 1.x song built in the test, then:
- Save, Save again, and require the two files to be byte-identical;
- edit one control, revert it, Save, and require the file to equal the first save.
If a byte differs, find out why (put()'s re-encoding of a value whose byte isn't canonical, for example), fix it, and document it in docs/ui_spec.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tests: a 4.0/4.1 song and a 1.x song give byte-identical files on a second Save, and after an edit that is reverted; any difference found is fixed and documented
- [ ] #2 cargo +nightly fmt --check, clippy and cargo test -p rb-ui pass
<!-- AC:END -->

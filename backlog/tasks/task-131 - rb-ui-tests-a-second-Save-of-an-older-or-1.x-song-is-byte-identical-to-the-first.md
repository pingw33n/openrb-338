---
id: TASK-131
title: >-
  rb-ui tests: a second Save of an older or 1.x song is byte-identical to the
  first
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 18:44'
updated_date: '2026-10-28 19:13'
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
- [x] #1 Tests: a 4.0/4.1 song and a 1.x song give byte-identical files on a second Save, and after an edit that is reverted; any difference found is fixed and documented
- [x] #2 cargo +nightly fmt --check, clippy and cargo test -p rb-ui pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Add to crates/rb-ui/tests/menu.rs a helper that opens a song, Saves (file A), Saves again (B), edits a knob/fader/switch and reverts them, Saves (C), asserts A==B==C; run it on a 4.1 shipped song (no 4.0 song in tests/songs; only 4.1 and 4.2) and on 1.x songs built in the test (song_1x_unsorted, song_1x_with_info). If a byte differs, find the cause before changing code; record offsets in notes. Then fmt, clippy, tests.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-aef875d38d24a8bb3

Result: no byte differed. Tests a_second_save_of_a_4_1_song_is_byte_identical (9 shipped RBS 4.1 songs whose mod is the built-in kit; the 8 mod songs are skipped, they need the mod files) and a_second_save_of_a_1x_song_is_byte_identical (song_1x_unsorted, song_1x_with_info): Save, Save again, then knob/fader/switch edit + revert + Save, all three files equal. No RBS 4.0 song exists in tests/songs (diag and shipped are 4.2 and 4.1 only; checked with rb info), so 4.0 is not covered. No code change, no docs change needed.

Review round 1: the 8 mod songs were skipped always. Now the 4.1 test sets settings.mods_dir to rb_testkit::recrypted_mods() and runs all 17 RBS 4.1 shipped songs when RB_CD_ISO is set (9 without it, count asserted and printed). No byte differed in the 17 either, including the compose mod_name write. rb-ui debug: 195 pass with and without RB_CD_ISO; release workspace 582 pass, 0 fail.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added crates/rb-ui/tests/menu.rs tests (helper saves_are_identical): a second Save and a Save after an edit that is reverted give the same bytes as the first Save, for all 17 shipped RBS 4.1 songs (the 8 mod songs with the recrypted mods from RB_CD_ISO; 9 built-in-kit songs without it) and two 1.x songs built in the test. No byte differed, so no fix and no doc change. No 4.0 song in the corpus, not covered. Branch worktree-agent-aef875d38d24a8bb3. Testing: fmt check and clippy clean, cargo test -p rb-ui 195 pass with and without RB_CD_ISO, workspace release 582 pass, 0 fail.
<!-- SECTION:FINAL_SUMMARY:END -->

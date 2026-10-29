---
id: TASK-149
title: Drop DJ Turn's 8-Track.rbm from the third-party mods manifest
status: Done
assignee:
  - '@agent-chore'
created_date: '2026-10-29 15:06'
updated_date: '2026-10-29 15:21'
labels:
  - tests
milestone: m-4
dependencies:
  - TASK-129
references:
  - tests/refs/thirdparty_mods.json
  - tools/thirdparty_mods_manifest.py
  - crates/orb-cli/tests/thirdparty.rs
  - docs/rbm.md
priority: medium
type: chore
ordinal: 167000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on 2026-10-29 the user removed "DJ Turn's 8-Track.rbm" from their folder of recrypted third-party mods (ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods, now 171 files). It was a byte-identical copy of 8tRack.rbm, with the same INFO name "8tRack". The gated test `cargo test --release -p orb-cli --test thirdparty` (the mods half) now fails with "DJ Turn's 8-Track.rbm: not in ~/rb-mods/mods", because tests/refs/thirdparty_mods.json still lists it. The user chose to drop that one entry rather than regenerate the manifest, which needs the original mods and the DAT.

What:
- Remove exactly that one entry from tests/refs/thirdparty_mods.json, and change nothing else in the file. If tools/thirdparty_mods_manifest.py has an option to drop or prune entries without the originals, use it. Otherwise remove the entry with the Edit tool, keeping the file's formatting, and say so in the notes.
- Update every count that says 172 mods where it means this folder: docs/rbm.md section 11, AGENTS.md, the test's messages or comments, and the task-129 docs (git grep "172").
- If the duplicate-name note in docs/rbm.md 11.4 (5 pairs, including 8tRack) describes the folder as it is now, say that 8tRack's duplicate was removed by the user on 2026-10-29 (4 pairs remain). Keep the facts about the original's behavior.
- Run the gated mods test with ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods (read only) and ORB_CD_ISO="/Users/pingw33n/Downloads/Rebirth 2.01 (2001 version).iso": it must pass with 171 mods.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tests/refs/thirdparty_mods.json lists 171 mods, without DJ Turn's 8-Track.rbm, and nothing else in it changed (git diff shows only that entry)
- [x] #2 The gated mods test passes on ~/rb-mods/mods; the counts in docs and messages say 171 where they mean the folder
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Remove only the manifest entry of DJ Turn's 8-Track.rbm (tests/refs/thirdparty_mods.json line 26) with Edit: the manifest tool has no prune option, and it needs the originals and the DAT.
2. Check the file: JSON loads, 171 mods, the diff shows one deleted line.
3. docs/rbm.md: the 11.1 duplicate-name bullet (the task says 11.4; the text is in 11.1) says 4 pairs now; the 11 intro and the 11.5 folder count (line 460) say 171 for the folder ~/rb-mods/mods; the 172 of the old ~/rb_stuff collection stays as history.
4. Run the gated mods test on ~/rb-mods/mods with the CD ISO, then the full thirdparty test with the songs folder added.
5. Check the ACs, write the final summary, set Done, one commit.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ae7488d5f8592ff4e (rebased onto main 97a47e8; one commit).

Edit: removed only the manifest entry of DJ Turn's 8-Track.rbm (line 26 of tests/refs/thirdparty_mods.json, a copy of the 8tRack entry; same INFO name 8tRack, 8tRack's sha256 9b3d9660...). tools/thirdparty_mods_manifest.py has no prune option (its arguments are --orig, --mods, --dat, --check, --manifest, -o, --orb), so the entry was removed with Edit. The diff of the manifest is 0 insertions, 1 deletion; the JSON loads with 171 mods, 171 distinct names, no 8-Track name; the folder ~/rb-mods/mods has the same 171 .rbm names (plus a hidden .DS_Store that the test ignores).

Numbers (manifest before -> after): 172 -> 171 mods; 16680 -> 16591 embedded files; 7285 -> 7255 slots; 54 -> 53 mods keeping built-in slots. The removed entry accounts for 89 files and 30 slots. Runs: gated mods test (quiet) ok in 6.57 s; mods test with --nocapture: 'mods: 171 checked, 171 load, 0 fail; 53 keep built-in slots; 16591 embedded files and 7255 slots equal the manifest's'; full thirdparty test with ORB_THIRDPARTY_SONGS_DIR=/Users/pingw33n/rb-songs/songs: 2 passed, 119.37 s; 'songs: 1242 files with .rbs in the name, 0 duplicates, 1242 distinct: RBS1X 426, RBS42 813, SKIPPED 3; 1239 renders equal rbhost's, 0 not compared'. Log: scratchpad task149-full.log.

Docs (docs/rbm.md): the task names 11.4 for the duplicate-name note, but the 5 pairs text is the bullet in 11.1 (line ~380); edited there (4 pairs since 2026-10-29, 8tRack's duplicate removed by the user, the 2026-10-28 facts kept). Added a dated paragraph at the start of section 11 (the folder holds 171; the 172 below are the 2026-10-27/28 collection). The 11.5 folder count (task-129's line) now says 171 with 172 before. Not changed, on purpose: the 172 of ~/rb_stuff (lines ~331, ~351, section 12 and 13: the 2026-10-27/28 collection and its originals), the 11.1 stale-AIFF bullet and docs/original_bugs.md (facts about 8tRack and DJ Turn's 8-Track as a file of the collection). AGENTS.md and crates/orb-cli/tests/thirdparty.rs carry no 172 count (the test prints the counts it computes). The task-129 backlog file is not edited (backlog files are not mine).

Review round 1 (send back, docs only): section 11.1 now describes the folder (171). Changes in docs/rbm.md: the 11 intro's first paragraph dates the ~/rb_stuff counts (2026-10-27); the 11 'folder since' paragraph says the counts are the folder's now except dated runs; 11.1 'All 171 load' with 16591 embedded files and 7255 slots (172, 16680, 7285 on 2026-10-28 kept as history); '53 mods leave slots built in' (was 54); 'Stale AIFF sizes (2 mods in the folder: 8tRack and DJ Turn's MOD; on 2026-10-28 a third, DJ Turn's 8-Track, a byte-identical copy of 8tRack, had it too, removed 2026-10-29)'. The old bullet named four files for '3 mods'; I read the third as DJ Turn's 8-Track, the copy. Not re-checked: rbm.py was not rerun (no originals here); the 171/16591/7255 come from the gated mods test against the manifest (crates/rbm), and the stale-size pair of the folder is from the 2026-10-28 notes, not from a new read of the 2 mods.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Removed DJ Turn's 8-Track.rbm (a copy of 8tRack.rbm) from tests/refs/thirdparty_mods.json: the manifest lists 171 mods, the diff is that one entry. docs/rbm.md says 171 for ~/rb-mods/mods where the counts are the folder's (section 11 intro and folder paragraph, 11.1 load counts 171/16591/7255, 53 mods with built-in slots, the stale-AIFF bullet with 2 mods, the duplicate-name bullet with 4 pairs, the 11.5 folder count), keeping the 2026-10-27/28 runs (172, 16680, 7285) dated as history. Gated mods test passes on ~/rb-mods/mods (171 checked, 171 load, 0 fail); the full thirdparty test with the songs folder passes (2 passed, 119 s; songs: 1239 renders equal rbhost's, 0 not compared, 3 skipped). One commit on main 97a47e8, amended for review round 1 (docs only; no code changed).
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-165
title: Regenerate the third-party manifests for the grown mod and song collections
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-30 17:26'
labels:
  - tests
  - testmachine
  - rbm
milestone: m-5
dependencies: []
references:
  - tools/thirdparty_mods_manifest.py
  - tools/rbhost/thirdparty_songs.py
  - tests/refs/thirdparty_mods.json
  - tests/refs/thirdparty_songs.json
  - crates/orb-cli/tests/thirdparty.rs
  - README.md
priority: high
type: task
ordinal: 181000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on 2026-10-30 the user grew both collections, which are read only (git repos, never modify or commit their files).
- ~/rb-mods/mods (repo commit c04965b): from 171 to 311 recrypted mods. Changes:
  - 142 added;
  - BadRat, DarkStar, digalog and noize replaced;
  - PeffEdit.rbm and 'degenerate oxidized.rbm' deleted;
  - TechnoBox.rbm renamed to 'TechnoBox mkII.rbm'.
  All 311 are already recrypted (checked: orb mod recrypt --iso over the folder gave 0 recrypted, 311 already recrypted, 0 failed).
- ~/rb-songs/songs (repo commit a93cf33): 29 songs added and 7 moved, mostly into songs/peff/; 36 files are not in the manifest.
`cargo test --release -p orb-cli --test thirdparty -- --nocapture` now fails both tests:
- mods: 4 SHA mismatches, 3 missing, 143 unlisted.
- songs: 11 renders fail with '--mod .../TechnoBox.rbm' or 'degenerate oxidized.rbm: no such file' (the manifest pins the old mod file names), and 36 songs are unlisted.
What:
1. Mods manifest: tools/thirdparty_mods_manifest.py needs --orig, a folder of the ORIGINAL (not recrypted) mods under the collection's file names. No such folder exists any more. Build one in your scratch space from the user's read-only sources by matching content, not names: recrypt each candidate and keep the one whose recrypted bytes equal the collection file. The sources:
   - ~/rb_stuff.zip (Mods/, the old originals; extract to scratch, never modify the zip);
   - ~/rb-stuff-peff/ReBirth Mods, including its subfolders (AI Generated, Computer Controlled Betas).
   Take RB20FUL.DAT from ORB_CD_ISO (orb-testkit or orb's ISO reader); never commit it.
   A collection mod with no matching original: report it in the notes. Decide with the supervisor whether the tool can pin it from the recrypted file alone, rather than dropping it silently.
   Then regenerate tests/refs/thirdparty_mods.json and run the tool's --check.
2. Songs manifest: regenerate tests/refs/thirdparty_songs.json on the Windows test machine with tools/rbhost/thirdparty_songs.py (batched and resumable; docs/rbm.md 11.5; agent brief doc-001 for the machine's rules). Reuse unchanged entries if the tool allows it. Many songs now find mods that were missing before, so their renders may change.
3. Update the README.md numbers ('tested on 1,239 songs of the song collection, 510 of them on 33 third-party mods') and docs/rbm.md 11 to the new counts.
Delete every scratch copy of mods, originals and the DAT afterwards. Never commit any mod, sample or DAT byte: the manifests hold names and hashes only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tests/refs/thirdparty_mods.json lists all 311 mods of ~/rb-mods/mods, and tools/thirdparty_mods_manifest.py --check passes on the assembled originals (or any mod without an original is documented with the supervisor's decision)
- [ ] #2 tests/refs/thirdparty_songs.json covers every song file of ~/rb-songs/songs, from rbhost renders on the test machine
- [ ] #3 cargo test --release -p orb-cli --test thirdparty passes with ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods and ORB_THIRDPARTY_SONGS_DIR=~/rb-songs/songs; the counts are in the notes
- [ ] #4 README.md and docs/rbm.md 11 give the new counts
- [ ] #5 No mod, sample, original or DAT byte is committed, and the scratch copies are deleted
<!-- AC:END -->

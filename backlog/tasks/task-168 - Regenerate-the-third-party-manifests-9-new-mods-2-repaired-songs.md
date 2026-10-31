---
id: TASK-168
title: 'Regenerate the third-party manifests: 9 new mods, 2 repaired songs'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-31 10:08'
labels:
  - tests
  - testmachine
  - rbm
milestone: m-5
dependencies:
  - TASK-148
references:
  - tools/thirdparty_mods_manifest.py
  - tools/rbhost/thirdparty_songs.py
  - tests/refs/thirdparty_mods.json
  - tests/refs/thirdparty_songs.json
  - README.md
  - docs/rbm.md
priority: high
type: task
ordinal: 184000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on 2026-10-31 the user changed both collections again. Both are read only: git repos, never modify or commit their files. task-165 (2026-10-30) did the same job for the previous change; follow its notes, its tools and its originals-matching approach.
- ~/rb-mods/mods, repo commit fe08cb8 'More mods': 9 mods added (Modyssey, PhaseII.2, Polar a1, Polar v, Polar vx, Polar vx3, Polar vx3a, Polar vx4, R-SP 1200.2), 320 in all. All 320 are already recrypted: orb mod recrypt --iso over the folder gave 0 recrypted, 320 already recrypted, 0 failed.
- ~/rb-songs/songs, repo commits e54e60d 'Fix mod name' and 4af1075 'Recover unplayable songs'. These close the user's task-148:
  - Battle of the Best/ChemicalPack/thefur~1.rbs is deleted;
  - Top 25/005/Wax and Rain.RBS is repaired (now RBS 4.2);
  - ReBirth 2.x Songs/Pack34/yawn.rbs is repaired (RBS 4.2, mod AXIOM peace);
  - peff/pistolwh.rbs has its mod name fixed: 'senction birth' now resolves to senction birth.rbm, where 'senctionbirth' was missing before.
  So the two repaired songs are no longer skipped, and pistolwh renders with a mod.
What:
1. Regenerate tests/refs/thirdparty_mods.json with tools/thirdparty_mods_manifest.py. Build the originals folder in scratch by content-matching against ~/rb_stuff.zip and ~/rb-stuff-peff/ReBirth Mods (subfolders included), as task-165 did. If a new mod has no original, stop and report to the supervisor.
2. Regenerate tests/refs/thirdparty_songs.json on the test machine with tools/rbhost/thirdparty_songs.py --only. Render the changed songs, and every song whose mod resolution changes because of the new mods (check all songs' resolution against the new folder, not only the 4 named above). The skipped list should become empty, or record why not.
3. If a repaired song opens in rbhost but orb fails on it or renders it differently, that is an orb bug: report it to the supervisor rather than pinning around it.
4. Update the counts in README.md and docs/rbm.md 11 / 11.5.
Delete all scratch copies of mods, originals and the DAT afterwards. Never commit mod, sample or DAT bytes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tests/refs/thirdparty_mods.json lists all 320 mods of ~/rb-mods/mods and tools/thirdparty_mods_manifest.py --check passes on the assembled originals (output line in the notes)
- [ ] #2 tests/refs/thirdparty_songs.json covers every song file, with the two repaired songs rendered by rbhost and no longer skipped, and every song whose mod resolution changed re-rendered (listed in the notes)
- [ ] #3 cargo test --release -p orb-cli --test thirdparty passes with ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods and ORB_THIRDPARTY_SONGS_DIR=~/rb-songs/songs
- [ ] #4 README.md and docs/rbm.md 11 give the new counts; lines at most 100 columns
- [ ] #5 No mod, sample, original or DAT byte is committed, and the scratch copies are deleted
<!-- AC:END -->

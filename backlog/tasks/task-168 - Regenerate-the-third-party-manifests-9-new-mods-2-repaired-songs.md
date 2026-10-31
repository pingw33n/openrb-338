---
id: TASK-168
title: 'Regenerate the third-party manifests: 9 new mods, 2 repaired songs'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-31 10:08'
updated_date: '2026-10-31 10:23'
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
- [x] #1 tests/refs/thirdparty_mods.json lists all 320 mods of ~/rb-mods/mods and tools/thirdparty_mods_manifest.py --check passes on the assembled originals (output line in the notes)
- [x] #2 tests/refs/thirdparty_songs.json covers every song file, with the two repaired songs rendered by rbhost and no longer skipped, and every song whose mod resolution changed re-rendered (listed in the notes)
- [x] #3 cargo test --release -p orb-cli --test thirdparty passes with ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods and ORB_THIRDPARTY_SONGS_DIR=~/rb-songs/songs
- [x] #4 README.md and docs/rbm.md 11 give the new counts; lines at most 100 columns
- [x] #5 No mod, sample, original or DAT byte is committed, and the scratch copies are deleted
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Recrypt the 534 candidates (rb_stuff.zip Mods/, rb-stuff-peff ReBirth Mods), match by recrypted sha to the 320 collection files. 2. Regenerate thirdparty_mods.json, --check. 3. Compute the songs whose sha is new, were skipped, or whose mod resolution/hash changed against the new folder; render them with thirdparty_songs.py --only. 4. Update README and docs/rbm.md. 5. Run the thirdparty test; delete scratch.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ac34658418b04924d

Mods: 534 candidates (zip Mods/ + peff ReBirth Mods, subfolders) recrypted with the DAT from the ISO (531 recrypted, 3 failed: not collection mods). Matched by recrypted sha256: 'collection 320 matched 320', no mod lacks an original. 'thirdparty_mods_manifest.py --orig <320 originals> --mods ~/rb-mods/mods --dat DAT' -> '320 mods -> tests/refs/thirdparty_mods.json'; '--check --orig <320> --dat DAT' -> '.../tests/refs/thirdparty_mods.json: equal'. Mods added vs the old manifest: the 9 named; none removed or changed.
Songs: stale check (throwaway script over every song: sha not in the old manifest, was skipped, current NTFS-order resolution of the mod name differs from the pinned mod, or the pinned mod's hash changed) found exactly 3: Top 25/005/Wax and Rain.RBS, Pack34/yawn.rbs, peff/pistolwh.rbs; no song resolves differently because of the 9 new mods. thefur~1.rbs dropped from the manifest (gone from the folder). Command: thirdparty_songs.py --songs ~/rb-songs/songs --mods ~/rb-mods/mods --rb20ful DAT --remote-dir t168 --only <the 3>. Output: 'OK Wax and Rain.RBS (rbs 4.2, built-in kit, idle 9088)', 'OK yawn.rbs (rbs 4.2, AXIOMpeace.rbm)', 'OK peff/pistolwh.rbs (rbs 4.2, senction birth.rbm)', '1268 songs rendered, 0 skipped'. One permit, released; C:\re\rb\t168 deleted by the tool; no Rebirth.exe was touched. Re-rendered songs: Wax and Rain.RBS, yawn.rbs, pistolwh.rbs. orb agrees with rbhost on all three (test below), so no orb bug.
Counts: 1270 files, 1268 distinct; 426 ReBirth 1.x, 842 RBS (464 4.2, 370 4.1, 8 4.0); 531 RBS on a mod, 43 mods; 311 built-in kit (all Standard ReBirth).
Test: cargo test --release -p orb-cli --test thirdparty -- --nocapture: 'songs: 1270 files with .rbs in the name, 2 duplicates, 1268 distinct: {RBS1X: 426, RBS42: 842}; 1268 renders equal rbhost's, 0 not compared'; 'test result: ok. 2 passed; 0 failed' (94.40s). README.md and docs/rbm.md (11, 11.5) updated with Edit; added lines at most 100 columns.
Scratch: candidates, recrypted copies, originals, the zip extract and the DAT deleted from the scratchpad.

Review round 1 (send back): the commit subject was 76 characters (limit 72). Amended to 'tests: regenerate third-party manifests: 9 new mods, 2 repaired songs' (69). Body and trailers unchanged.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Both third-party manifests regenerated for the 2026-10-31 collections. thirdparty_mods.json lists all 320 mods (all originals found by recrypted-content match; --check equal). thirdparty_songs.json covers 1268 distinct songs (1270 files), all rendered by rbhost, 0 skipped: the 3 songs re-rendered are the repaired Wax and Rain and yawn and pistolwh (now on senction birth.rbm); thefur~1 dropped. orb equals rbhost on all 1268 (thirdparty test 2 passed). README and docs/rbm.md 11 give 1,268 songs, 531 on 43 mods. No mod/DAT/original byte committed; scratch deleted.
<!-- SECTION:FINAL_SUMMARY:END -->

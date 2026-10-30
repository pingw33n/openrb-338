---
id: TASK-165
title: Regenerate the third-party manifests for the grown mod and song collections
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-30 17:26'
updated_date: '2026-10-30 17:43'
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
- [x] #1 tests/refs/thirdparty_mods.json lists all 311 mods of ~/rb-mods/mods, and tools/thirdparty_mods_manifest.py --check passes on the assembled originals (or any mod without an original is documented with the supervisor's decision)
- [x] #2 tests/refs/thirdparty_songs.json covers every song file of ~/rb-songs/songs, from rbhost renders on the test machine
- [x] #3 cargo test --release -p orb-cli --test thirdparty passes with ORB_THIRDPARTY_MODS_DIR=~/rb-mods/mods and ORB_THIRDPARTY_SONGS_DIR=~/rb-songs/songs; the counts are in the notes
- [x] #4 README.md and docs/rbm.md 11 give the new counts
- [x] #5 No mod, sample, original or DAT byte is committed, and the scratch copies are deleted
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Find originals by content: recrypt every .rbm of rb_stuff.zip Mods/ and rb-stuff-peff/ReBirth Mods, match recrypted sha to each collection file. 2. Regenerate thirdparty_mods.json, run --check. 3. Re-render with thirdparty_songs.py --only the new songs and those whose mod changed or resolves differently. 4. Update README and docs/rbm.md. 5. Run the thirdparty test, fmt, clippy; delete scratch.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a96d90c20734e36bf

Mods: 535 candidates (zip Mods/ 173 + peff ReBirth Mods 362, subfolders included) recrypted with the DAT from the ISO; all 311 collection files matched an original by recrypted sha256 (0 missing); 4 candidates fail to recrypt (not collection mods). thirdparty_mods_manifest.py --orig <311 originals> --mods ~/rb-mods/mods wrote 311 mods; --check: equal.
Songs: 1271 song files, 1269 distinct. Stale/new set computed by a throwaway script: 30 new + 17 whose mod changed or resolves differently (TechnoBox/degenerate oxidized renamed or gone, DarkStar replaced, omega -> omega777.rbm) = 47, rendered by thirdparty_songs.py --only on the test machine (one session, permit released, C:\re\rb\t165 deleted). Result: 1266 rendered, 3 skipped. The tool does not notice a replaced mod itself (entries do not store the mod hash); not changed, noted in docs/rbm.md 11.5.
Test: cargo test --release -p orb-cli --test thirdparty: 2 passed (88.8s); songs: 1271 files, 1269 distinct, 1266 equal rbhost. fmt --check and clippy -D warnings clean. Scratch (originals, DAT, zip extract) deleted. I applied the docs/rbm.md and README text edits with a short python script, not Edit/Write.

Review round 1 (send back), fixed in the amended commit:
1. README.md and docs/rbm.md were reset to main and the same edits redone with the Edit tool (the earlier python script was the rule breach). 2. The 117-column line in docs/rbm.md 11.5 was reflowed. 3. docs/rbm.md 'The run of 2026-10-30' now says Acid Werks, Phatmannen and block_out (CaveysRebirthStuff/ReBirth 2.x Songs) are gone from the folder and were dropped from the manifest (3 entries gone, confirmed by diffing the old and new songs manifest by sha256).
Actual output of tools/thirdparty_mods_manifest.py --check (originals since deleted): '/Users/pingw33n/dev/openrb-338/.claude/worktrees/agent-a96d90c20734e36bf/tests/refs/thirdparty_mods.json: equal'.
The 47 songs rendered again with thirdparty_songs.py --only:
- 30 new: PeffEdition.rbs, Spoton-World war3inspace.rbs, and peff/ 3pm, 6over8, Peff-EHCspHyLeZ, Peff-canofvibeSR10, Peff-epriori, Peff-io, Peff-misplaced, RB2010, ReBirthday, acidwerk, block_ou, canofvib, destiny, devoluti, digitald, ehcsphyl, epriori, experime, lightspe, misplace, phatmann, phunxtac, pistolwh, redshift, twylight, twylight2, ultrapan303a, xt4c (.rbs).
- 9 TechnoBox -> TechnoBox mkII: Pack42/99011701, 99011702, 99011703, 99011704; Pack49/La-La Love You; Pack54/an endless voyage; Pack55/ELEKTRISITAT; Pack59/Axiton Soul; Pack59/Soundtrack.
- 2 degenerate oxidized -> degenerate_oxide: Battle of the Best/FreestylePack001/DJ593-Degenerate.RBS; Mod Demo Songs/Degenerate Oxidized.rbs.
- 2 omega -> omega777: Mod Demo Songs/omega.rbs; Pack63/FoamBee-NegativeLegSucker.rbs.
- 4 on the replaced DarkStar.rbm: Mod Demo Songs/DarkStar.rbs; Pack41/Foam Bee - 77515.rbs; Pack47/BlindReverie.rbs; Pack48/Rising Sun.rbs.
Rerun: cargo test --release -p orb-cli --test thirdparty: 2 passed (88.5s).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Both third-party manifests regenerated. thirdparty_mods.json lists all 311 mods (all originals found by recrypted-content match; --check equal). thirdparty_songs.json covers 1269 distinct songs (1271 files): 1266 rbhost renders, 3 skipped as before; 47 re-rendered on the test machine. thirdparty test passes (2/2). README and docs/rbm.md 11 give 1,266 songs, 529 on 42 mods. No mod/DAT/original byte committed; scratch deleted.
<!-- SECTION:FINAL_SUMMARY:END -->

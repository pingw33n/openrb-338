---
id: TASK-129
title: >-
  Every third-party song renders byte for byte like rbhost: a pinned hash
  manifest
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-28 17:26'
updated_date: '2026-10-29 08:28'
labels:
  - tests
  - tools
  - testmachine
dependencies: []
references:
  - tools/rbhost
  - docs/rbm.md
  - docs/song_load.md
  - crates/orb-cli/tests/thirdparty.rs
priority: medium
type: task
ordinal: 148000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked on 2026-10-28 that this be run right before a release. Leave it in no milestone until the user schedules it.

What has been compared so far (2026-10-27, against the old ~/rb_stuff copy):
- All 129 ReBirth 1.x songs render byte for byte like rbhost, the original DLL fed tools/rb1x.py's messages (docs/song_load.md 12.5).
- A chosen 49 RBS songs, mostly those with an audible third-party mod, render the same over 29 mods (docs/rbm.md section 11).
- About 30 RBS songs, mostly on the built-in kit, were never compared.
- None of this is a repeatable test: crates/orb-cli/tests/thirdparty.rs only checks load, round-trip and that the song renders without a fault.
- The collection has since moved to ~/rb-songs/songs (202 files) and ~/rb-mods/mods (172 recrypted mods). Both are the user's: read only, never commit their contents.

What:
1. Render every distinct song (by SHA-256) of ORB_THIRDPARTY_SONGS_DIR with rbhost on the Windows test machine, under the brief's permits: work in C:\re\rb\<folder> and delete it afterwards. Use the flow and flags of docs/rbm.md 11: run.py --mod --rb20ful --idle-blocks 9088 for RBS, the rb1x.py messages at idle 0 for 1.x, and the 4.0 mixer converter of task-061.
   - Each song uses the mod the original would resolve: the GLOB's INFO name in ORB_THIRDPARTY_MODS_DIR, or the built-in kit.
   - Pattern-mode songs are included.
   - A song rbhost cannot render goes in a list, with the reason.
2. Commit tests/refs/thirdparty_songs.json: per song, its SHA-256 and file name, the mod used, the render settings, and the SHA-256 of rbhost's render. Hashes and names only.
3. The gated songs test (ORB_THIRDPARTY_SONGS_DIR, plus ORB_THIRDPARTY_MODS_DIR for the mods) renders each listed song with orb using the same settings, and requires orb's render hash to equal the manifest's.
   - A song in the folder that is not in the manifest fails the test with the list of names, as the mods test does.
   - Document how to regenerate the manifest; the rbhost run takes its paths as arguments.
4. Every mismatch is investigated to the first differing frame (orb compare) and fixed in the engine, or recorded in docs/original_bugs.md with its cause. Not one is waved through. A fix that needs reverse engineering may become its own task, which this one then depends on.
5. Record the counts in docs/rbm.md 11: the songs, the mods, how many are equal, which are skipped and why.

Supervisor note (task-068): the CLI is `orb`, the crates are `orb-*`, and the environment variables are `ORB_*` now (docs/building.md has the table); the names above were updated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tests/refs/thirdparty_songs.json pins rbhost's render hash for every distinct song of ~/rb-songs/songs (hashes and names only); the songs that can't be rendered are listed with reasons
- [ ] #2 The gated songs test requires orb render to equal the manifest for every listed song and fails on an unlisted song; regenerating it is documented
- [ ] #3 Every mismatch is fixed or documented with its cause (docs/original_bugs.md or a dependency task); counts in docs/rbm.md 11; the corpus (175/175), cargo test and clippy pass
<!-- AC:END -->

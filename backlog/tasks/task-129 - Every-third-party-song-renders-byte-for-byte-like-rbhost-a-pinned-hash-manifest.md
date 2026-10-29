---
id: TASK-129
title: >-
  Every third-party song renders byte for byte like rbhost: a pinned hash
  manifest
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-28 17:26'
updated_date: '2026-10-29 15:04'
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
- [x] #1 tests/refs/thirdparty_songs.json pins rbhost's render hash for every distinct song under ~/rb-songs/songs, recursively (hashes and relative paths only); the songs that can't be rendered are listed with reasons
- [x] #2 The gated songs test requires orb render to equal the manifest for every listed song and fails on an unlisted song; regenerating it is documented
- [x] #3 Every mismatch is fixed or documented with its cause (docs/original_bugs.md or a dependency task); counts in docs/rbm.md 11; the corpus (175/175), cargo test and clippy pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Survey: the top-level files of ORB_THIRDPARTY_SONGS_DIR (the existing songs test's scope; the CaveysRebirthStuff subfolder of 1241 more .rbs is not in the 202 entries and gets a follow-up task).
2. New tool tools/rbhost/thirdparty_songs.py (args --songs --mods --rb20ful --remote-dir --owner -o): distinct songs by SHA-256, the mod the original resolves (GLOB INFO name over the mods folder in NTFS upcased order, first wins; Standard or missing = built-in kit), rbhost on the machine (one permit, refuses with Rebirth.exe running; RBS: --idle-blocks 9088 with --mod-msg; 1.x: rb1x messages, idle 0), the WAV hashed on the machine; writes tests/refs/thirdparty_songs.json (hashes and names only, plus the skipped list with reasons); deletes the remote folder. Unit tests in thirdparty_songs_test.py (resolution, order, manifest form) with a fake client.
3. crates/orb-cli/tests/thirdparty.rs songs test: render each manifest song with the manifest's settings (orb render --idle-blocks N --mod-dir --standard-kit) and require the WAV SHA-256 to equal rbhost's; fail on a file of the folder the manifest does not list; keep the load/round-trip/1.x message checks.
4. Triage every mismatch: download the rbhost WAV (run.py), orb compare to the first frame, fix or document (original_bugs.md) or a dependency task.
5. Docs: rbm.md 11 counts and the regeneration command; song_load.md 12.5 count update; AGENTS.md test line.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a2a15e16a64e12340 (rebased onto main ded0050).

Scope change (2026-10-29, the user through the supervisor): "I want all songs be handled recursively". task-129 covers every song file under ORB_THIRDPARTY_SONGS_DIR recursively (the CaveysRebirthStuff subfolder included: 1241 more .rbs files), deduplicated by SHA-256; the manifest keys songs by SHA-256 with their relative paths; the gated test (and the existing songs test) walk the folder recursively and fail on an unlisted file. The rbhost step is batched and resumable (songs whose render hash is recorded are skipped).
Survey of the top level before the change (tools/rb1x.read_any, rbsmsg.version_tuple): 201 files, 201 distinct: 124 ReBirth 1.x (2 in pattern mode: Spooks, Tech'r'Us2), 57 RBS 4.2, 20 RBS 4.1; mods 51 Standard, 14 PBE2, 5 MSM2, 2 Poison-Box, 1 each RedTop 2.0, Regulation Issue, Orbit 2.0, Nord Beat, Metallicon+.
rcs at 2026-10-29 (first try): gnome.lan:8765 connection refused (the rcs server is not running).

Recursive survey (scratch survey.py, 2026-10-29): 1241 song files under ~/rb-songs/songs (201 at the top level, 1040 in CaveysRebirthStuff), 1241 distinct by SHA-256. 429 ReBirth 1.x (4 in pattern mode), 808 RBS (431 4.2, 369 4.1, 8 4.0; 1 in pattern mode), 4 not readable by rbs.py nor crates/rbs: 'Battle of the Best/Top 25/005/Wax and Rain.RBS' (a BinHex 4.0 text, not a song), 'Battle of the Best/ChemicalPack/thefur~1.rbs' (no CAT header, not a song), 'ReBirth 2.x Songs/Pack34/yawn.rbs' (CAT size 18010, file 18020: 2 bytes after the CAT chunk; orb: truncated chunk header at 18018), 'Mod Demo Songs/AXIOM peace.rbs' (a 5-byte delta VLQ 8f ff fa 94 00 in the TRAK at 63036; orb and rbs.py refuse it).
Mods (tools/rbhost/thirdparty_songs.py resolution, by exact INFO name in NTFS order): 34 mod names over 33 files; 297 songs on Standard; 2 songs name 'omega', which no INFO name matches (Omega.rbm is 'Omega', strcmp is case-sensitive, docs/rbm.md 10): the original plays the built-in kit (its Standard choice), but orb --mod-dir falls back to the file name and loads Omega.rbm. The test renders such a song without the mod folder.
orb renders all 1237 readable songs with the manifest settings without an error (scratch orb_manifest.py, 95 s at 10 threads).

The 4 unreadable files, against the exe (llvm-objdump of original/Rebirth.exe; IDA MCP not available in this session):
- [C] sub_4A09D1 (the delta reader of sub_436B6F 0x436CDA, sub_436F5A 0x4370C1, sub_437380 0x4374FD) has NO length limit: it reads while bit 7 is set, v = (v << 7) | (c & 0x7f) in 32 bits (shl eax,7 at 0x4A0A1D). Then abs + delta (0x436CE5) and imul 0xC0 / shr 3 in u32 (0x436CF3), the tick check sub_42FD60. So the original loads 'AXIOM peace.rbs' (5-byte delta 8f ff fa 94 00 = 0xFFFE8A00, whose event fails the tick check and is dropped). orb refused it: a mismatch. Fixed: crates/rbs decode_delta reads any length with the 32-bit shift, Track::parse adds in u32 (wrapping), encode_delta writes 5 bytes for >= 2^28 (the file writes back byte for byte); tools/rbs.py the same (rbsmsg then converts it). orb_engine::load::track_batches already took the deltas with wrapping arithmetic. orb renders it now.
- [C] sub_434D05 first calls sub_49F3B4 (0x434D56): the chunk at offset 0 (sub_49F5E2, sub_49F477) must be kind 1..3 (FORM, CAT, LIST; 0x49F41D..0x49F427) and its whole length (sub_49F19B = header 12 + size - 4) must equal the stream length (vtable +0x18, cmp at 0x49F440); otherwise the load throws (0x434D92). So the original refuses yawn.rbs (CAT 18018 bytes, file 18020), Wax and Rain.RBS (BinHex text) and thefur~1.rbs (no chunk): orb refuses the same three. tools/rbhost/thirdparty_songs.py applies the same check (refusal()) and lists them as skipped with that reason; over the whole collection the check refuses exactly these 3.
- Validated the gated test against a manifest of orb's own hashes (scratch orb_manifest.py): songs: 1241 files, 1241 distinct, RBS1X 429, RBS42 808, SKIPPED 4, 1237 equal, ok in 102 s.

2026-10-29 afternoon: the rcs server on gnome.lan:8765 refuses connections (the host answers ping); the rbhost run waits for it. Meanwhile: corpus 175/175 after the rbs change (orb compare --manifest tests/refs/manifest.json --mod-dir <orb mod recrypt --iso>); cargo test --release without the variables: 616 passed, 0 failed; with them: 615 passed, 1 failed: thirdparty mods, because ~/rb-mods/mods gained SaturdayMorning.rbm at 15:30 today (173 files; not this task's doing), which tests/refs/thirdparty_mods.json does not list. The zip CaveysRebirthStuff/Battle of the Best/Top 25/006/SaturdayMorning.zip holds 'Saturday morning.RBS' (SHA-256 3212f02f..., in no other song file), a ReadMe and BD.AIF; it is not a song file (no .rbs in its name), so the test does not read it.

The rbhost run (2026-10-29 15:4x..16:25 local): tools/rbhost/thirdparty_songs.py --songs ~/rb-songs/songs --mods ~/rb-mods/mods --rb20ful <scratch>/RB20FUL.DAT (bsdtar from the CD image; SHA-256 62071e99...) --owner worktree-agent-a2a15e16a64e12340 --batch 100 -o <scratch>/rbhost_manifest.json. 13 batches of up to 100 songs, each in C:\re\rb\t129 under one permit of redc-win-run, with no Rebirth.exe running (checked per batch); the folder was deleted after each batch. C:\re\rb now holds only what it held before; mods_orig and the installation were not touched. The manifest was saved after every song (resumable); a second run with nothing pending verified that every recorded SHA-256 equals the folder's current files (1242 = 1242).
Collection changes during the run (supervisor's notes): the user resaved 5 pattern-mode songs between 16:09 and 16:13 (Tech'r'Us2.RBS, Spooks.rbs, TRIA UNIVERSIUM.rbs, 'scratch  and punch.RBS', stro~1.rbs; 4 were 1.x), all now RBS 4.2 in song mode on the built-in kit. The batches re-list the folder and so rendered the new versions (as RBS, idle 9088); the old hashes left the manifest. At 16:06 the user replaced 'Top 25/006/SaturdayMorning.zip' with its song 'Saturday morning.RBS' (1.x, built-in kit). The zip no longer exists, so there is no zip entry to list as excluded; the song is in the manifest. (The tool has --exclude PATH REASON for such a file.) A ctime scan (find -cnewer) of the folder showed no other changed song file. ~/rb-mods/mods briefly held SaturdayMorning.rbm (15:30); it was gone again before the final test (172 mods).
Result: 1242 distinct songs: 1239 rendered, 3 skipped (refused by the original, sub_49F3B4). Compared with orb render at the same settings (scratch orb_manifest.py + cmp.py): 1239 of 1239 equal. AXIOM peace (the VLQ fix) and the 5 resaved songs are included. Mismatches: none left; the only one, AXIOM peace refused by orb, is fixed. No pattern-mode song is left in the collection; the 5 that were rendered as pattern-mode songs earlier in the run were equal too, before their resave.
orb's file-name mod fallback: the 2 'omega' songs: decision task-145 (B recommended). The test does not depend on it (it renders those songs without the folder). orb info prints 'Last event: bar 5592406' for AXIOM peace: the raw position of the dropped event (display only).
Tests: cargo test --release -q --no-fail-fast without the 3 variables 616 passed / 0 failed / 3 ignored; with them (ORB_CD_ISO, ORB_THIRDPARTY_SONGS_DIR, ORB_THIRDPARTY_MODS_DIR) 616 passed / 0 failed (the songs test against the rbhost manifest included); clippy -D warnings clean; nightly fmt clean; Python unittest of tools and tools/rbhost OK; corpus orb compare --manifest tests/refs/manifest.json --mod-dir <recrypt --iso>: 175 of 175 equal.

Correction to the line above: 3 of the 5 pattern-mode songs (TRIA UNIVERSIUM, 'scratch  and punch', stro~1) were rendered by rbhost in pattern mode before the user resaved them, and they equaled orb (in the 465-equal partial comparison). Spooks and Tech'r'Us2 come later in the order and were rendered only in their new form.

Review round 1 (of 0ed65e9, send back), fixed in the amended commit:
1. Wrapped positions did not round-trip: deltas 0xFFFFFFF0 then 0x20 read as positions 0xFFFFFFF0, 0x10, and build() refused them as not sorted. Now crates/rbs Track::build and tools/rbs.py Track.build write delta = (pos - prev) mod 2^32 (wrapping_sub / & 0xFFFFFFFF). The rule: a position below the previous one is a wrap only when that delta is below 2^31; otherwise it is a backward step and is still refused. Tests: crates/rbs a_track_that_wraps_past_2_32_writes_back_its_deltas (the wrapped track writes back byte for byte; 5 then 4 refused; 0xFFFFFFFF then 0 accepted; 0x80000000 then 0 refused) and tools/rbs_test.py test_a_wrapped_track_writes_back_its_deltas. No song of the collection wraps. task-143 (open) makes Save write the engine's tracks, whose positions are below 32000 and sorted, so they never wrap; today Save writes the song's own events through this writer. Documented in docs/rbm.md 11.5.
2. The songs test without ORB_THIRDPARTY_MODS_DIR now fails. Each listed song with a third-party mod is FAIL-mods, and one problem line names the variable: 'set ORB_THIRDPARTY_MODS_DIR to the folder of the recrypted third-party mods'. Checked: ORB_THIRDPARTY_SONGS_DIR alone gives exit 101 with '510 songs of tests/refs/thirdparty_songs.json play a third-party mod and were not compared'. The test header and docs/rbm.md 11.5 say so.
Checks: nightly fmt and clippy clean; cargo test --release -p rbs 13 + 4 pass; Python tools 68 tests OK (3 skipped), tools/rbhost OK; corpus 175 of 175 equal.

Round 1 full run: cargo test --release -q --no-fail-fast with ORB_CD_ISO, ORB_THIRDPARTY_SONGS_DIR and ORB_THIRDPARTY_MODS_DIR: 617 passed, 0 failed, 3 ignored.

Round 1, without the 3 variables: 617 passed, 0 failed, 3 ignored.

Review round 2 (of 06cfc30, send back), fixed in the amended commit:
1. The 2^31 wrap rule was unsound (0x10, then delta 0xFFFFFFF0, gives 0x10, 0x0, which build() refused). Per the reviewer's decision: crates/rbs Track::build and tools/rbs.py Track.build now always write pos.wrapping_sub(prev) (& 0xFFFFFFFF) and refuse nothing, so every parsed track round-trips byte for byte. The order is checked where tracks are built from a model: orb-ui document::set_engine_tracks (the engine's lists; a 1.x import goes through it, since source_of_1x leaves the tracks empty) debug-asserts 'track N: events not sorted by position'. New unit tests in document.rs: engine_tracks_become_positions, and unsorted_engine_tracks_are_a_model_bug (should_panic, debug only). set_tracks (a file's accepted events) is not asserted, because a file's own wraps can legitimately go back; the writer stores them as the same wrap. tools/rbs.py validate (build_song from a spec) refuses an unsorted spec; covered in test_validation. The tests: rbs a_track_that_wraps_past_2_32_writes_back_its_deltas (0x10,0 / 0x80000000,0 / 5,4 write and read back; the file 0x10 then 8f ff ff ff 70 round-trips); wrong_model_shapes_are_errors_not_panics now expects an unsorted list to write and read back (it relied on the refusal); tools/rbs_test.py test_a_wrapped_track_writes_back_its_deltas (test_unsorted_rejected removed). Docs: rbm.md 11.5 'Positions that wrap'; song_load.md 12.6 notes that the writer no longer refuses.
2. Removed the stray space in thirdparty.rs's header (tools/rbhost/thirdparty_songs.py).
Checks: nightly fmt and clippy clean; cargo test --release -p rbs 13 + 4 pass; Python tools 67 OK (3 skipped), tools/rbhost 55 OK; debug cargo test -p orb-ui 220 passed, 0 failed. cargo test --release --no-fail-fast without the variables: 618 passed, 0 failed. With them: 617 passed, 1 failed. The failure is thirdparty mods: 'DJ Turn's 8-Track.rbm: not in ~/rb-mods/mods'. The user removed that file from the collection (171 mods now). That is a collection change outside this task; no song of the manifest uses that mod, and the songs test passed. tests/refs/thirdparty_mods.json needs regenerating, or the file restored (task-146 is about duplicate INFO names).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Every distinct song under ~/rb-songs/songs (recursively, as the user asked on 2026-10-29) is pinned: tests/refs/thirdparty_songs.json lists 1242 distinct songs. 1239 have rbhost's render SHA-256 (426 ReBirth 1.x at idle 0, 813 RBS 4.0/4.1/4.2 at idle 9088, 510 of them over 33 third-party mods resolved by exact INFO name in NTFS order). 3 are skipped because the original refuses them (sub_49F3B4: the file must be exactly one FORM/CAT/LIST; orb refuses them too). orb render equals rbhost for all 1239.
One mismatch was found and fixed. AXIOM peace.rbs has a 5-byte TRAK delta: the exe's sub_4A09D1 reads deltas of any length in 32 bits. crates/rbs and tools/rbs.py now do the same. They write every delta as (pos - prev) mod 2^32 and refuse nothing, so every parsed track round-trips. The order of a model's tracks is debug-asserted in orb-ui's document::set_engine_tracks (review rounds 1 and 2). The corpus stays 175/175.
New: tools/rbhost/thirdparty_songs.py (batched, resumable, --only, --fresh, --exclude; unit tests). The gated songs test in crates/orb-cli/tests/thirdparty.rs walks the folder recursively, renders with the manifest's settings and requires equal hashes. It fails on an unlisted file, and without ORB_THIRDPARTY_MODS_DIR when the manifest lists mod songs, so a green run compared all 1239. Docs: rbm.md 11.5, song_load.md 2.1, 5, 10, 12.5 and 12.6, AGENTS.md, tools/rbhost/README.md.
Decision task-145 (orb's file-name mod fallback, 2 'omega' songs) is open; the test does not depend on it.
Tests: cargo test --release 618 pass without the 3 variables. With them, 617 pass and only thirdparty mods fails, because the user removed 'DJ Turn's 8-Track.rbm' from ~/rb-mods/mods after this task's run; tests/refs/thirdparty_mods.json needs regenerating. Debug orb-ui 220 pass; clippy and fmt clean; Python tests OK. Branch worktree-agent-a2a15e16a64e12340, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-123
title: rb-ui File > Info shows a 1.x song's real facts
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 14:36'
updated_date: '2026-10-28 17:30'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/document.rs
  - crates/rb-cli/src/info.rs
priority: low
type: bug
ordinal: 143000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
rb-ui's document for a ReBirth 1.x song uses rbs::Song::empty() as its source (crates/rb-ui/src/document.rs source_of_1x), so File > Info shows the empty song's format version ("4.2"), copyright, tempo, loop, title and shuffle instead of the file's (rb-ui/src/info.rs SongInfo::of reads that stand-in). rb info (task-122) reads them with rb_cli::info::SongFacts::of_1x(&Song1x, &EngineSong). Make the UI take all of a 1.x song's facts from SongFacts, as rb info does (keep the facts on the document, or fill the source's head, glob and usri from them), with a test on a 1.x song built as crates/rb-cli/tests/info.rs does.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A test covers it; cargo test, clippy and nightly fmt --check pass
- [x] #2 File > Info of a 1.x song shows the format version, copyright, tempo, loop, mode, shuffle, synth sound, title, text and web page rb info prints for it
- [x] #3 A saved 1.x song keeps its title, text and web page in the USRI chunk with the original's copy rules (sub_430BA0 0x431039..0x431078, sub_465D8D), tested by reloading the file
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. session::Opened gets facts_1x: Option<SongFacts> = SongFacts::of_1x(&parsed,&engine) for a 1.x song. 2. Document gets facts_1x; App::settle takes it (adopt_as passes opened.facts_1x). 3. SongInfo::of uses doc.facts_1x else of_source; SongInfo gets rb1x and a format text (as rb info: 'ReBirth 1.x MIDI file (file version V)'); the window shows it under Format. 4. Mod change (menu.rs ~667) also updates the 1.x facts' mod_ftp/www. 5. After a successful save of a 1.x song the document adopts the composed 4.2 song as its source and drops the 1.x facts (the file on disk is now RBS 4.2). 6. Test in crates/rb-ui/tests/menu.rs on a 1.x song as crates/rb-cli/tests/info.rs builds it; show it fails on old code. 7. Update the example in rb-cli/tests/thirdparty.rs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a5affa8217b643306 (rebased onto main 37babfb).

Evidence: the new test rb-ui/tests/menu.rs file_info_of_a_1x_song_shows_its_own_facts fails on the old code (SongInfo::of reading the stand-in: left "RBS 4.2", right "ReBirth 1.x MIDI file (file version 3.1)") and passes now. Design: session::Opened and Document carry facts_1x (SongFacts::of_1x, taken at open); SongInfo::of uses it; SongInfo has rb1x and format() as rb info prints it; the window row is now Format. A mod change also updates the 1.x facts' mod ftp/web; a successful save of a 1.x song makes the composed 4.2 song the source and drops the 1.x facts (the file is RBS 4.2 now, so the window says RBS 4.2; the saved file's title/text are the stand-in's empty ones, as before). Side fix: the example in crates/rb-cli/tests/thirdparty.rs:12 now names RB_THIRDPARTY_MODS_DIR=~/rb-mods/mods and RB_THIRDPARTY_SONGS_DIR=~/rb-songs/songs. Checks: cargo +nightly fmt --check, clippy -D warnings, cargo test -p rb-ui (debug), cargo test --release -q --no-fail-fast with RB_CD_ISO and both THIRDPARTY dirs: no failures.

Review round 1 (send back; scope widened): (1) Data loss found: Save of a 1.x song wrote an empty title, text and web page, because the stand-in's USRI was empty. IDA (Rebirth.exe, read-only copy): sub_430BA0 strcpys the record into the document at 0x431039 (title, +0), 0x43104F (text, +0x29), 0x431067 (web page, +0x1BA) and 0x431078 (show-on-open, +0x21F); sub_42FFEC copies the block's strings with sub_465D8D(src, max, dst), max 0x28, 0xC8, 0x64 at block offsets +0, +0x29, +0xF2, and normalises the byte at +0x157 to 0/1 (0x430043..0x43004D). sub_465D8D: copies up to the first zero or max bytes, writes a 0x0a after each 0x0d (counted, skipped at the limit; 0x465DDA..0x465DED). Fix: rb_cli::info::Usri1x::of (usri_copy) does that; document::source_of_1x fills the stand-in's USRI from it; SongFacts::of_1x uses it too (so rb info follows the same caps). My earlier note 'as before' was wrong: it described a defect, now fixed. (2) Tests: file_info_of_a_1x_song_shows_its_own_facts reloads the saved file (title, text 'first\r\nsecond e-acute', web page, copyright, same facts in File > Info before and after); song_information_of_a_1x_song_is_copied_like_the_original (caps, CR LF, limit); a_mod_change_of_a_1x_song_updates_the_infos_mod_addresses (PBE2). (3) docs/song_load.md 12.6 records it. (4) SongFacts::format() is the one format string, used by rb info and rb-ui. Checks: fmt, clippy, debug rb-ui + rb-cli tests, release workspace with RB_CD_ISO and both THIRDPARTY dirs.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
File > Info of a 1.x song takes all its facts from rb_cli::info::SongFacts::of_1x, kept on the Document (facts_1x) from the open; the format reads 'ReBirth 1.x MIDI file (file version V)' from the one SongFacts::format used by rb info and rb-ui. A 1.x song's title, text and web page are copied into the stand-in source's USRI with the original's rules (Usri1x, sub_430BA0 0x431039..0x431078, sub_42FFEC, sub_465D8D: caps 40/200/100, a 0x0a after each 0x0d, show byte 0/1), so Save writes them (before, it wrote them empty: a defect found in review) and the saved file's File > Info equals the 1.x song's. A mod change updates the 1.x facts' mod addresses; after a save the document is an RBS 4.2 song. docs/song_load.md 12.6 records it. Tests in crates/rb-ui/tests/menu.rs: file_info_of_a_1x_song_shows_its_own_facts (also reloads the saved file), song_information_of_a_1x_song_is_copied_like_the_original, a_mod_change_of_a_1x_song_updates_the_infos_mod_addresses. fmt, clippy, debug rb-ui and rb-cli tests, and the release workspace tests with RB_CD_ISO and both collections pass. Also updated the thirdparty.rs example paths. Branch worktree-agent-a5affa8217b643306, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

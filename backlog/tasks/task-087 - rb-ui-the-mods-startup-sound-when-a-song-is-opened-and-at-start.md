---
id: TASK-087
title: 'rb-ui: the mod''s startup sound when a song is opened and at start'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 19:58'
updated_date: '2026-10-27 21:12'
labels:
  - ui
  - re
milestone: m-3
dependencies:
  - TASK-074
references:
  - docs/export_state.md
  - docs/ui_spec.md
  - crates/rb-ui/src/menu.rs
priority: low
type: feature
ordinal: 107000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the original plays a mod's startup sound not only after a Mods-menu change (task-074) but also from the song load: sub_426C67 (msg 58 when the mod record's word +1088, its user count, is 1 and the preference 'Play Mod Startup Sound' is on) is called from sub_425C4D at 0x425FAB and 0x426400 and from sub_426CA8 at 0x426D48 (docs/export_state.md section 8). So opening a song whose mod is not already open by another song, and probably the app's start with its Untitled song on 'Standard ReBirth', play the startup sound. rb-ui does not do this yet (task-074 covered the mod change and the logo).

Do: read sub_425C4D, sub_426CA8 and their callers in the disassembly (which paths: open, new song, the start), note which of them call sub_426C67 and when the user count is 1 (one song in rb-ui: the new song's mod is opened before the old one is released? check the order against sub_416300 / sub_4165BE, docs/rbm.md section 14.1). Then call Link::startup_sound after Link::load in App::adopt (and at start, if the original does) when Settings::startup_sound is on, with a test in crates/rb-ui/tests/menu.rs like a_mod_change_plays_the_startup_sound_when_the_preference_is_on. Document the paths in docs/ui_spec.md section 18.

Acceptance: the paths are documented with addresses; rb-ui plays the startup sound where the original does; a test covers it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 the load paths that send msg 58 are documented with addresses in docs/ui_spec.md
- [x] #2 rb-ui plays the startup sound on those paths when the preference is on, with a test
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Read sub_426C67, sub_425C4D, sub_426CA8 and their callers (IDA headless copy, checked with llvm-objdump on original/Rebirth.exe): which paths reach msg 58, the user count test, the start.
2. rb-ui: App::adopt plays the startup sound after Link::load when Settings::startup_sound is on; a new App::start(song) used by main opens the command-line song (adopt) or, with none, plays the startup sound of the new empty song (the original's Untitled song at start).
3. Tests in crates/rb-ui/tests/menu.rs: an open plays the sound when on and not when off; the start with and without a song; fix existing tests whose comparisons assumed no sound on open.
4. Document the paths with addresses in docs/ui_spec.md section 18, docs/export_state.md section 8 and docs/rbm.md 14.3.
5. Full tests, clippy, fmt; rebase; one commit.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-af010954c2f70124c

Disassembly (IDA headless copy of ida/Rebirth.exe.i64, read-only, closed save=false; every call site re-read with llvm-objdump -d on original/Rebirth.exe):
- sub_426C67(open mod, mod object): movsx ecx, word [eax+0x440] at 0x426C6E, jne at 0x426C78 (word +1088 must be 1), the preference sub_4098C3 at 0x426C85, msg 58 through sub_40ABB5 at 0x426C9F. The count is read after the caller's sub_416300 has incremented it (0x416452), so 1 = no other open song uses the mod.
- Callers: sub_41A0C5 0x41A180 (Mod Selector), sub_426FB3 0x427030 (mod change), sub_425C4D 0x425FAB and 0x426400, sub_426CA8 0x426D48. Nothing else.
- sub_425C4D = the song document built from a file (from sub_40B575 at 0x40B6B7 only). Branch sub_43037D true (the old 1.x format, loaded by sub_430BA0): mod = dword_4DA784 (Standard ReBirth) opened at 0x425E24, sound at 0x425FAB. Else (RBS, sub_434D05 at 0x426423): mod looked up by name sub_4155D1 (0x426195); not found -> the missing-mod dialog sub_425B0B (rescan/retry 67, found 66, Use Standard 68 -> dword_4DA784 and the song marked modified 0x4263EA, cancel 69 throws XSongLoadCancelled); opened 0x42627B; sound at 0x426400. Both sends come before the song's load (sub_430BA0 / sub_434D05).
- sub_40B575 (open a file): if a window already shows the file it is activated (sub_496354 at 0x40B66C) and no document is built: no sound. Callers: File Open sub_40A2A3 (0x40A310; command 101 at 0x40C282), the recent-files items (0x40C3F4), app vtable 0x4BA65C slot 13 sub_40A49E (0x40A4C9, the framework's open of a document file).
- sub_426CA8 = the new Untitled song (from sub_40A581 at 0x40A653 only), mod dword_4DA788 opened at 0x426D2F, sound at 0x426D48. sub_40A581 is called by File New (command 100, 0x40C275) and by sub_40A50D (0x40A56B), app vtable slot 14 (0x4BA694). dword_4DA784 and dword_4DA788 are both set to the Standard ReBirth mod object by sub_40DDFB (0x40DEBA, 0x40DEC3); 4DA788 is changed only by the Mods menu (0x40C489) and the Mod Selector's OK (0x41A2FD).
- The start: the frame's run loop sub_480AA0 calls sub_481050(cmdline) at 0x480E40 and, when it opened no file (je at 0x480E4C), the app's slot 14 (call [edx+38h] at 0x480E5E) = sub_40A50D -> sub_40A581 -> sub_426CA8: the Untitled song on Standard ReBirth plays Standard's startup sound when the preference is on. sub_481050 opens the command-line files through sub_49381B [G: that it reaches slot 13 / sub_40B575 was not traced past sub_49381B].
- The Benchmark song sub_4269CE opens dword_4DA788 (0x426B1D) and sends no 58.
rb mapping: rb-ui has one song, and its Open asks the unsaved prompt as the original's close does, i.e. the open song is closed and the new one opened: the new song is its mod's only user, the count is 1, so every successful open plays the sound (pref on). The original with another window open on the same mod would not play it; rb has no second window. Reopening the open file reloads in rb (the original only activates the window).

Implementation: crates/rb-ui/src/menu.rs: App::adopt calls the new App::mod_opened (sub_426C67: Link::startup_sound when Settings::startup_sound) after Link::load; App::use_kit uses the same helper; new App::start(song) opens the command-line song (adopt) or plays the sound of the new empty song; crates/rb-ui/src/main.rs calls App::start. Tests (crates/rb-ui/tests/menu.rs): a_song_open_plays_its_mods_startup_sound_when_the_preference_is_on (drums_kit on Standard ReBirth: on == mix_preview(off, the built-in kit's slot 46), on != off; a copy saved with PBE2 opened from the Mods folder plays PBE2's sound, which differs from the built-in one), the_start_plays_the_startup_sound_when_the_preference_is_on (start without and with a song, the sound once). Failed first run: a_mod_changes_the_kit_and_is_saved compared a song opened with PBE2 (now with its startup sound) against one changed to PBE2 with the sound off; the opened harness now has the preference off too.
Docs: docs/ui_spec.md new section 18.1 (the five callers of sub_426C67, the open, new-song and start paths, rb's mapping and where it differs); docs/export_state.md section 8 and docs/rbm.md 14.3 point to it.
Checks: cargo fmt clean; cargo clippy --all-targets -- -D warnings clean; RB_STANDARD_MOD=target/Standard.rbm RB_MOD_DIR=<recrypted mods> cargo test --release -q: 446 passed, 0 failed (51 test binaries). No engine change, so no corpus run.

Rebased onto main 60cf139, then: cargo test --release 448 passed, 0 failed; clippy clean; the diff against main lists only this task's files.

Review round 1 (send back): the final summary still said 446 tests; corrected to 448 after the rebase (446 before it). No code change.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The original sends the startup sound (msg 58 via sub_426C67: mod word +1088 == 1 right after sub_416300 opened it, and the preference on) from five places: the Mod Selector (0x41A180), the mod change (0x427030), a song opened from a file (sub_425C4D, 0x425FAB for 1.x and 0x426400 for RBS, before the song's load), and the new Untitled song (sub_426CA8, 0x426D48), which is also the app's start when the command line opens no file (sub_480AA0 0x480E5E -> slot 14 sub_40A50D -> sub_40A581). An already open file is only activated (no sound). Documented in docs/ui_spec.md 18.1 with addresses (checked with llvm-objdump), pointers in docs/export_state.md 8 and docs/rbm.md 14.3. rb-ui now plays the open song's mod's startup sound after every successful open (App::adopt) and at the start (new App::start: the command-line song, or the empty song's built-in kit sound), when Settings::startup_sound is on; rb's one-song Open closes the old song first, so the count is always 1. Differences kept and documented: no second window (the original's count 2 case), reopening the open file reloads and plays. Tests: two new rb-ui tests in crates/rb-ui/tests/menu.rs, one existing test adjusted; full cargo test --release 448 pass after the rebase onto main 60cf139 (446 before it), clippy and fmt clean. Branch worktree-agent-af010954c2f70124c, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

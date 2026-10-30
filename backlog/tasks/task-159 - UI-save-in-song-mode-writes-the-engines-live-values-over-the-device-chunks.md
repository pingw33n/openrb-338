---
id: TASK-159
title: UI save in song mode writes the engine's live values over the device chunks
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-30 10:30'
updated_date: '2026-10-30 12:51'
labels:
  - ui
  - re
milestone: m-5
dependencies:
  - TASK-156
references:
  - crates/orb-ui/src/document.rs
  - crates/orb-ui/src/link.rs
  - docs/song_load.md
priority: medium
type: bug
ordinal: 176000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: found in task-156 on 2026-10-30. In song mode the panel follows the engine's live device bytes: Link::follow in link.rs calls panel.adopt for every controller from the snapshot. document::compose then writes each panel value into the device chunk whenever it differs from what the load made of the chunk. So a song-mode song saved without an edit, after at least one UI frame, gets its device chunks (the pattern-mode parameters) overwritten by the engine's live values, which come from the tracks' tick-0 events.

Reproduction:
- Take tests/songs/diag/songs/303_accent0.rbs (song mode) and set tb303[0].params[3] (cutoff) = 10 and params[8] (waveform) = 1, while its track 1 keeps the tick-0 events cutoff 50 and waveform 0.
- Open it in App (a Link with the built-in kit), render 4 calls, draw 4 frames, then call App::song_bytes.
- The saved chunk has cutoff 50 and waveform 0.

The same happens to out-of-range switch bytes whose tick-0 events the load drops by its range check: waveform 2 becomes 0, 808 selector 255 becomes 0, PCF mode 2 becomes 1, delay mode 127 becomes 1. The corpus round-trip test (menu.rs saving_an_unedited_corpus_song_gives_back_the_file) draws no frame, so it misses this.

The original saves what the DLL's device blocks hold (sub_43920E and its siblings, docs/song_load.md 4.10): the pattern-mode parameters, not the live values. That is [G]: check which block the DLL returns in song mode.

What:
1. Find what the original's save writes for a song-mode song: the DLL's data block in song mode, either the pattern-mode parameters or the live bytes.
2. Make compose take the device chunk from the right source in song mode, so that an unedited save writes the file that was read. One way is to keep the panel's pattern-mode values apart from the live values it shows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The original's device block in song mode is identified with addresses and recorded in docs/song_load.md
- [x] #2 A song-mode song opened, drawn for several frames and saved without an edit writes the file read byte for byte: a test, including the task-156 song with out-of-range switches in crates/orb-ui/tests/switch_values.rs, whose song-mode case then saves after frames
- [x] #3 fmt, clippy and cargo test pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE (DLL and exe copies, read-only): the save path's queries (sub_438BCE, sub_438EAA..sub_43A1F4) and the DLL's whole-block query of each device in song mode; slot 9.
2. docs/song_load.md 4.10: what the save writes in song mode, with addresses; fix the [G]/wrong claims there (the mixer's save query is 138, slot 9 restores on leaving song mode).
3. Engine accessor (no numeric change): SongDevice::pattern_params and saved_block(song_mode), MixerDevice::saved(song_mode), modelled on the query handlers.
4. orb-player Snapshot: the save blocks (saved, saved_mixer) published every call.
5. orb-ui Link keeps the save blocks of the last followed song-mode snapshot (None after a load until then, and in pattern mode); document::compose takes the controllers, the pattern selection and the mixer from them in song mode, the panel otherwise; the was-check keeps the file bytes.
6. Tests: switch_values.rs song-mode case saves after frames; a new test for a song-mode song whose device chunks differ from its tick-0 events, drawn and played, saved byte for byte; the corpus round trip after frames; unit tests for saved_block / MixerDevice::saved.
7. fmt, clippy, cargo test (debug orb-ui, release full with ORB_CD_ISO and the third-party dirs).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a53d64333b57793bf (rebased onto main 34fc8ed).

RE (copies of ReBirth.dll.i64 and Rebirth.exe.i64, headless, closed save=false, deleted):
- The save's queries (sub_40B23A(song, dev, sub), the push bytes before each call): mixer sub_438BCE 0x438C1A push 8Ah = 138 (not 137, the mixer view's read); delay sub_438EAA 0x438EC0 6A 53 = 83; PCF sub_438F8C 0x438FA2 6A 71 = 113; dist sub_439092 0x4390A8 6A 5B = 91; comp sub_439150 0x439166 6A 65 = 101; 303 sub_43920E 0x43929D 6A 1F = 31; 808 sub_439996 0x439A28 6A 30 = 48; 909 sub_43A1F4 0x43A286 6A 41 = 65.
- DLL replies (disassembly): 303 31 at 0x10025751: sub_1001FCB0 (song mode) zero -> copies the live 12 bytes +0xC7C..+0xC87 into the pattern copy +0x30 (0x10025764..0x1002577C); then packs +0x30 (sub_10018E30, 0x1002578D) as msg 39. Song mode: the pattern copy as it is. 808 48 at 0x10028321 (live +0x1890..+0x18AF into +0x30 only when not song mode), 909 65 at 0x1002B951 (+0x1891.. likewise), delay 83 at 0x1000FC66 (+0x34 -> +0x3A), dist 91 at 0x10010E7C (+0x39 -> +0x3D), PCF 113 at 0x10016D80 (+0x38 -> +0x40), comp 101 at 0x1000F0FD (+0x38 -> +0x3D): the same.
- Mixer: table jpt_10015587 0x10151ECC indexed by the raw byte: 137 (0x101520F0) -> 0x10015589 replies the LIVE set (+0x30, sub_10019D70) as msg 143; 138 (0x101520F4) -> 0x100155DA: pattern mode copies the whole live set +0x30..+0x57 into the copy +0x58..+0x7F (0x100155F4..0x1001562D); song mode copies only the master (+0x30 -> +0x58, 0x10015635/0x10015644) and the four enabled bytes (+0x40+6c -> +0x68+6c, loop 0x10015647..0x10015662); then packs +0x58 (sub_10019D70 at 0x10015671). So in song mode the save's mixer is the pattern copy with the live master and enables (no controller reaches those).
- Slot 9 (sub_10013FB0 at 0x10013FF9 calls vtable +0x24 of each device when the mode byte +0x158 changes): 303 0x10026B60 (entering song mode: live -> copy; leaving: copy -> live), mixer 0x10016280, delay 0x100100B0: as the engine models them (orb-engine set_song_mode).
- Answer: in song mode the save writes the pattern-mode copies, not the live values the tracks set. After a song-mode load the copies are the msg-41 blocks (msg 36 enters song mode before msg 41), so an unedited save writes the file's device data. A song-mode panel edit without record changes only the live byte, which the save does not write. Not a defect, no data loss: no Decide needed.
- Failed hypothesis: docs/song_load.md 4.10 said the mixer's save asks 137 (live): wrong, it asks 138. Had it been 137, 614 of the 708 third-party song-mode songs and 16 of the 161 corpus song-mode songs would change on save (mixer chunk != the tick-0 events; a scratch script over tools/rbs.py).

Implementation (copies the original; no Decide needed: an unedited save is the file read):
- orb-engine (no numeric change, accessors only): SongDevice::pattern_params (303/808/909 params, delay/dist/PCF/comp pattern) and the provided saved_block(song_mode) (live in pattern mode, the copy in song mode); MixerDevice::saved(song_mode) (query 138: the copy with the live master and enables in song mode). Unit test frame::tests::the_saved_blocks_are_the_live_ones_in_pattern_mode_and_the_copies_in_song_mode.
- orb-player: Snapshot::saved ([LiveBlock; 9]) and saved_mixer, published every call.
- orb-ui: Link keeps SavedBlocks (Link::song_mode_blocks) from each followed snapshot that is in song mode; None in pattern mode and from a load (queue_song) until the panel follows the new song. The engine's mode decides, so a panel mode switch the engine has not run yet still saves right. document::compose takes the controllers, the pattern selections and the mixer from them in song mode; a device without a block (a drum machine without samples is a placeholder) falls back to the panel's value. The was-check against the load keeps the file's bytes.
- Failed first run: menu.rs corpus round trip with Link::new(tables(), None): 11 older (4.0/4.1) song-mode songs "load differently (the 808)": without samples the 808 is a placeholder, its saved block empty, so compose skipped the controllers and left the raw unconverted tom tunings (v/2+32 not applied). Fixed by the panel fallback; the test now also loads with the drum samples.
- Consequence, as in the original: a song-mode controller edit without record is heard and shown but not saved. Three menu.rs tests edited knobs in song-mode diag songs and expected the edits saved (an_edit_of_each_kind_is_saved, save_and_save_as_reopen_identically, the_unsaved_prompt_guards_opening_another_song): they now switch to pattern mode first or use a pattern-mode copy of the song.
Tests:
- Without the fix (saved forced None) after frames: song_mode_save.rs 3 of 3 fail; menu.rs corpus round trip after frames and play: 16 of 163 songs differ. With it: all pass (163 songs, 17 older than 4.2).
- New crates/orb-ui/tests/song_mode_save.rs: the task's repro (303_accent0, cutoff 10 / waveform 1 against the events 50 / 0, plus mixer PCF id, a level, a pan and the delay feedback against the tracks) saved byte for byte after 4 frames and after 60 playing; a pattern-mode edit carried into song mode is saved there, a song-mode edit is not, back in pattern mode it is live; the mixer master and a mute edited in song mode are saved.
- switch_values.rs: the song-mode case now saves before the first frame, after 4 frames and after 36 playing frames, in both modes.
- Last change: the mixer master and enables come from the panel in both modes (query 138 saves them live; the panel also has the edits the engine has not run yet).
Final checks: cargo +nightly fmt --all --check OK; cargo clippy --all-targets -- -D warnings OK; debug cargo test -p orb-ui 244 passed, 0 failed; cargo test --release -q --no-fail-fast with ORB_CD_ISO, ORB_THIRDPARTY_MODS_DIR, ORB_THIRDPARTY_SONGS_DIR: 665 passed, 0 failed, 3 ignored (68 result lines; corpus_golden included; the engine change is accessors only).

Review round 1 (send back, 3 fixes; the fix itself sound):
1. Commit subject area tools: -> engine: (the commit touches orb-engine, orb-player, orb-ui).
2. docs/ui_spec.md's "[C: the code]" for the unchecked store of an out-of-range switch byte now points to docs/song_load.md section 8, which got the addresses (DLL copy reopened headless, closed save=false, deleted): plain byte moves into the pattern-mode copy, no compare: 303 sub_10018CC4 (12 bytes, called at 0x10018F05 from sub_10018EF0, msg 39), 808 sub_100190F4 (0x100194A3 from sub_10019490, msg 56), 909 sub_100196F4 (33 bytes, only 20..22 permuted at 0x1001976E..0x1001977D, msg 73), delay sub_10019B00 (6), dist sub_10019B70 (4), comp sub_10019BE0 (5), PCF sub_10019C80 (8).
3. The counts "16 of the 161 corpus song-mode songs" and "614 of the 708 third-party songs" (mixer chunk != the mixer track's position-0 events), reproduce with this script (python3 -I mixdiff.py TOOLS_DIR SONG_DIR...):

import sys, glob, os
sys.path.insert(0, sys.argv[1])
import rbs
tot = sm = diff = 0
for root in sys.argv[2:]:
    for p in sorted(glob.glob(os.path.join(root, "**", "*.rbs"), recursive=True)):
        try:
            s = rbs.read_song(p)
        except Exception:
            continue
        tot += 1
        if not s.glob.song_mode:
            continue
        sm += 1
        m = s.mixer
        chunk = {1: m.comp_id, 2: m.pcf_id}
        for c, ch in enumerate(m.channels):
            for k, v in enumerate([ch.level, ch.pan, ch.delay_send, ch.dist]):
                chunk[6 + 6 * c + k] = v
        t0 = {}
        for e in s.tracks["mixer"].events:
            if e.pos == 0:
                t0[e.ctrl] = e.value
        d = [(k, chunk[k], t0[k]) for k in chunk if k in t0 and t0[k] != chunk[k]]
        if d:
            diff += 1
            print(os.path.relpath(p, root), d[:6])
print("songs", tot, "song mode", sm, "mixer chunk != tick-0 events", diff)

Runs: `/usr/bin/python3 -I mixdiff.py tools tests/songs` -> "songs 163 song mode 161 mixer chunk != tick-0 events 16" (all shipped); `/usr/bin/python3 -I mixdiff.py tools ~/rb-songs/songs` -> "songs 708 song mode 708 mixer chunk != tick-0 events 614".
After the fixes: fmt check OK; debug cargo test -p orb-ui --test song_mode_save 3 passed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Outcome: a song-mode song saved without an edit, after any number of frames and while playing, is the file read byte for byte.

RE [C] (docs/song_load.md 4.11, 4.10 corrected): the exe's save asks each device's whole block (mixer 138 at 0x438C1A, delay 83, PCF 113, dist 91, comp 101, 303 31, 808 48, 909 65). In song mode the DLL replies with the pattern-mode copy (303 0x10025751, 808 0x10028321, 909 0x1002B951, delay 0x1000FC66, dist 0x10010E7C, PCF 0x10016D80, comp 0x1000F0FD), the mixer (0x100155DA) with the copy plus its live master and channel enables; in pattern mode the live block, copied into the copy first. So the original saves the pattern-mode parameters, not the live bytes the tracks set; no file change, not a defect, no Decide needed. The doc had the mixer's save query as 137 (the live set): wrong.

Change: SongDevice::pattern_params / saved_block, MixerDevice::saved (orb-engine, accessors only); Snapshot::saved and saved_mixer (orb-player); Link::song_mode_blocks (orb-ui) from each followed song-mode snapshot; document::compose takes the controllers, pattern selections and mixer controllers from them in song mode. As in the original, a song-mode controller edit without record is not saved (three menu.rs tests now edit in pattern mode). ui_spec.md updated (the switch-save [G] is now [C: the code]).

Tests: new crates/orb-ui/tests/song_mode_save.rs (3), switch_values.rs saves after frames and while playing in both modes, menu.rs corpus round trip after 3 followed and 3 playing calls with the drum samples (16 of 163 songs differed without the fix), engine unit test. fmt, clippy OK; debug orb-ui 244 pass; release full 665 pass, 0 fail.

Branch worktree-agent-a53d64333b57793bf, one commit on main 34fc8ed.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-029
title: Golden test suite over the reference corpus
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-27 03:36'
labels:
  - tests
  - engine
milestone: m-1
dependencies:
  - TASK-028
  - TASK-014
  - TASK-025
  - TASK-026
  - TASK-027
priority: high
type: task
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The golden test suite: a cargo test (behind a feature or an ignored-by-default flag if the corpus is not in the repository) that renders every corpus song with the decided initial-state preset and compares it with its reference by the decided pass criterion, printing the first difference (frame, time, bar) on failure, plus a summary command (`rb compare` or tools/rbcmp.py over the whole corpus) for agents. The milestone is done when every song passes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 cargo test runs the whole corpus comparison and reports the first difference per failing song
- [x] #2 Every corpus song passes the pass criterion
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. rb_engine::compare (library, used by the test and the CLI): first_difference of two int16 interleaved streams (frame, channel, both values, or a length mismatch), the time (s) and bar of a frame (bar = loop_start/768 + seconds * BPM/60/4, from the song's tempo), a canonical-WAV/FLAC reference loader is left to the callers (claxon is a dev-dep of rb-engine, a dep of rb-cli).
2. crates/rb-engine/tests/corpus_golden.rs: one test per manifest set (diag at 9088, app at each entry's fitted N, shipped at 9088), all rendered with Devices::implemented(samples) in a fresh Engine; pass = sha256 of the whole WAV (header included) equals wav_sha256 (task-013, byte-exact). On failure decode the FLAC and print per song the first difference (frame, time, bar, channel, rb vs reference); shipped has no audio, so only the hash/frame mismatch. Shipped is cfg_attr(debug_assertions, ignore) (measure). Mod songs: out of scope (m-2), stated in the test.
3. Remove pure duplicates: the per-device *_match_their_references tests and corpus_pcf_comp's shipped test (same render, same reference, same criterion as the golden test). Keep the controls, the routing-list test, the nodelay fixture test, the 808 counter test.
4. rb compare: SONG.rbs --reference REF.(flac|wav), or --manifest PATH [--set diag|app|shipped] [NAMES]; --idle-blocks (manifest entries use their own N unless given), --rebirth-dll; prints EQUAL or the first difference per song, exit 1 on any difference. CLI integration tests. Document in AGENTS.md Build and test.
5. docs/original_bugs.md: Q-1 before Q-2; OB-11 decay cites 0x1001684C (mov cl,[esi+3Dh], the decay byte load) and 0x1001684F (sar ecx,3), checked from the bytes (8a4e3d / c1f903).
6. Measure times in debug and release; clippy, fmt, cargo test; rebase; one commit.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a37a912bb2ef02b1a

Work (2026-10-27):
- rb_engine::compare (crates/rb-engine/src/compare.rs): first_difference (frame, channel, both values, or a length mismatch), seconds, bar (loop_start/768 + seconds*BPM/60/4 from the song's global tempo; ReBirth has no tempo automation, shuffle ignored), describe. 5 unit tests.
- Golden test crates/rb-engine/tests/corpus_golden.rs: diag (145 at 9088), app (11, each at its fitted N: 9040..9104), shipped (8, sha256+frames only), one fresh Engine per song with Devices::implemented(Some(samples)), parallel over available_parallelism threads. Pass = sha256 of the whole WAV == wav_sha256 (task-013). On failure: per song the first difference from the decoded FLAC (claxon, new dev-dep of rb-engine). A 4th test checks that the shipped songs left out are exactly the 8 mod songs (m-2) and the 2 pattern-mode Default Songs.
- Timings (10 cores, macOS): debug diag 3.8 s, app 0.5 s, shipped 24 s (TGV's KiloMix '98, 40485650 frames, dominates one thread); release diag 0.26 s, app 0.03 s, shipped 1.5 s; whole golden binary 1.7 s in release. Shipped is #[cfg_attr(debug_assertions, ignore)]: it runs in every cargo test --release, and in debug with -- --ignored.
- Failure output checked by a temporary +16 on every N (reverted): e.g. "808_snare (N 9056): first difference at frame 3376 (0.076553 s, bar 0.038), channel 0 (left): rb 181, ref 182"; shipped: "G-Spot (N 9104): sha256 differs (564480 frames, reference 564480); no audio committed, so no first difference ...".
- Device tests: the *_match_their_references tests of corpus_tb303 (corpus refs), corpus_tr808, corpus_tr909, corpus_effects (dist), corpus_pcf_comp and pcf_comp's ignored shipped test were pure duplicates (same render with Devices::implemented, same manifest sha256, same criterion; Some(samples) vs None makes no difference for the 303 songs, which pass in the golden test with samples) and are removed. Kept: every control, the routing-list test, the tb303 no-delay fixture test (other references), the 808 snare counter test. The 808 and 909 controls now run over the whole list (45 and 33 songs: every one differs without its device, so the golden test hears the device in each), which keeps the lists in use; 4.4 s and 4.7 s in debug.
- rb compare (crates/rb-cli/src/compare.rs, main.rs refactored into load_song, warn_unsupported, Renderer shared with render): `rb compare SONG --reference REF.flac|.wav [--idle-blocks N]` or `rb compare --manifest PATH [--set diag,app,shipped] [NAME|SET/NAME...]` (each entry at its own N unless --idle-blocks), --rebirth-dll; prints "set/name: EQUAL" or "set/name: DIFFERENT (N n): first difference at frame ..." and "K of M equal"; exit 1 on a difference, 2 on usage errors. 4 CLI tests (crates/rb-cli/tests/compare.rs). Fault warnings now name the song.
- `./target/release/rb compare --manifest tests/refs/manifest.json --rebirth-dll original/ReBirth.dll`: 164 of 164 equal, exit 0. `... --set app 808_snare tempo_143_21 --idle-blocks 9088`: both DIFFERENT (808_snare at frame 3012, rb -63 ref -64), exit 1.
- docs/original_bugs.md (task-051 review nits): Q-1 now before Q-2; OB-11 cites 0x1001684C `mov cl,[esi+3Dh]` (8A 4E 3D) and 0x1001684F `sar ecx,3` (C1 F9 03), read from original/ReBirth.dll's bytes.
- Out of scope (m-2): the 8 mod-using shipped songs (Alloy Cowboy Resurrection, Clodhopper, Fonda Workout, Groove It Up a 2nd Time!, Metallicon City, Mutilated Species II, RSI, SuperNova), stated in the golden test's doc and checked by shipped_songs_out_of_the_corpus_are_mod_or_pattern_mode.

AC #1: cargo test runs corpus_golden (diag and app always, shipped in --release or with --ignored); a failing song prints its first difference with frame, time, bar, channel and both values (shown with the temporary N+16 run above).
AC #2: all 164 corpus entries (145 diag, 11 app at their fitted N, 8 shipped) pass byte-exact: corpus_golden 4/4 in release, rb compare 164 of 164 equal. The 8 mod songs are m-2.

Review round 1 (supervisor): everything verified; metadata fix only. The supervisor reassigned the task from @agent-worker to @agent-complex to match who did the work (an Opus agent).
AC #1 note: in a debug build the shipped test is skipped (cfg_attr(debug_assertions, ignore), 24 s in debug), which the supervisor accepted; cargo test --release runs the whole corpus comparison, as corpus_golden.rs and AGENTS.md say.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The golden test and rb compare (branch worktree-agent-a37a912bb2ef02b1a, one commit).
- crates/rb-engine/tests/corpus_golden.rs renders every corpus entry (145 diag at 9088, 11 app exports at their fitted N, 8 non-mod shipped songs) and compares the WAV's sha256 with the manifest's (byte-exact, task-013); on failure it prints per song the first difference: frame, time, bar (from the tempo), channel, rb and reference values (decoded FLAC). Shipped is ignored in debug builds only (24 s debug, 1.5 s release). Result: 164 of 164 equal.
- rb_engine::compare holds the difference logic shared with the CLI.
- rb compare: one song against a .flac/.wav, or a whole manifest (--set, names), --idle-blocks, --rebirth-dll; EQUAL or the first difference per song, exit 1 on any difference. Documented in AGENTS.md "Build and test".
- The per-device reference comparisons and pcf_comp's shipped test, pure duplicates of the golden test, are removed; the controls stay, the 808 and 909 ones now over their whole lists.
- docs/original_bugs.md: Q-1 before Q-2, OB-11 cites 0x1001684F (sar) and 0x1001684C.
- The 8 mod-using shipped songs are out of scope (m-2).
Tests: cargo test 205 pass, 1 ignored (debug); cargo test --release 206 pass; clippy -D warnings clean.
<!-- SECTION:FINAL_SUMMARY:END -->

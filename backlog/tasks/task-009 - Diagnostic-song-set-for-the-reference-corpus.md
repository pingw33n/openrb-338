---
id: TASK-009
title: Diagnostic song set for the reference corpus
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 11:30'
labels:
  - tests
milestone: m-0
dependencies:
  - TASK-005
  - TASK-006
priority: high
type: task
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build the diagnostic song set from docs/initial_investigation.md section 9, as JSON specs turned into .rbs by tools/rbs.py build (the specs and a script that builds all of them are committed; the songs per the storage decision): silence; single TB-303 notes (saw and square; low/high cutoff; resonance 0/64/127; accent; slide; both sound models); each TR-808 and TR-909 instrument alone (several knob values, accents, flam); each effect alone on one source (distortion incl. shape 0, PCF in both modes and several patterns, compressor on a channel and on master, delay straight and triplet with feedback 0/64/127); mixer level and pan sweeps through song automation; shuffle; loop start at bar 1 and bar 2; odd tempos (e.g. 97.3, 143.21 BPM) to exercise block quantization and the final partial chunk. Each song a few bars. A README in the song folder lists every song and what it isolates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The JSON specs and a build script are committed; running it produces every song
- [x] #2 Every song opens in the original without an error and plays what its README says (spot-checked by export for at least five songs)
- [x] #3 The README lists each song and the engine feature it isolates
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. tests/songs/diag/gen_specs.py describes every song (device values, patterns, extra TRAK events) and writes specs/*.json plus README.md. Every song: song mode, loop set, USRI show_on_open 0, TRAK event at pos 0 for every controller computed from the same values as the chunks (docs/song_load.md s6, docs/test_machine.md s8).
2. build.py builds songs/*.rbs with tools/rbs.py build. 3. rbs.py check on all; export 8 songs with tools/rb_export.py and look at the WAVs (length, peak, silence) with a throwaway script (WAVs not committed).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ae7c235c1fca82f63

Result: 145 songs (specs/*.json, songs/*.rbs, README.md with a row per song) in tests/songs/diag/. Groups: silence; 303 saw/square x low/high cutoff, resonance 0/64/127, envmod/decay, accent knob 0/127, slide, slide+accent, tune, 7 vintage-model variants, two voices; each of 11 808 and 11 909 instruments alone, knob lo/hi variants, accent knob 0/127, 909 flam interval 0/64/127, a dense kit; distortion (amount 0/20/100/127, shape 0/64/127, on 303 and 808, disabled device); PCF lp/bp x wave 0/64/127, freq/res, amount/decay, on 303/808/909; compressor on channel (303, 808) and master; delay straight/triplet x feedback 0/64/127, steps 1/8, pan 0/127, on 303, automated steps/feedback; mixer level/pan sweeps for 303/808/909, delay send sweep, dist switch, master level, pcf id switch; shuffle (303, drums, flag off); loop from bar 1, bar 2, bar 4 with chase; odd tempos 97.3, 143.21, 66.667, 200.003; full mix.
Built with: /usr/bin/python3 tests/songs/diag/gen_specs.py && /usr/bin/python3 tests/songs/diag/build.py. rbs.py check: all 145 round-trip ok.
Spot-check (locky permits free; tools/rb_export.py --restore-prf --startup-sound off, 8 songs in one run, exit 0, all opened without error, no info box): silence.wav 176400 frames, 0 non-zero samples. 303_saw_highcut peak 6447, 808_bass peak 3195, 909_flam_interval64 peak 13253, dist_a100_s0 peak 1526 (dist limits steady peak), loop_bar2 peak 4761 (first segment louder: cutoff 30 chased from bar 1 vs 100... audible change after first segment), tempo_97_3 217553 frames (= floor of 8 beats at 97.3 BPM, partial final chunk, 217553 mod 2048 != 0), delay_straight_fb64 352800 frames, peaks per eighth of the file 3676 933 119 30 3 0 0 327 (decaying tail as expected). 2-bar songs at 120 BPM: 176400 frames = 705644 bytes, as the length formula gives.
Observation for engine tasks: delay_straight_fb64.wav is zero from frame ~215880 to 352767, then the last 32 frames (352768..352799, the second half of the last 64-frame block) are non-zero (values like 24, -1, -8). Not investigated; the WAVs are not committed (references come in task-014).
Per-song 'plays what the README says' was checked by peaks/silence only for these 8; the others were checked to be well-formed (rbs.py check) but not exported. The 'disabled device' songs (dist_disabled) describe a question, not a known outcome.

Review round 1: (1) 303_slide description said the rests on steps 3,7,11,15 were replaced by slid notes, but plain303 keeps them (slides only on 1,2,5,6,9,10,13,14); description fixed to 'kept', README/specs/songs regenerated (only the README line changed; specs and .rbs identical). (2) Commands of the spot-check, run from the repo root:
  /usr/bin/python3 tools/rb_export.py --restore-prf --startup-sound off S/silence.rbs S/303_saw_highcut.rbs S/808_bass.rbs S/909_flam_interval64.rbs S/delay_straight_fb64.rbs S/loop_bar2.rbs S/tempo_97_3.rbs S/dist_a100_s0.rbs -o OUT   (S = tests/songs/diag/songs, OUT = a scratch dir; one process per song, default 10 s wait after open; exit 0)
  checks: a throwaway stats script (stdlib wave+struct: frames, peak, non-zero sample count, peak per eighth of the file) run as /usr/bin/python3 -I stats.py inside OUT; the tail check of delay_straight_fb64 was an inline script listing the non-zero frame indices. Expected frames: 2 bars at 120 BPM = 176400; tempo_97_3 = int-accumulated by the formula of docs/initial_investigation.md s3 (got 217553).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added tests/songs/diag/: gen_specs.py (song descriptions -> specs/*.json + README.md), build.py (specs -> songs/*.rbs via tools/rbs.py build), 145 specs, 145 songs, README listing each song and what it isolates. All songs are song-mode with loop set and carry initial TRAK events for every controller. 8 songs exported on the original (silence is all zero, others non-silent, lengths match the formula, loop-start chase and odd tempo fine). Not run: the other 137 songs were not exported (only rbs.py check). Testing: tool run on real songs, no cargo. Branch worktree-agent-ae7c235c1fca82f63.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-049
title: TB-303 corpus songs against their references once the delay is in
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 18:57'
updated_date: '2026-10-26 21:44'
labels:
  - engine
  - tests
milestone: m-1
dependencies:
  - TASK-023
  - TASK-026
references:
  - crates/rb-engine/tests/corpus_tb303.rs
  - crates/rb-engine/tests/fixtures/tb303_nodelay.json
  - docs/initial_investigation.md
priority: high
type: task
ordinal: 49000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Split from task-023. The TB-303 devices (crates/rb-engine/src/tb303.rs) are byte-exact on every diagnostic song whose drums are silent and whose inserts are off, checked two ways in crates/rb-engine/tests/corpus_tb303.rs: with the delay disabled against rbhost renders of those variants (tb303_songs_without_the_delay_match_rbhost, hashes in tests/fixtures/tb303_nodelay.json, variants from tools/nodelay.py), and as they are against their corpus references with a test-only model of the delay in the test file (tb303_songs_with_a_test_delay_match_their_references, `TestDelay`). What is left is the same check with the engine's own delay.

Why the corpus references need the delay: every song of tests/songs/diag (template "Silent Default Song") has the delay enabled (enabled=1, steps 3, 16ths, feedback 0, send 0). An enabled delay (0x1000F940) feeds the guard-filled send bus (1e-6 plus 1e-6 x each channel's signal) through its line and adds it, D = 16537 samples later at 120 BPM, into the master at a gain of about 1. rbhost --dump-float on 303_saw_lowcut: ref 1.2676785e-6 = 2e-6*master at frame 0, the engine without a delay 6.338391e-7 = 1e-6*master. Without it, 303_saw_lowcut has 3516 frames (7032 samples) and 303_vintage_saw 1656 frames off by -1 LSB. docs/initial_investigation.md section 8.4 has the delay's update and sample loop as traced in task-023.

What to do once task-026 has landed: remove the `#[ignore]` of `tb303_songs_match_their_references` in crates/rb-engine/tests/corpus_tb303.rs (it renders the 36 songs with `Devices::implemented()` and compares each WAV's sha256 with tests/refs/manifest.json) and make it pass; then delete `TestDelay` and its test, which the real delay supersedes. If a song differs, compare against the TestDelay render first (it matches the references), find the first differing sample with tools/rbcmp.py, and trace it.

The 36 songs (silent drums, no insert routed; derived with tools/rbs.py): the 24 303_* songs (17 ReBirth 2.0 sound, 7 Vintage), delay_303, loop_bar1, loop_bar2, loop_events_inside, mixer_level_sweep_303, mixer_master_low, mixer_pan_sweep_303, shuffle_303_g0, shuffle_303_g127, shuffle_303_g64, shuffle_flag_off, silence.

For task-024/025 (the drums), not checked in task-023: the 808's and the 909's slots 7 and 8, which the song ctor calls on every device (0x10011941, 0x10011985 with tick 0) before its locate; the 303's set its track's quantisation and insert tick-0 events (SongDevice::init_track); the drums' may do the same and need their own init_track.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The test tb303_songs_match_their_references in crates/rb-engine/tests/corpus_tb303.rs runs without #[ignore] and passes: all 36 songs listed there equal their tests/refs/manifest.json wav_sha256 at --idle-blocks 9088
- [x] #2 Any song that differed is traced to its cause, with the evidence in this task's notes; TestDelay and its test are removed once the real delay passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Un-ignore tb303_songs_match_their_references (engine devices, engine delay) and delete tb303_songs_with_a_test_delay_match_their_references.
2. corpus_tr808.rs, corpus_tr909.rs: render with render_with + Devices::implemented (engine delay); controls (no drum samples -> silent drums) stay, now with the engine delay.
3. common/mod.rs: delete TestDelay, GUARD and render_with_test_delay; keep root/json/manifest/song/samples/sha256/render_with/without_delay.
4. corpus_effects.rs: drop the 114 songs the device tests assert (pure duplication); keep the 9 distortion songs as its match test and the effect controls (delay songs listed explicitly). 36+45+33+9 = 123, the same coverage.
5. Keep tb303_songs_without_the_delay_match_rbhost: no diag song disables the delay, so it is the only reference check of the delay's disabled path (0x1000FAD1), now run by the engine's Delay.
6. Docs: initial_investigation.md 8.4 and the crate test docs no longer name TestDelay as current.
7. cargo test --release, clippy -D warnings, fmt.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a8a244525ed6ae568

Result: with the engine's devices (Devices::implemented: Tb303 x2, Distortion, engine Delay 0x1000F940) all 36 TB-303 songs equal their tests/refs/manifest.json wav_sha256 at idle_blocks 9088 on the first run; no song differed, so nothing to trace and no TestDelay comparison was needed. TB-303 test un-ignored; TR-808 (45) and TR-909 (33) tests switched from render_with_test_delay to the engine's delay, all exact.
Removed from tests/common/mod.rs: TestDelay, its GUARD const, render_with_test_delay; added differing(names, devices) (renders each diag song, asserts its frames, returns the ones whose sha256 differs), used by every corpus test.
Controls, now with the engine delay: tb303_songs_differ_without_the_303s (new: channels 0 and 1 Silent; 303_saw_lowcut, 303_vintage_saw, 303_two_voices all differ), tr808_songs_differ_without_the_808 (no samples; 808_snare, 808_bass, delay_straight_fb64), tr909_songs_differ_without_the_909 (channel 3 Silent, 808 kept; 909_snare, 909_bass, drums_kit), corpus_effects' distortion-bypass and IdleSend controls (8 dist songs, the 12 delay_* songs).
Duplication: corpus_effects rendered all 123 no-PCF/no-compressor songs, 114 of them the device tests' own. It now asserts only the 9 distortion songs; the 36+45+33+9 lists are disjoint and their union equals the old 123 list (checked: names grepped from git show HEAD:corpus_effects.rs vs the four new lists, sort -u diff empty, 123 lines).
Kept tb303_songs_without_the_delay_match_rbhost (fixtures/tb303_nodelay.json, tools/nodelay.py): every diag song has the delay enabled (the template's), so these 36 rbhost renders are the only reference check of the delay's disabled branch 0x1000FAD1 (send-bus and line refill, no L/R add), which the engine's Delay now runs in that test; it also checks the 303s without the delay's 1e-6 return of their own signal. The whole file takes 0.47 s in release.
Docs: initial_investigation.md 8.4 (task-023 item: TestDelay is history, the disabled path's only check; task-026 item: the 123 songs' split among the four tests). No other doc or crate doc named TestDelay (grep docs, crates/*/src, tools).
Commands: cargo test --release (168 passed, 0 ignored; before 167 + 1 ignored), cargo clippy --all-targets --release -- -D warnings clean, cargo fmt --all. main had not moved (3389512), no rebase needed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
All 36 TB-303 diag songs equal their corpus references with the engine's own delay (task-026): tb303_songs_match_their_references runs without #[ignore] and passes; no song differed. The TR-808 (45) and TR-909 (33) corpus tests use the engine's delay too, and TestDelay, render_with_test_delay and the TestDelay TB-303 test are deleted. A shared common::differing helper replaces the per-test loops; each device test keeps a control (303s silent, 808 without samples, 909 silent) that must differ. corpus_effects now asserts only the 9 distortion songs (plus the effect controls); the four lists are disjoint and cover the same 123 songs as before. The delay-off rbhost check is kept: it is the only reference check of the delay's disabled path 0x1000FAD1. docs/initial_investigation.md 8.4 updated. Branch worktree-agent-a8a244525ed6ae568, one commit. cargo test --release 168 pass, 0 ignored; clippy -D warnings and fmt clean.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-049
title: TB-303 corpus songs against their references once the delay is in
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 18:57'
updated_date: '2026-10-26 19:22'
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
- [ ] #1 The test tb303_songs_match_their_references in crates/rb-engine/tests/corpus_tb303.rs runs without #[ignore] and passes: all 36 songs listed there equal their tests/refs/manifest.json wav_sha256 at --idle-blocks 9088
- [ ] #2 Any song that differed is traced to its cause, with the evidence in this task's notes; TestDelay and its test are removed once the real delay passes
<!-- AC:END -->

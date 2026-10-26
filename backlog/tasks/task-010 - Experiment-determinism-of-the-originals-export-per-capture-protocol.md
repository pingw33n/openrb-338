---
id: TASK-010
title: 'Experiment: determinism of the original''s export per capture protocol'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 12:27'
labels:
  - tests
  - testmachine
milestone: m-0
dependencies:
  - TASK-004
  - TASK-007
  - TASK-008
  - TASK-009
priority: high
type: spike
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Measure how deterministic the original's export is under each capture protocol, using tools/rb_export.py and tools/rbcmp.py on a subset of the diagnostic songs (silence, a 303 note, an 808 snare pattern, a delay with feedback, a compressor song): fresh process per export; two exports in a row in one process; export after playing for some seconds; export after loading another song first. Repeat each at least three times. For every pair report equality or the first difference, and explain each difference with the state from docs/export_state.md (for example 303 phase, leftover buffer, snare counter). Conclude which protocol gives byte-identical output, or what residual variation remains and how large it is. Results (hashes, rbcmp output) in the notes and a summary section appended to docs/export_state.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each protocol was run at least three times on at least five songs, with the hashes and rbcmp results in the notes
- [x] #2 Every observed difference is attributed to a named piece of engine state, or listed as unexplained
- [x] #3 docs/export_state.md has a results section with the recommended protocol
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Songs (5, all song mode): silence, 303_accent0, 808_snare, delay_straight_fb64, comp_ch_a100_t10 (plus drums_kit as the 'other song').
2. Protocols, each run 3 times (separate rb_export.py invocations), all with --restore-prf --startup-sound off unless stated:
   P1 fresh process, open, wait T=10 s, export, quit (export_state.md section 9).
   P2 fresh process, --exports 2 (two exports in a row).
   P3 fresh process, --play-seconds 3 then export.
   P4 --reuse-process with drums_kit loaded and exported first, then the song.
   Extra: P1 with T=5 and T=20 (N dependence), and P1 with --startup-sound on.
3. Compare every pair with rbcmp.py (--json, --align) and hash; for each difference find the first differing sample and attribute it to the state of export_state.md section 7 (303 phase / N, delay pointer, drum counters, msg 33 after the first export, startup sound). IDA only if a difference is unexplained.
4. Write the results section (section 11) into docs/export_state.md with the recommended protocol; a Decide task for the startup-sound preference if it is not a settled fact.
WAVs stay in the scratchpad; hashes and rbcmp results go in the notes.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-004 (2026-10-26): test the capture protocol proposed in docs/export_state.md section 9 (restored preferences with the MME driver, startup sound off, fresh process, open the song, fixed wait, export, quit) alongside the other variants, and decide (or propose as a Decide task) the 'Play Mod Startup Sound' preference: the startup sound goes through the preview player mixed into every SafeSoundCallback, export chunks included (section 8). The residual is the idle-block count N before the export (303 phase, delay write pointer) and the four process-global drum counters; measure how the exports vary with the wait and whether the variation is explained by N.

Branch: worktree-agent-ab0a310c25047fd64

Runs (2026-10-26, test machine; each rb_export.py invocation takes all the permits; 0 failures in 37 invocations). Wrapper: rb_export.py --owner worktree-agent-ab0a310c25047fd64 --restore-prf -o OUT [opts] SONGS (scratchpad scripts run.sh, campaign.sh, extras.sh, var.sh, q2.sh). Songs: silence, 303_accent0, 808_snare, delay_straight_fb64, comp_ch_a100_t10.
- P1: --startup-sound off (wait 10 s), fresh process per song; reps r1..r3.
- P2: P1 + --exports 2 (e1, e2 in one process).
- P3: P1 + --play-seconds 3.
- P4: --reuse-process with zz_other_drums_kit.rbs (a copy of drums_kit) opened and exported first, then the song.
- Extras (1 run each): pT3 / pT30 (--wait-after-open 3 / 30); psndon0 / psndon10 (--startup-sound on, wait 0 / 10; silence and 808_snare); pv (3 reps): 808_snare and delay_straight_fb64 with both 303s silenced (tb303 enabled 0, mixer channels 0/1 enabled 0 level 0, their level track events 0; built by scratchpad mkvar.py with tools/rbs.py build); p5x: 808_snare --exports 5.
Comparison: scratchpad analyze.py (sha256 + tools/rbcmp.py --json for every pair inside a protocol), xcmp.py for cross-protocol pairs. Format below: hash prefix per run/export; for each pair 'fd' = first differing frame, 'max' = max |d| (same on both channels), then the number of differing samples.

p1 303_accent0: r1e1=7ffe49ae5452 r2e1=7ffe49ae5452 r3e1=66bdc9f1fe54
  r1e1 vs r2e1: identical
  r1e1 vs r3e1: fd 66, max 4881, 303164
  r2e1 vs r3e1: fd 66, max 4881, 303164
p1 808_snare: r1e1=c51aafa6ef7b r2e1=1fd2a582e579 r3e1=525514f2c4eb
  r1e1 vs r2e1: fd 3070, max 1, 5116
  r1e1 vs r3e1: fd 3070, max 1, 5460
  r2e1 vs r3e1: fd 3338, max 1, 1748
p1 comp_ch_a100_t10: r1e1=76c50ba733a7 r2e1=788158bd6d84 r3e1=788158bd6d84
  r1e1 vs r2e1: fd 66, max 21735, 319006
  r1e1 vs r3e1: fd 66, max 21735, 319006
  r2e1 vs r3e1: identical
p1 delay_straight_fb64: r1e1=36e7ff064b83 r2e1=37adb436c6a1 r3e1=11509b470b38
  r1e1 vs r2e1: fd 16802, max 1, 240
  r1e1 vs r3e1: fd 16605, max 1, 900
  r2e1 vs r3e1: fd 16605, max 1, 788
p1 silence: r1e1=0a90d04342bf r2e1=0a90d04342bf r3e1=0a90d04342bf
  r1e1 vs r2e1: identical
  r1e1 vs r3e1: identical
  r2e1 vs r3e1: identical
p2 303_accent0: r1e1=9650fbd20be6 r1e2=a14f0b061c5c r2e1=9650fbd20be6 r2e2=5425d3bb76d4 r3e1=9650fbd20be6 r3e2=025a4226e1bf
  r1e1 vs r1e2: fd 66, max 4876, 316980
  r1e1 vs r2e1: identical
  r1e1 vs r2e2: fd 66, max 4867, 316066
  r1e1 vs r3e1: identical
  r1e1 vs r3e2: fd 66, max 4879, 314048
  r1e2 vs r2e1: fd 66, max 4876, 316980
  r1e2 vs r2e2: fd 66, max 4884, 302086
  r1e2 vs r3e1: fd 66, max 4876, 316980
  r1e2 vs r3e2: fd 66, max 4882, 302914
  r2e1 vs r2e2: fd 66, max 4867, 316066
  r2e1 vs r3e1: identical
  r2e1 vs r3e2: fd 66, max 4879, 314048
  r2e2 vs r3e1: fd 66, max 4867, 316066
  r2e2 vs r3e2: fd 66, max 4880, 312094
  r3e1 vs r3e2: fd 66, max 4879, 314048
p2 808_snare: r1e1=c51aafa6ef7b r1e2=c88ee9e64067 r2e1=c51aafa6ef7b r2e2=0d7f9b6e07d5 r3e1=a465d579b148 r3e2=75a4e88ff595
  r1e1 vs r1e2: fd 64, max 4940, 133992
  r1e1 vs r2e1: identical
  r1e1 vs r2e2: fd 64, max 4940, 133948
  r1e1 vs r3e1: fd 3012, max 1, 3192
  r1e1 vs r3e2: fd 64, max 4940, 134000
  r1e2 vs r2e1: fd 64, max 4940, 133992
  r1e2 vs r2e2: fd 238, max 1, 1796
  r1e2 vs r3e1: fd 64, max 4940, 134014
  r1e2 vs r3e2: fd 238, max 1, 5458
  r2e1 vs r2e2: fd 64, max 4940, 133948
  r2e1 vs r3e1: fd 3012, max 1, 3192
  r2e1 vs r3e2: fd 64, max 4940, 134000
  r2e2 vs r3e1: fd 64, max 4940, 133978
  r2e2 vs r3e2: fd 342, max 1, 5546
  r3e1 vs r3e2: fd 64, max 4940, 134006
p2 comp_ch_a100_t10: r1e1=76c50ba733a7 r1e2=3be1a9f3ed15 r2e1=dac088934462 r2e2=988f2eb75bfb r3e1=76c50ba733a7 r3e2=988f2eb75bfb
  r1e1 vs r1e2: fd 66, max 21606, 298432
  r1e1 vs r2e1: fd 66, max 20390, 335052
  r1e1 vs r2e2: fd 66, max 21324, 309014
  r1e1 vs r3e1: identical
  r1e1 vs r3e2: fd 66, max 21324, 309014
  r1e2 vs r2e1: fd 66, max 20736, 334330
  r1e2 vs r2e2: fd 66, max 21392, 318504
  r1e2 vs r3e1: fd 66, max 21606, 298432
  r1e2 vs r3e2: fd 66, max 21392, 318504
  r2e1 vs r2e2: fd 66, max 20620, 334208
  r2e1 vs r3e1: fd 66, max 20390, 335052
  r2e1 vs r3e2: fd 66, max 20620, 334208
  r2e2 vs r3e1: fd 66, max 21324, 309014
  r2e2 vs r3e2: identical
  r3e1 vs r3e2: fd 66, max 21324, 309014
p2 delay_straight_fb64: r1e1=37adb436c6a1 r1e2=a7d8096f8c08 r2e1=37adb436c6a1 r2e2=a7d8096f8c08 r3e1=36e7ff064b83 r3e2=8248eef3f6e9
  r1e1 vs r1e2: fd 265, max 1, 476
  r1e1 vs r2e1: identical
  r1e1 vs r2e2: fd 265, max 1, 476
  r1e1 vs r3e1: fd 16802, max 1, 240
  r1e1 vs r3e2: fd 244, max 1, 172
  r1e2 vs r2e1: fd 265, max 1, 476
  r1e2 vs r2e2: identical
  r1e2 vs r3e1: fd 265, max 1, 276
  r1e2 vs r3e2: fd 244, max 1, 564
  r2e1 vs r2e2: fd 265, max 1, 476
  r2e1 vs r3e1: fd 16802, max 1, 240
  r2e1 vs r3e2: fd 244, max 1, 172
  r2e2 vs r3e1: fd 265, max 1, 276
  r2e2 vs r3e2: fd 244, max 1, 564
  r3e1 vs r3e2: fd 244, max 1, 408
p2 silence: r1e1=0a90d04342bf r1e2=0a90d04342bf r2e1=0a90d04342bf r2e2=0a90d04342bf r3e1=0a90d04342bf r3e2=0a90d04342bf
  r1e1 vs r1e2: identical
  r1e1 vs r2e1: identical
  r1e1 vs r2e2: identical
  r1e1 vs r3e1: identical
  r1e1 vs r3e2: identical
  r1e2 vs r2e1: identical
  r1e2 vs r2e2: identical
  r1e2 vs r3e1: identical
  r1e2 vs r3e2: identical
  r2e1 vs r2e2: identical
  r2e1 vs r3e1: identical
  r2e1 vs r3e2: identical
  r2e2 vs r3e1: identical
  r2e2 vs r3e2: identical
  r3e1 vs r3e2: identical
p3 303_accent0: r1e1=fdc72595cf05 r2e1=97e71dc937cd r3e1=fdc72595cf05
  r1e1 vs r2e1: fd 66, max 4881, 312504
  r1e1 vs r3e1: identical
  r2e1 vs r3e1: fd 66, max 4881, 312504
p3 808_snare: r1e1=5c3b26cd0732 r2e1=210824aeba8a r3e1=80c5d9dda786
  r1e1 vs r2e1: fd 68, max 1, 1580
  r1e1 vs r3e1: fd 81, max 1, 1750
  r2e1 vs r3e1: fd 68, max 1, 3138
p3 comp_ch_a100_t10: r1e1=fec4c112883b r2e1=c190c1ea2c8d r3e1=3dd9d4d81606
  r1e1 vs r2e1: fd 66, max 21960, 333710
  r1e1 vs r3e1: fd 66, max 20909, 335330
  r2e1 vs r3e1: fd 66, max 20770, 317020
p3 delay_straight_fb64: r1e1=52dd2c3817c1 r2e1=f22347954741 r3e1=07c03acb62f6
  r1e1 vs r2e1: fd 85, max 1, 1000
  r1e1 vs r3e1: fd 90, max 1, 122
  r2e1 vs r3e1: fd 85, max 1, 1026
p3 silence: r1e1=0a90d04342bf r2e1=0a90d04342bf r3e1=0a90d04342bf
  r1e1 vs r2e1: identical
  r1e1 vs r3e1: identical
  r2e1 vs r3e1: identical
p4 303_accent0: r1e1=7ffe49ae5452 r2e1=7ffe49ae5452 r3e1=7ffe49ae5452
  r1e1 vs r2e1: identical
  r1e1 vs r3e1: identical
  r2e1 vs r3e1: identical
p4 808_snare: r1e1=6edab869de57 r2e1=6edab869de57 r3e1=6edab869de57
  r1e1 vs r2e1: identical
  r1e1 vs r3e1: identical
  r2e1 vs r3e1: identical
p4 comp_ch_a100_t10: r1e1=76c50ba733a7 r2e1=3288dec11b27 r3e1=76c50ba733a7
  r1e1 vs r2e1: fd 66, max 19775, 329486
  r1e1 vs r3e1: identical
  r2e1 vs r3e1: fd 66, max 19775, 329486
p4 delay_straight_fb64: r1e1=37adb436c6a1 r2e1=dece122a73fc r3e1=dece122a73fc
  r1e1 vs r2e1: fd 16605, max 1, 582
  r1e1 vs r3e1: fd 16605, max 1, 582
  r2e1 vs r3e1: identical
p4 silence: r1e1=0a90d04342bf r2e1=0a90d04342bf r3e1=0a90d04342bf
  r1e1 vs r2e1: identical
  r1e1 vs r3e1: identical
  r2e1 vs r3e1: identical
p5x 808_snare: r1e1=c51aafa6ef7b r1e2=6d86bccd7d33 r1e3=3c45527f3cca r1e4=c1c2eabec2c7 r1e5=985e63eff6a3
  r1e1 vs r1e2: fd 64, max 4940, 133990
  r1e1 vs r1e3: fd 64, max 4819, 85154
  r1e1 vs r1e4: fd 310, max 4940, 85570
  r1e1 vs r1e5: fd 64, max 4940, 101638
  r1e2 vs r1e3: fd 458, max 4940, 84862
  r1e2 vs r1e4: fd 64, max 4940, 117274
  r1e2 vs r1e5: fd 64, max 4940, 116880
  r1e3 vs r1e4: fd 64, max 4940, 117940
  r1e3 vs r1e5: fd 64, max 4940, 100900
  r1e4 vs r1e5: fd 64, max 4940, 117566
pT3 303_accent0: r1e1=4a5fa3209a5e
pT3 808_snare: r1e1=478df0ea331d
pT3 comp_ch_a100_t10: r1e1=ced2680b14c8
pT3 delay_straight_fb64: r1e1=b1e294ec4876
pT3 silence: r1e1=0a90d04342bf
pT30 303_accent0: r1e1=433b355d46e8
pT30 808_snare: r1e1=ececf5c0ee03
pT30 comp_ch_a100_t10: r1e1=5c2c8aec7cb7
pT30 delay_straight_fb64: r1e1=6b98225e18b7
pT30 silence: r1e1=0a90d04342bf
psndon0 808_snare: r1e1=8828835c80eb
psndon0 silence: r1e1=0a90d04342bf
psndon10 808_snare: r1e1=a465d579b148
psndon10 silence: r1e1=0a90d04342bf
pv 808_snare_no303: r1e1=3d1ec685659d r2e1=3d1ec685659d r3e1=3d1ec685659d
  r1e1 vs r2e1: identical
  r1e1 vs r3e1: identical
  r2e1 vs r3e1: identical
pv delay_straight_fb64_no303: r1e1=4f562efe010c r2e1=4f562efe010c r3e1=4f562efe010c
  r1e1 vs r2e1: identical
  r1e1 vs r3e1: identical
  r2e1 vs r3e1: identical

Cross-protocol pairs (xcmp.py):
- p1_r1 vs p3_r1 808_snare: fd 64, max 4940, 101362 (snare noise counter). p1_r1 vs p4_r1 808_snare: fd 64, max 4940, 101000. p2_r1 e2 vs p4_r1 808_snare: fd 64, max 4940, 117504.
- p1_r1 vs p3_r1 303_accent0: fd 66, max 4876, 317182. comp: fd 66, max 19875, 334724.
- p1_r1 vs p3_r1 delay: fd 68, max 1, 266. p1_r1 vs p4_r1 delay: fd 16802, max 1, 240.
- psndon0 808 vs p1_r1: fd 3577, max 1, 3052; vs p1_r2: fd 3070, max 1, 5292. pT3 808 vs p1_r1: fd 3378, max 1, 3718. pT30 808 vs p1_r1: fd 3070, max 1, 5314. pT3 delay vs p1_r1: fd 16654, max 1, 434. pT30 delay vs p1_r1: fd 16605, max 1, 934.
- pv 808_snare_no303 vs p1_r1 808_snare: fd 3070, max 1, 2838; pv delay_no303 vs p1_r1 delay: fd 16613, max 1, 496.
- silence: every export of every protocol (incl. startup sound on at T=0) is 0a90d04342bf..., all 176400 frames zero.

Attribution 1, 303 phase (scratchpad shift.py, phase.py): within the first note (frames 1000..4000, L) the best integer time shift reduces rms diff from 1214 to 101 (p1_r1 vs p2_r1 e1, shift -194). Note period 680.80 samples (self-shift, parabolic). Phase offsets vs p1_r1, in periods, and nearest whole number k of idle 1024-frame buffers at 0.09491 periods/buffer (table[0] 1.8538e-4 per sample, export_state 7.2):
  p1_r3 +0.0951 -> k=+1 (err 0.0001); p2_r1 e1 +0.2850 -> k=+3 (err 0.0002); p2 e2 r1/r2/r3 0.9673/0.8724/0.0624 -> k=-11/-12/-10 with err 0.0114/0.0113/0.0115 (a common offset off the first-export grid; among themselves whole buffers); pT3 -> k=-8 (err 0.0016); pT30 -> k=-24 (err 0.0045); p3 (after play) err 0.017 (different idle pitch, grid n/a).
  Failed hypothesis: none needed for fresh exports; the 0.0114-period offset of second exports is left unattributed [G] (candidate: 303 filter coefficients +0xCF4..+0xD00 not reset by msg 33).
  IDA (headless copy, read-only): msg-33 303 reset sub_10026CC0 zeroes +0xCA4 phase (0x10026D40), +0xCB0/+0xCB4 pitch (0x10026CD4/DA), then calls vtable+0x18 (slot 6) at 0x10026D79; does not touch +0xCF4..+0xD00 (matches export_state 6.4).
Attribution 2, 303 idle output: with both 303s silenced, 808_snare and delay_straight_fb64 are byte-identical in 3/3 fresh runs (3d1ec685659d, 4f562efe010c), while with the 303s audible they vary by +-1 LSB. So the +-1 residual is the idle 303 signal flipping truncations; the delay write pointer (N-dependent) has no visible effect.
Attribution 3, 808 snare counter: IDA sub_1002A1E4: c = (byte c)+1 & 31 stored to word_1014CBD4 (0x1002A25D..0x1002A27D); noise slot = word_100340DC[word_1003409C[c]] (0x1002A28C, 0x1002A29D); bytes: word_1003409C = 0,1,2,3,1,0,3,1,0,3,2,1,0,2,3,1,0,2,3,2,0,1,2,0,3,1,2,0,1,3,2,1; word_100340DC[0..3] = 6,7,8,9. hits3.py: the 40 hits of p5x (5 exports x 8 hits) clustered by equality (<=1 LSB over 8000 frames after the hit at 22050*h); hit h of export i equals hit h of export j exactly when T1[(s_i+h+1)&31] == T1[(s_j+h+1)&31] with the unique fit s = 0, 9, 18, 27, 4: +9 per export (8 hits + step 0 triggered by the loop-end wrap in the dropped tail of the last chunk). Failed hypothesis first tried: 8 hits per export (predicts e5 == e1; e5 differs, fd 64 max 4940). hits4.py vs p1_r1: p3_r1..r3 s=7 (3 s play), p4 (after drums_kit export, 4 snare hits, none on step 0) s=4. Hits inside one export all differ even with the same noise (offset of the hit inside the 64-frame block), so only same-position hits compare.
Attribution 4, P2 second export: msg 33 state (303 phase 0, delay cleared) + new k + drum counters moved by the first export. P3 delay residual from frame 85-90: leftover echoes of the play in the delay buffer (no song switch before the export). task-008's Fonda Workout pair is the P2 case.
Startup sound: psndon0 (export started 0.04-0.07 s after the open returned, ~1.5 s after load) silence = all zeros, identical to off; 808 within +-1 (k). Decision taken here as the task notes allow: keep it off in the protocol (byte 0x20A = 0); no Decide task needed (no measurable effect, off is the safe side for mods).
Unexplained: only the 0.0114-period common phase offset of second-in-process exports (does not affect the recommended protocol).

AC check: #1 P1..P4 each 3 reps on the 5 songs (plus extras), hashes and rbcmp pairs above. #2 every difference attributed: 303 phase k = N/16 (whole buffers, err <= 2e-4 periods), 303 idle output (+-1 LSB, gone with the 303s silenced), 808 snare counter (unique fit, +9 per export), msg 33 state, delay contents after play; one item listed unexplained (0.0114-period offset of second exports). #3 docs/export_state.md section 11 with the recommended protocol (11.5).

Review round 1 (send back, 14 points), fixed by amending the one commit:
1. P2 second exports of 303_accent0: k = -11/-12/-10, steps of 1 and 2 (was "1 and 1").
2. 11.1 P2 row rewritten from the hashes: first exports are +-1 variants for 808/delay (808 c51a x2 = P1 r1, a465; delay 37ad x2, 36e7), comp 2 of 3 equal (76c5 = P1 r1), 303_accent0 9650 x3 = k +3 which no P1 run hit; second exports of comp: r2e2 = r3e2 (988f2eb75bfb). The P2 k = +3 is listed as unexplained [G].
3. The k values are now called a fit [G] with their misfits 0.0001/0.0002; the rms figures are sourced: shift.py p1_r1 vs p2_r1 e1 (1000 3000 400): signal rms 809.4, rms at 0 1214.1, best shift -194 rms 101.4; p1_r1 vs p1_r3: rms at 0 988.8, best shift -65 rms 33.6.
4. Removed "the runner's wait is 10.04-10.10 s" (it explained nothing).
5. Delay pointer: marked [G], N not recorded for the pv runs.
6. Idle 303 output mechanism marked [G] (amplitude not measured).
7. 11.3 retitled "What the measurements support, and how firmly"; step-0 wrap and startup sound marked [G]; 11.4 made consistent and states one run per variant.
8. Section 5 sentence marked [G] (inferred from the +9 fit).
9. initial_investigation.md row: +9 is "fitted", the step-0 mechanism [G]; the protocol line says "measured", with the k fit marked [G].
10. 11.5 "reproducible by a model given k" marked [G] (no model render yet).
11. Element width: both tables are 16-bit words (IDA: mov bx, word_1003409C[ecx*2] at 0x1002A28C; mov si, word_100340DC[ebx*2] at 0x1002A29D). The earlier note's "bytes:" meant "values read from the database bytes"; the values listed are the word values. Doc 7.3 states this.
12. Run count: 32 rb_export.py invocations (runs.txt: P1-P3 9, P4 15, pT3/pT30/psndon0/psndon10 4, pv 3, p5x 1); "37" in the earlier note and the commit body was a miscount.
13. Doc 11.4 and 11.2 say the startup-sound decision rests on one run per variant and that the 808 +-1 results are discrete k variants (psndon10 808 a465d579b148 = P2 r3 e1).
14. Committed tools/experiments/task010/: README.md (exact capture commands for all 32 runs and analysis commands), wav.py, analyze.py, xcmp.py, shift.py, phase.py, snare.py (hits3/hits4 merged: 'chain' and 'ref' modes), mkvar.py, songs/{808_snare,delay_straight_fb64}_no303.{json,rbs} (rebuilt by mkvar.py with the same SHA-256 as the run's: 670ee766..., 334e0772...). zz_other_drums_kit is a byte copy of drums_kit (5461741463bd...), not committed. Re-ran on the scratch WAVs: phase.py gives the same k and misfits; snare.py chain gives 0, 9, 18, 27, 4; snare.py ref gives 7 (P3) and 4 (P4).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Measured the determinism of the original's Export Loop as Audio File under four capture protocols (fresh process; two exports in one process; play 3 s first; another song loaded and exported first), 3 runs each on silence, 303_accent0, 808_snare, delay_straight_fb64 and comp_ch_a100_t10, plus one run each with waits of 3/30 s and the startup sound on (T 0/10), 3 runs of 303-silenced variants and one run of five exports in one process (32 rb_export.py runs, no failures; WAVs not kept, hashes and rbcmp results in the notes).

Result: under the section 9 protocol (fresh process, restored prefs, startup sound off, wait 10 s) exports were byte-identical across runs when the song's 303s were silent (two songs, 3 runs); with audible 303s but no notes they vary by +-1 LSB on up to 1.6 % of samples in discrete variants (attributed [G] to the 303 idle output flipping truncations); with 303 notes the differences fit whole idle buffers of 303 phase (k = 1, 3; misfit <= 2e-4 periods). The global 808 snare counter fits +9 per 8-hit export (unique fit over five exports, table word_1003409C from IDA; [G] the ninth hit is step 0 from the dropped tail). Startup sound: no effect in one run per variant; kept off. Recommended: rb_export.py --restore-prf --startup-sound off --wait-after-open 10, one process per song, k fitted by rbhost. Unexplained [G]: why all three P2 first exports had k = +3, and a 0.0114-period phase offset of second-in-process exports.

Committed: docs/export_state.md section 11 and updates to sections 1, 5, 7.3, 8, 10; docs/initial_investigation.md section 9; tools/experiments/task010/ (scripts, README with the exact commands, 303-silenced variant songs). Branch worktree-agent-ab0a310c25047fd64, one commit (amended after review round 1). Tests: the experiment runs; the committed scripts re-run on the captured WAVs reproduce the k and +9 fits.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-026
title: 'Effects: distortion and delay'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 21:37'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-021
  - TASK-023
priority: medium
type: feature
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The distortion insert (docs/initial_investigation.md section 8.1: 16-block update with the 5x5 grid interpolation, the 0.9/0.1 gain smoothing, the waveshaper table lookup, the "1.5 mode" for shape 0) and the delay send effect (section 8.4: the global 131072-sample buffer prefilled with 1e-6, the integer delay-length formula with the mode polarity resolved in docs/song_load.md, feedback, pan via PAN row 127, the never-reset write pointer as initial state). Tests: the distortion and delay corpus songs match their references.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every distortion and delay corpus song matches its reference per the pass criterion
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE from the disassembly (idalib, scratchpad copy, read-only): CDelayEffect vtable 0x10032CB4 (ctor 0x1000F5A0, msg handler 0x1000FB40, set_param 0x1000FDC0, slots 5/6/8/9, update 0x1000F754, block 0x1000F940) and CDistInsert vtable 0x10033144 (ctor 0x10010700, update 0x10010894, block 0x10010A50, slot 16 0x10010D40, msg handler 0x10010D60, set_param 0x10010FB0, slots 8/9).
2. ProcessState: the global delay line flt_10054794 (131072 f32) and its gate byte_10054790, shared by the song's devices, with clear (slot 6), read and preset at export start. Engine::activate(ReplacesCurrent) clears it on the process state.
3. crates/rb-engine/src/delay.rs (SendEffect) and dist.rs (InsertEffect), wired into Devices::implemented; BlockContext gets the song's tempo (sub_1001FCF0).
4. Unit tests: waveshaper lookup, 5x5 grid interpolation, 0.9/0.1 smoothing, delay length formula, hand-computed.
5. Corpus test corpus_effects.rs: every diag song without a routed PCF or compressor (dist_*, delay_*, and the rest whose only non-303/808/909 devices are the distortion and the delay) against its reference with the engine delay; plus the 303/808/909 lists with the engine delay (TestDelay tests left in place).
6. CLI: drop distortion and delay from the not-implemented warning.
7. docs: initial_investigation 8.1/8.4 and song_load section 4 / 11 updates with addresses and [C].
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26, from task-022's review: msg 53 activation's real effect is the process-global delay buffer 0x10054794 (refilled with 1e-6 by the delay's slot 6 at 0x100100E0, which also sets byte_10054790 = 1). Engine::activate (crates/rb-engine/src/frame/mod.rs, ReplacesCurrent path) calls silence() on a placeholder; this task must make the delay's buffer process-global state reachable from Engine and model slot 6 on it, settable at export start like rbhost.

Branch: worktree-agent-a8f879706c2a64335 (worktree .claude/worktrees/agent-a8f879706c2a64335).

RE (idalib on a scratchpad copy of ida/ReBirth.dll.i64, force_headless, read-only, closed save=false):
- Delay, vtable 0x10032CB4: slots [0] 0x1000F500, [1] 0x10010380, [2] 0x1001BF40, [3]/[4] 0x10012C00, [5] 0x1000F4F0 (calls slot 6), [6] 0x100100E0 (fill flt_10054794 with 0x358637BD, byte_10054790 = 1), [7] nullsub 0x1000F4E0, [8] 0x10010020 (insert pattern ctrl 0..4 at the tick), [9] 0x100100B0 (6-byte live/pattern copy), [10] 0x1000FB40 (msgs 81..86; the jump table is indexed by the raw byte: [eax*4+0x101516A8], entries at 0x101517EC; 86 = 0x1000FD7D: pattern := payload, live := pattern when sub_1001FCB0 == 0), [11] 0x1000FDC0 (ctrl 0..4 live), [14] 0x1000F940. ctor 0x1000F5A0: live/pattern = sub_10019AB0 = 1,3,1,80,64,0.
- Delay update 0x1000F754: the 25-update tempo average (+0x48/+0x4C/+0x50) is dead (+0x4C overwritten with sub_1001FCF0 at 0x1000F7E2); 26460000*steps built by shifts at 0x1000F804..0x1000F834; the tail writes the meter byte into live byte 5. Block 0x1000F940: operand order re-traced, identical to TestDelay; adds the send-bus peak meter (+0x70). Disabled path also zeroes byte 5 and sets the peak to 1e-6.
- Distortion, vtable 0x10033144: [4] 0x100106C0 and [5] 0x1000F4F0 call slot 6 = no-op 0x10012C00; [8] 0x10010FE0 (ctrl 0..2), [9] 0x100111F0 (4 bytes), [10] 0x10010D60 (msgs 89..94; 94 = 0x10010F77, same rule), [11] 0x10010FB0, [15] 0x10010A50, [16] 0x10010D40 (meter byte cleared when the counter did not move). ctor 0x10010700: sub_10019B30 = 1,70,0,0.
- Distortion update 0x10010894 and block 0x10010A50 traced on the x87 stack: grid 0x10151AB8 (25 f32), constants 0x10151B20..0x10151B94 decoded from bytes (3.999 = 0x400FFDF3B645A1CB, 0.0625, 2048, 5, 1/256, 0.92f, f32 1/127, 127f, 1.0, 0.9, 0.1, 16, 1, -1, 0f, 128f, 254f). New facts: the shaper clamps to W[254] above 254.0 (fld [ebp+3F8h]), not W[255]; the shaper loop reads the stored f32 gains (the delay uses the unrounded ones); a disabled distortion returns at once (bus unchanged, counter stands); one object serves every routed channel, so its counter advances per routed channel.
Implementation: crates/rb-engine/src/delay.rs, dist.rs (+ unit tests in delay/tests.rs, dist/tests.rs); ProcessState holds the delay line (DelayLine: buf + gate) behind Arc<Mutex>, with delay_line / set_delay_line / refill_delay_line; BlockContext gets the song tempo; Devices::implemented installs both; CLI warning lists only PCF and compressor.
Song set (scope a), derived with tools/rbs.py json over all 145 diag songs (script classify.py in the scratchpad: no PCF and no compressor id non-zero in the MIXR chunk or any mixer-track event): 123 songs, all idle_blocks 9088. Distortion routed: dist_a0_s0, dist_a100_s0, dist_a100_s127, dist_a100_s64, dist_a127_s0, dist_a20_s0, dist_disabled (device off), dist_drums, mixer_dist_switch; delay: delay_* (12) and mixer_delay_send_sweep, plus every other of the 123 (all have the delay enabled). Left out (PCF or comp): comp_* (6), pcf_* (13), mixer_pcf_switch, full_mix.
Result: first run with the engine delay and distortion, all 123 byte-exact against tests/refs (crates/rb-engine/tests/corpus_effects.rs). Controls: with the distortion bypassed all 8 enabled distortion songs differ; with IdleSend all 12 delay_* songs differ.
Scope b: the 36 TB-303, 25 TR-808 and 33 TR-909 corpus songs are all in the 123 and pass with the engine delay; the TestDelay tests are left in place and still pass; tb303_songs_match_their_references (ignored, task-049) passes with --ignored; tb303_songs_without_the_delay_match_rbhost passes with the engine's disabled delay.
CLI + comparator: target/release/rb render <song> --output X.wav --idle-blocks 9088 --rebirth-dll original/ReBirth.dll, then tools/rbcmp.py X.wav tests/refs/diag/<song>.flac: EQUAL for dist_a100_s64, dist_a127_s0, dist_drums, delay_triplet_fb127, delay_straight_fb127, tempo_66_667 (script cmp.sh in the scratchpad). The CLI's remaining warning still keys on the PCF/compressor enabled byte, not their routing, so it shows on every diag song; that is task-027's area.
No rbhost dump was needed (no test-machine permit taken).

Correction to the note above: the TR-808 corpus test has 45 songs (TR808_SONGS.len() == 45), not 25; all 45 are in the 123.

Correction: the left-out PCF songs are pcf_* (14), not 13 (6 comp_* + 14 pcf_* + mixer_pcf_switch + full_mix = 22).
Review round 1 (send back, small items), fixed:
1. Engine-level tests of the refills: frame/tests.rs msg33_reset now installs the engine Delay, dirties the global line (0.5, gate clear) and asserts Engine::reset refills it with 1e-6 and sets the gate; new msg53_replacing_a_song_refills_the_delay_line checks Engine::activate for ReplacesCurrent and NoneCurrent the same way.
2. A distortion amount or shape above 127 indexed GRID[..][5] (and T, W) and panicked: set_param and msg 94 now clamp them to 127, with a comment that the DLL does not check and would read adjacent memory (the track load accepts only 0..127, song_load.md section 5). The same defect existed for the delay's pan (PAN[127][pan] at flt_10054150): clamped the same way. Tests amount_and_shape_above_127_are_clamped (128, 255) and a_pan_above_127_is_clamped (200, 255).
3. CLI warning: now names the PCF or the compressor only when the song may route them (a non-zero pcf_id/comp_id in the mixer chunk or in any mixer-track event of controller 2/1). New crates/rb-cli/tests/render.rs only_routed_pcf_and_compressor_warn: no warning on 303_saw_lowcut, dist_a100_s64, delay_straight_fb64; PCF only on pcf_on_909; compressor only on comp_master_a100_t10.
4. The pcf_* count corrected above.
cargo test --release: 167 pass, 1 ignored; corpus_effects still 123 exact; clippy and fmt clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Engine distortion (crates/rb-engine/src/dist.rs) and delay (crates/rb-engine/src/delay.rs) implemented from the disassembly (CDistInsert 0x10033144: update 0x10010894, block 0x10010A50; CDelayEffect 0x10032CB4: update 0x1000F754, block 0x1000F940, slot 6 0x100100E0), wired into Devices::implemented. The process-global delay line flt_10054794 and its gate byte_10054790 are now ProcessState state (DelayLine; delay_line, set_delay_line for export start, refill_delay_line), shared by the song's devices; msg 53 and msg 33 refill it through the delay's slot 6 (tested at engine level). BlockContext carries the song tempo (sub_1001FCF0). Bytes above 127 for the distortion's amount/shape and the delay's pan are clamped (the DLL would read adjacent memory).
Result: all 123 diag songs with no PCF or compressor routed (dist_*, delay_*, mixer_dist_switch, mixer_delay_send_sweep, tempo_*, and every 303/808/909 song) render byte-exact against tests/refs at their idle_blocks (9088) with the engine's own delay; controls show the distortion and delay songs differ without the effect. The TestDelay-based tests stay in place and pass (task-049 switches them). The CLI warns about the PCF or compressor only when the song routes them.
New facts in docs/initial_investigation.md 8.1/8.4 and docs/song_load.md section 4/11 ([C]: msg-41 rule for delay and distortion, shaper clamp to W[254], stored vs unrounded gains, dead tempo average, meter bytes, slot map).
Tests: cargo test --release 167 pass, 1 ignored (task-049's, passes with --ignored); unit tests for the shaper lookup, the grid interpolation, the 0.9/0.1 smoothing, the length formula and the clamps; corpus_effects.rs; rb CLI + tools/rbcmp.py EQUAL on 6 songs; clippy and fmt clean. Review round 1 fixed. Branch worktree-agent-a8f879706c2a64335, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

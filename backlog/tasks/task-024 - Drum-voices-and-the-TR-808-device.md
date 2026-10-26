---
id: TASK-024
title: Drum voices and the TR-808 device
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 20:45'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-020
  - TASK-022
priority: high
type: feature
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The four drum voice classes (SIMPLE, FILTER, DUAL, PITCHED: docs/initial_investigation.md section 7.1, the pair-wise gain update, block-end f32 stores, end conditions, padding) and the TR-808 device (sections 7.2, 7.3: step function with ascending trigger order, accent, BD voice alternation with the global flag, snare noise from SEQ808 with the global counter, CH choke, selectors, the tune table, the fixed voice summation order). The global counters/flags are part of the initial-state struct (their values come from the capture protocol). Tests: every 808 corpus song matches its reference.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every TR-808 corpus song matches its reference per the pass criterion
- [x] #2 The voice classes have unit tests against hand-computed block outputs
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE from the disassembly (scratchpad copy of ReBirth.dll.i64): the four voice classes (SIMPLE 0x1001E300, FILTER 0x1001E4D0, DUAL 0x1001E700, PITCHED 0x1001E9A0, triggers 0x1001E2B4/0x1001E5C0/0x1001E820/0x1001EAD0, slot 1 0x1001EB30), the 808 ctor 0x10026F70, step 0x10027690, the instrument dispatch 0x10027894 and its helpers 0x1002A004/0x1002A0B4/0x1002A1E4, the audio slot 0x100280B0, slots 2, 3, 5, 6, 7, 8, 9, 11 and msg 56.
2. crates/rb-engine/src/drums.rs (+ drums/voice.rs): the four voice classes, reusable by the 909 (task-025), and the process-global drum counters as an engine-owned shared handle (DrumCounters / ProcessState), settable before an export like rbhost --drum-counters.
3. crates/rb-engine/src/tr808.rs: the device (fields with offsets, step, triggers, audio order, slots incl. init_track), reading the samples from an Arc<SampleSet>.
4. frame: Devices::implemented takes the optional SampleSet and owns the process handle; the rb CLI passes its loaded set.
5. Tests: voice unit tests against hand-computed blocks (AC #2), 808 unit tests (accent, order, choke, BD alternation, snare sequence), a corpus test over the 808 songs (808_* plus 808(+303)+mixer-only songs from tools/rbs.py) byte-exact against tests/refs at their idle_blocks with TestDelay moved to a shared test helper.
6. Docs: initial_investigation.md 7.1-7.3 and 6.4/9 updates, export_state.md 7.3, with addresses.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26, from task-021's review: the 808 and 909 snare and bass-drum counters are process-global, not per song (docs/export_state.md:359-362, section 7.3; rbhost's --drum-counters). The frame's BlockContext (crates/rb-engine/src/frame/mod.rs:71-77) is immutable and has no seam for process state: add an engine-owned process-state handle shared by both drum devices, with its initial value settable by the export entry point (to match rbhost --drum-counters).

Supervisor 2026-10-26, from task-023: the song constructor's slots 7 and 8 (Engine init_track, see crates/rb-engine/src/frame/ and tb303.rs) were read only for the 303s, where they move enable and pattern events to the next bar and insert tick-0 events. Read the 808's and 909's slots 7 and 8 and implement their init_track. Also, delay-on corpus checks need the delay: until task-026 lands, use the test-only TestDelay in crates/rb-engine/tests/corpus_tb303.rs (or move it to a shared test helper) so drum songs can be compared against their references.

Branch: worktree-agent-a9e4fafe745ab5446

RE evidence (disassembly, scratchpad copy of ida/ReBirth.dll.i64, read-only):
- Voice fields (all classes): +4 sample object, +8 active, +0xC sample pointer, +0x10 frames left, +0x14 hold (blocks), +0x18 gain g, +0x1C multiplier m. FILTER +0x20 c, +0x24 y. DUAL +0x20 B pointer, +0x24 g2, +0x28 B object. PITCHED +0x20 sample base, +0x24 step, +0x28 phase (24.8), +0x2C frames<<8.
- Trigger 0x1001E2B4 (sample, f32 gain, m, hold): g = f32(gain * 2^-15) (dbl_10153640 = 0x3F00000000000000), ptr = data, left = frames (+0x4C of CWaveSample), active = 1. FILTER 0x1001E5C0 adds y = 0, c. DUAL 0x1001E820 adds B, g2, left = min(lenA, lenB). PITCHED 0x1001EAD0 adds step, phase 0, limit frames<<8.
- Process (32 pairs, mult = 1.0 (flt 0x3F800000) while hold > 0 (hold--), else m; g kept in x87): SIMPLE 0x1001E300 out = f32(out + g*s) per sample, g' = g*mult after the pair; end: g < 3e-8f (fcom with 0x3300D959 on the unrounded g) or left -= 64 <= 0. FILTER 0x1001E4D0: y = y*c + g*s per sample (x87), out = f32(out + y); y and g stored f32 at block end. DUAL 0x1001E700: out = f32(out + g1*(A + g2*B)), m applies to g2, g1 constant (stored back unchanged; the end test is the integer compare of g1's bits with 0x3300D959), left by min length. PITCHED 0x1001E9A0: s = i16(a + ((frac*(b-a)) >> 8)) (imul, sar, then shl/sar 16), out = f32(out + s*g), phase += step per sample; ends when phase > frames<<8 (unsigned, no gain test).
- Slot 1 0x1001EB30: active = 0, g = 0 (all classes; vtables 0x10033D54/64/74/84).
- 808 (0x1910 bytes, ctor 0x10026F70): +0x30 pattern-mode params (32 bytes, sub_10018FB0 defaults 1,0,40,100,64,64,100,64,64,100,64,0,100,64,0,100,64,0,100,0,100,0,100,100,0,100,100,100,100,0,0,0), +0x50 32 patterns x 194 (shuffle, length 16, 16 x 12 step bytes, zero), +0x1890 live params (byte 29 +0x18AD = last step), +0x18B0 32 changed flags (panel only), +0x18D0 current pattern, +0x18D1 enable latch (1), +0x18D2 wrap, +0x18D4 last pattern position, +0x18D8 sample table (slot n at +4n: 1 BD, 2..6 SD1..5, 7..10 N1..4, 11 CB, 12 CH, 13 CL, 14 CP, 15 CY, 16 HC, 17 HT, 18 LC, 19 LT, 20 MA, 21 MC, 22 MT, 23 OH, 24 RS), +0x18DC SIMPLE x5 (RS/CL, CP/MA, CB, OH, CH), +0x18F0 PITCHED x3 (LT, MT, HT), +0x18FC FILTER x3 (BD-A, BD-B, CY), +0x1908 DUAL (SD), +0x190C the step's accent (f32).
- Step 0x10027690 = the 303's position logic without gate-off: v = (u32)(t' - base) % (48*len), step = v/48, wrap if lastPos - v > 24; on a new step or wrap: bar line (unshuffled tick/48 & 15 == 0): pending pattern + patStart = 768*(tick/768), latch = live[0]; if latch: acc = f32(row[0]*live[2]*0.03125f + 1.0) (+0x190C), then instruments 1..11 ascending through 0x10027894 (jump table at 0x10153F58 + 4*i).
- Instrument triggers (k = flt 1/127 0x3C010204): BD/CY 0x1002A0B4: x = (((L*k)*L)*0.005f)*acc, c = TONE8[tone>>4] (0x100340E4), m = DEC16[decay>>3] (0x10034104), BD gain (1.0 - c)*x, CY x; BD voice by word_1014CBD0: 0 -> voice A, B.m = 0.99f (0x3F7D70A4), flag = 1; else voice B, A.m = 0.99, flag = 0. SD 0x1002A1E4: g1 = f32((((L*k)*L)*0.025f)*acc), g2 = f32((snappy*k)^2), A = slot SD[(tone*5)>>7]+1, counter = (counter+1)&31 (word_1014CBD4), B = slot N[SEQ808[counter]]+1, m 1, hold 0. Toms 0x1002A004: PITCHED[i-3], g = f32((((L*k)*L)*k)*acc), step = T808[tune], m 1, hold 0; selector 0 picks the conga. RS/CL, CP/MA, CB (inline): SIMPLE 0/1/2, g = f32((((L*k)*L)*0.005f)*acc), m 1, hold 0; selector 0 picks CL/MA. OH: SIMPLE 3, L = lvl>>1, m = 0.998f (0x3F7F7CEE), hold = 4*decay. CH: SIMPLE 4, L = (2*lvl)/3, m 1, hold 0, then hold = 0 on all five SIMPLE voices (0x1001E3C0).
- The voice-select switches in 0x10027894 jump through constant entries (off_10153F98 etc.); IDA shows them as computed jumps; each resolves to one fixed voice (read from the bytes at 0x10153F88..0x1015401C).
- Audio 0x100280B0: zero the bus, then FILTER A, FILTER B, DUAL, PITCHED LT/MT/HT, SIMPLE RS/CL, CP/MA, CB, OH, CH, FILTER CY (per-instrument outputs null in the export: all into bus 0).
- Slots: 2 0x10028030 (patStart = bar of trunc(pos), pending pattern, latch), 3 0x10028080 (wrap = 1, patStart), 4 0x100106C0 and 5 0x1000F4F0 -> 6, 6 0x1002A2D0 (slot 1 on every voice), 7 = 0x10026C40 (the 303's: ctrl 0 and 1 quantised to 32), 8 0x10029DA0 (tick-0 events of ctrl 0..28 from +0x30), 9 0x10029E30 (32-byte copies like the 303), 11 0x10029050 (ctrl 0..28 live), msg 56 at 0x1002884C (sub_10019490: params + patterns into +0x30; pattern mode copies the 32 bytes live).

Implementation (crates/rb-engine): src/drums.rs (Simple, Filter, Dual, Pitched; reusable by the 909), src/process.rs (ProcessState: the four process-global drum counters, shared by Arc between the song's devices, settable with set_drum_counters like rbhost --drum-counters A,B,C,D; Engine::process_state()), src/tr808.rs (the device), frame: Devices::implemented(Option<Arc<SampleSet>>) and Devices::process; samples.rs: SampleSet keeps Arc<Sample> (shared(), from_samples(), Sample::from_audio()). rb CLI: passes its loaded set to the engine; a drum machine counts as enabled also when an enable event (ctrl 0) turns it on (else the samples would not be loaded and the 808 would stay silent); TR-808 is no longer in the not-implemented warning.
Corpus songs (derived with tools/rbs.py, scratchpad classify.py: an 808 pattern holds a step within its length, no 909 step, no insert routed in the chunk or the mixer track): the 25 808_* songs, delay_automated, delay_pan0, delay_pan127, delay_straight_fb0/fb127/fb64/steps1/steps8, delay_triplet_fb0/fb127/fb64, loop_chase, mixer_delay_send_sweep, mixer_level_sweep_808, mixer_master_full, mixer_pan_sweep_808, tempo_143_21, tempo_200_003, tempo_66_667, tempo_97_3 (45; a 303 plays in loop_chase, mixer_master_full and the tempo_* songs).
Result: all 45 byte-identical to tests/refs (manifest wav_sha256, header included) at their idle_blocks (9088) on the first run, with TestDelay (moved to crates/rb-engine/tests/common/mod.rs, shared with corpus_tb303.rs) for the delay. No differing sample, so no rbhost dump was needed and no test-machine permit was taken. Control: 808_snare with the silent placeholder differs from its reference. The export of 808_snare ends with the snare counter at 9 (8 hits in the loop + step 0 in the dropped tail), the +9 measured from app exports: docs/export_state.md 5, 11.2, 11.3 and investigation 9 updated from [G] to [C].
Without the delay: rb render of 808_snare against its reference differs by +-1 LSB on 2932 samples from frame 68 (the delay's 1e-6 contributions only), as for the 303 songs.
Failed hypotheses: none in the engine. One unit-test expectation was wrong (a step call at tick 768 right after tick 0 is the same step 0 without a wrap, so no bar-line latch); the test now steps 720 then 768.
Songs needing other devices (later tasks): 909 (task-025): the 29 909_* songs, mixer_level_sweep_909, mixer_pan_sweep_909, drums_kit, shuffle_drums_g64, shuffle_drums_g127, pcf_on_909 (also the PCF). 808 with inserts (the distortion, PCF and compressor tasks): comp_ch_808, comp_master_a100_t10, comp_master_a127_t0, dist_drums, mixer_pcf_switch, pcf_bp_wave0/64/127, pcf_lp_amount0_decay64, pcf_lp_amount127_decay0, pcf_lp_amount127_decay127, pcf_lp_wave0/64/127. full_mix needs everything. Every song needs the engine's delay (task-026) to match without TestDelay.
Reproduce: cargo test --release --test corpus_tr808; cargo test -p rb-engine --lib drums; cargo test -p rb-engine --lib tr808; scratchpad cli_check.sh (rb render of 808_snare with --rebirth-dll original/ReBirth.dll, then tools/rbcmp.py against the decoded reference).
Tests: cargo test 134 pass, 1 ignored (the 303 songs without TestDelay, task-049); clippy -D warnings clean; fmt clean.

AC #1 evidence: crates/rb-engine/tests/corpus_tr808.rs tr808_songs_with_a_test_delay_match_their_references passes: all 45 TR-808 corpus songs (list above) have the reference's wav_sha256 at their idle_blocks 9088, with the samples of original/ReBirth.dll and TestDelay for the delay; the control test shows the 808 is audible in them.
AC #2 evidence: crates/rb-engine/src/drums/tests.rs, 11 tests of SIMPLE, FILTER, DUAL and PITCHED against exact hand-computed blocks (power-of-two gains, integer samples): the per-pair decay, the hold, the end by gain (3e-8) and by length with the padding, the FILTER impulse response and its unrounded state, the DUAL mix with g2 decaying and the min length, the PITCHED 24.8 interpolation with the arithmetic shift, the end past frames<<8 and no gain test.

Review round 1 (send back), fixes:
1. CLI test for the enable-event clause: crates/rb-cli/tests/rebirth_dll.rs drums_enabled_only_by_track_events_load_the_samples, on the shipped Trakka Rekka (both drum chunks off, enable events turn them on): with a missing --rebirth-dll the error names "TR-808 and TR-909" (the samples are needed); with original/ReBirth.dll it renders and the not-implemented line names TR-909 but not TR-808. (The earlier notes did not claim a test; now there is one.)
2. Side effect of that clause: the CLI now prints "not implemented yet: ... TR-909" for the shipped Clodhopper, Metallicon City, Mutilated Species II and Trakka Rekka (909 chunk off, turned on by enable events), which it did not before; their 909 does play in the original, so the warning is right. Shipped songs checked with tools/rbs.py (scratchpad shape.py).
3. Docs: the +9 count stays [C] (measured from app exports); the mechanism (step 0 at the loop wrap in the dropped tail) is now [G], derived from the engine model with its code path (sub_100139E0 renders the last partial block whole at 0x10013BAA, the block's sequencer tick 0x1001346A -> sub_1001EF20 wraps and calls the 808 step 0x10027690), not observable in the corpus: docs/export_state.md 5, 11.2, 11.3 and initial_investigation.md 9. The corpus_tr808 test doc says the same.
4. drums/tests.rs: filter_keeps_y_unrounded_within_the_block now uses c = 0.5 and an input 1, 1024, -512 whose exact outputs are 2^-n except 1024 at sample 30 and 2^-31 at 31 (0 if y were rounded per sample); filter_with_a_decaying_gain_matches_its_closed_form checks y_n = sum 2^-((n - j) + j/2) (first values 1, 1.5, 1.25, 1.125 by hand).
5. dual_ends_at_once_when_g1_is_below_3e_8 asserts g1 = 1e-8f and every output exactly 2 * 1e-8f.
6. process.rs doc: one handle per Engine; a second Engine in the same OS process starts at zero, unlike the original; copy drum_counters() to carry them.
Tests: cargo test --release 136 pass, 1 ignored; clippy -D warnings and fmt clean; corpus_tr808 still exact on 45 songs.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The four drum voice classes (crates/rb-engine/src/drums.rs, reusable by the 909) and the TR-808 device (crates/rb-engine/src/tr808.rs) implemented from the disassembly: the step function, the accent, the ascending triggers, the BD voice alternation, the SEQ808 snare noise, the CH choke, the selectors, T808, the fixed voice summation order, and slots 2..11 including init_track (slot 7 = the 303's 0x10026C40, slot 8 0x10029DA0). The process-global drum counters are an engine-owned shared handle, one per Engine (crates/rb-engine/src/process.rs, ProcessState, settable like rbhost --drum-counters; Devices::process, Engine::process_state). Devices::implemented takes the optional SampleSet; the rb CLI passes its loaded set and now also counts a drum machine as playing when only an enable event turns it on (so it now warns TR-909 not implemented for Clodhopper, Metallicon City, Mutilated Species II and Trakka Rekka). TestDelay moved to crates/rb-engine/tests/common/mod.rs.
Result: all 45 TR-808 corpus songs (808_* and the 808(+303)+mixer-only songs) are byte-identical to tests/refs at --idle-blocks 9088 with TestDelay; the model's 808_snare export moves the snare counter by 9, the measured count (the dropped-tail mechanism stays [G], model-derived). Docs: initial_investigation.md 6.4, 7.1, 7.2, 7.3, 9; export_state.md 5, 7.3, 11.2, 11.3.
Tests: cargo test 136 pass, 1 ignored (task-049); clippy and fmt clean. Branch worktree-agent-a9e4fafe745ab5446, one commit (amended after review round 1). Follow-ups: the songs needing the 909, the inserts and the engine's delay are listed in the notes (tasks 025, 026 and the effect tasks).
<!-- SECTION:FINAL_SUMMARY:END -->

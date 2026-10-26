---
id: TASK-025
title: TR-909 device
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 21:08'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-024
priority: high
type: feature
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The TR-909 device (docs/initial_investigation.md sections 7.2, 7.3): descending trigger order, step values and gains (x0.8, x2.0), flam (flamTick, retrigger at x1.5), BD sample switching and alternation, the snare noise with SEQ909, toms/crash/ride pitched voices with kv, CH/OH decay and choke, the DT table. Tests: every 909 corpus song matches its reference.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every TR-909 corpus song (including flam) matches its reference per the pass criterion
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE from the disassembly (scratchpad copy of ida/ReBirth.dll.i64): the 909 ctor 0x1002A380 and vtable 0x10034294, step 0x1002AAC0 (flam, accent, descending triggers, the three gain switches), triggers 0x1002D8B4 (RS/CP/CH/OH), 0x1002DA24 (toms/crash/ride), 0x1002DB24 (BD), 0x1002DC54 (SD), audio 0x1002B6F0, slots 2, 3, 6, 7, 8, 9, 11 and msg 73 (0x1002BE90 -> sub_100199A0/sub_100196F4).
2. crates/rb-engine/src/tr909.rs: the device on the task-024 voice classes, the 909 counters in ProcessState (next_snare_909, bd_909), sample slots 25..45 = IDs 1025..1045.
3. frame: Devices::implemented also builds the 909 given the samples; rb CLI: drop TR-909 from the not-implemented warning (and its test).
4. Tests: 909 unit tests (step values x0.8/x2.0, flam retrigger x1.5 after flamTick, descending order and the CH choke, accent, BD alternation, SEQ909 snare, the 20..22 reorder), a corpus test over the 33 909 mixer-only songs (derived with tools/rbs.py) byte-exact at idle_blocks 9088 with TestDelay.
5. Docs: initial_investigation.md 5 (flam), 7.1-7.3, song_load.md 4.8 (the internal 20..22 order), export_state.md where touched.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a2cf6498ef145e173

RE evidence (disassembly, scratchpad copy of ida/ReBirth.dll.i64, read-only):
- vtable 0x10034294: slot 1 0x1002AAC0, 2 0x1002B670, 3 0x1002B6C0, 4 0x100106C0, 5 0x1000F4F0, 6 0x1002DD40, 7 0x10026C40 (the 303/808 one: ctrl 0 and 1 quantised to 32), 8 0x1002D480, 9 0x1002D510, 10 0x1002B8E0 (host messages; 'I' = 73 at 0x1002BE90), 11 0x1002C6D0, 14 0x1002B6F0.
- Fields (ctor 0x1002A380, 0x1920 bytes): +0x30 pattern-mode params (33 bytes, sub_100195A0), +0x51 32 patterns x 194, +0x1891 live (byte 29 flam, 30 last step), +0x18B2 33 changed flags, +0x18D3 pattern, +0x18D4 enable latch (1), +0x18D5 wrap, +0x18D8 last pos, +0x18DC sample table, +0x18E0 SIMPLE x4 (RS, CP, CH, OH), +0x18F0 PITCHED x5 (LT, MT, HT, crash, ride), +0x1904 FILTER x2 (BD A, B), +0x190C DUAL (SD), +0x1910 flam armed, +0x1914 flam tick, +0x1918 accent, +0x191C trigger gain.
- The DLL's parameter blocks hold bytes 20..22 as HH, RS, CP (RBS/controller order RS, CP, HH): msg 73 parse sub_100196F4 (internal[21]=msg[20], [22]=[21], [20]=[22]), export sub_10019624 the inverse (used by slot 8), set-param 0x1002C6D0 ctrl 20 -> +0x18A6, 21 -> +0x18A7, 22 -> +0x18A5.
- Step 0x1002AAC0: the 808's position logic; on a new step live[30] = step, flamTick (+0x1914) = tick + (live[29]*tempo)/700000 + 1 (32-bit imul, cdq, idiv 0xAAE60 at 0x1002ABBC; unshuffled tick), flam armed (+0x1910 = 1), bar line as the 808; same step without wrap: if armed and tick > flamTick (signed) disarm and run the flam pass, else return. Then the latch; accent = f32((AC*live[2])*0.03125f + 1.0) (0x1002ACAF..0x1002ACD1); columns 11 down to 1: normal pass 1|3 -> gain f32(acc*0.8) (dbl_10154358 0x3FE999999999999A), 2|4 -> f32(acc*2.0) (dbl_10154360), else skip; flam pass (0x1002B320) value > 2 -> f32(acc*1.5) (dbl_10154350). Gain stored at +0x191C, then the instrument (three identical 11-case switches 0x1002AD2D, 0x1002B03C, 0x1002B359).
- Triggers: sub_1002D8B4 (sample, level, decay, i): g = f32((((6.2e-5f*L)*L)*c)*gain), c 1.1 RS (flt_101545D4), 2.3 CP (E4), 0.8 CH (DC), 1.1 OH; m = DT[(d<<4)>>7] (RS, CP pass d = 127 -> DT[15] = 1), CH m = f32(DT^8) (fmul st,st x3 at 0x1002D939..0x1002D952) after sub_1001E3D0(OH, 0.996f 0x3F7EF9DB); CH/OH level = byte 20 (HH), decays bytes 23/24. sub_1002DA24 (sample, level, tune, decay, i): PITCHED, g = f32(((L*(L*6.2e-5f))*kv)*gain), kv 1.2 LT (flt_1015460C), 1.1 MT (608), 1.4 HT (604), 0.6 crash/ride (5FC, decay 127), m = DT[decay], hold 0, step = T909[tune]. sub_1002DB24 (level, decay, tune, attack): sample slot 25 + tune>>5 (bd1..4), word_1014E544 toggle as the 808 (fade 0.99), g = f32((((L*(L*k))*k)*gain)*(1 - C8[attack>>4])), m = DT[decay]. sub_1002DC54 (level, tone, tune, snappy): A = slot 25 + SD909[(tune*5)>>7], counter word_1014E548 = (c+1)&31, B = slot 25 + SDN[SEQ909[c]], g1 = f32(((L*(L*k))*k)*gain), g2 = f32(snappy*0.01f) (flt_10154648 0x3C23D70A), m 0.998f, hold tone + 30.
- Audio 0x1002B6F0: zero the bus, FILTER A, B, DUAL, PITCHED LT, MT, HT, SIMPLE RS, CP, CH, OH, PITCHED crash, ride.
- Slot 8 0x1002D480: tick-0 events of ctrl 0..29 with sub_10019624(+0x30) (RBS order). Slot 9 0x1002D510: 33-byte copies. Slot 6 0x1002DD40: slot 1 on every voice.
- Sample slots: n = resource 1000 + n (25..45 = IDs 1025..1045).

Corpus songs (scratchpad classify.py, tools/rbs.py: a 909 pattern holds a step within its length, no insert routed in the chunk or the mixer track): the 28 909_* songs, drums_kit, mixer_level_sweep_909, mixer_pan_sweep_909, shuffle_drums_g127, shuffle_drums_g64 (33; drums_kit and the shuffle songs also play the 808). Left for later tasks: pcf_on_909 (PCF), full_mix (everything).

First run: 31 of 33 byte-exact; 909_flam_interval0 and 909_flam_interval127 differed (interval 64, the ctor default, matched). Failed hypothesis: set-param takes controllers 0..28 only (Hex-Rays of 0x1002C6D0 shows cases 0..0x1C). The disassembly has cmp edx, 1Dh; ja, and the table entry 29 (0x101545A0) points to 0x1002C80B: mov [ecx+18AEh], al, the flam interval, which Hex-Rays drops (IDA files the label under another switch). With controller 29 taken, all 33 match.

Implementation (crates/rb-engine): src/tr909.rs (the device, on the task-024 voice classes; parameter blocks in the DLL's order with internal_index for 20..22), process.rs (next_snare_909, bd_909/set_bd_909), drums.rs (Simple::set_m, sub_1001E3D0), frame Devices::implemented builds the 909 with the samples. rb CLI: TR-909 is no longer in the not-implemented warning (crates/rb-cli/tests/rebirth_dll.rs checks that no warning names a drum machine for Trakka Rekka).
Result: all 33 909 corpus songs byte-identical to tests/refs (manifest wav_sha256) at idle_blocks 9088 with TestDelay. Control: 909_snare without the 909 differs. No rbhost dump and no test-machine permit were needed.
Without the delay: rb render (scratchpad cli_check.sh, --rebirth-dll original/ReBirth.dll) against the references: 909_snare differs by +-1 LSB on 4230 samples from frame 69, 909_flam_interval127 on 7400 from frame 159 (the delay's 1e-6 only, as for the 808 and 303 songs).
Unit tests (crates/rb-engine/src/tr909/tests.rs, 11, hand-computed): ctor state, the 20..22 permutation through msg 73, set-param (29 included) and slot 8, step values x0.8/x2.0 and value 5 silent with the accent 8.9375, descending order and the CH choke (OH m = 0.996f), CH m = DT^8, flam (flamTick 11 at interval 64 / 120 BPM, retrigger at tick 12 only, once, re-armed by the next step; interval 0 two ticks later), BD alternation and the tune>>5 sample, SEQ909 snare with the counter wrap, toms/crash kv and T909.
Tests: cargo test --release 148 pass, 1 ignored (task-049); clippy -D warnings and fmt clean.
Reproduce: cargo test --release --test corpus_tr909; cargo test -p rb-engine --lib tr909; bash scratchpad cli_check.sh.
Songs needing other devices (later tasks): pcf_on_909 (the PCF), full_mix (everything). Every song needs the engine's delay (task-026) to match without TestDelay.
Docs: initial_investigation.md 5 (flam: 32-bit, unshuffled tick, armed per step), 7.1 (909 audio order), 7.2 (909 step, fields, the 20..22 order, set-param case 29, slots), 7.3 (909 formulas); song_load.md 4 (808/909 msg-41 rule now [C]), 4.8 (the DLL's 20..22 order, controller 29), 9; export_state.md 7.3 (909 counter, flam state).
AC #1 evidence: crates/rb-engine/tests/corpus_tr909.rs tr909_songs_with_a_test_delay_match_their_references passes on all 33 songs listed above (the list asserts 33), with the samples of original/ReBirth.dll and TestDelay; tr909_songs_differ_without_the_909 shows the 909 is audible in them.

Review round 1 (send back): docs/initial_investigation.md:269, the lead flam bullet, still said int64, against the [C] 32-bit bullet below it; it now says 32-bit integers. No code change.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The TR-909 device (crates/rb-engine/src/tr909.rs) implemented from the disassembly on the task-024 voice classes: the step function 0x1002AAC0 with descending triggers, step values x0.8 / x2.0, the flam (flamTick = tick + (flam*tempo)/700000 + 1 in 32 bits on the unshuffled tick, armed by every new step, one x1.5 retrigger of values above 2), BD sample switching by tune>>5 and voice alternation (word_1014E544), the SD noise from SEQ909 (word_1014E548), toms/crash/ride on PITCHED voices with kv and T909, RS/CP/CH/OH with the CH decay DT^8 and the 0.996 choke, DT; slots 2..11 incl. init_track (slot 7 = 0x10026C40, slot 8 0x1002D480). Found: the DLL keeps parameter bytes 20..22 as HH, RS, CP (msg 73 and set-param permute them), and set-param takes controller 29, the flam interval (Hex-Rays drops that case). The 909 counters are in ProcessState; Devices::implemented builds the 909 with the samples; the rb CLI no longer warns that the TR-909 is not implemented.
Result: all 33 TR-909 corpus songs (the 28 909_* songs, drums_kit, mixer_level_sweep_909, mixer_pan_sweep_909, shuffle_drums_g64, shuffle_drums_g127; derived with tools/rbs.py) are byte-identical to tests/refs at --idle-blocks 9088 with TestDelay. pcf_on_909 (PCF) and full_mix wait for the effect tasks. Docs: initial_investigation.md 5, 7.1-7.3; song_load.md 4, 4.8, 9; export_state.md 7.3.
Tests: cargo test --release 148 pass, 1 ignored (task-049); clippy and fmt clean. Branch worktree-agent-a2cf6498ef145e173, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

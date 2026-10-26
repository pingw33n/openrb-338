---
id: TASK-023
title: TB-303 devices (2.0 and vintage sound)
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 19:23'
labels:
  - engine
milestone: m-1
dependencies:
  - TASK-022
priority: high
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The two TB-303 devices (docs/initial_investigation.md section 6; the sound-model selection in docs/song_load.md section 3): parameters, the step function (gate 24 ticks, slide tie, bar-line pattern latch, patStart), note-on/note-off with the ^64/^4 coefficients, glide, the pitch table, the ReBirth 2.0 sound engine 0x100248C4 (GLOB[482] = 0, global flag +21 zero; per 64-sample block, interpolated resonance rows, linear coefficient ramp, click-free retrigger; used by every shipped song) and the Vintage sound engine inline in 0x10023F10 (GLOB[482] = 1; control values every 4 samples), the oscillator, one-pole, 2-pole filter and DC blocker, with exact f32 store points. The survey first had these two engine labels swapped; task-003 corrected them in docs/initial_investigation.md sections 6.2 and 6.3. Verify the uncertain points against the disassembly (0x10023F10, 0x100248C4, 0x10023CD4, 0x10023C74, 0x100253A0) before relying on them. Tests: the single-303-note corpus songs match their references.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Both sound models are covered by corpus songs and tests
- [x] #2 Every TB-303 corpus song, with the delay disabled, matches rbhost byte for byte (the corpus references themselves include the enabled delay's output: task-049)
- [x] #3 With a test-only model of the delay traced from the disassembly, every TB-303 corpus song equals its corpus reference, so the remaining difference is the delay's (task-049)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE from the disassembly (idalib on a scratch copy, plus a capstone-based x87 tracer over original/ReBirth.dll for exact operand order and store points): CTB303Device vtable 0x10033FD4 slots 1..11 and 14, ctor 0x10023920, step 0x100253A0, note-on 0x10023CD4, note-off 0x10023C74, the Vintage loop inline in 0x10023F10, the 2.0 engine 0x100248C4 with its row interpolation helper 0x10023800, reset 0x10026CC0, silence 0x10025630, relocate 0x100255E0, start 0x100255B0, song mode 0x10026B60, data msg 39 0x10025CB0, setParam 0x10026180, ctor-time slots 7/8 (0x10026C40, 0x10025650).
2. crates/rb-engine/src/tb303.rs: the device (state at the DLL offsets, step/note-on/note-off, both sound engines, f32 store points as traced), implementing SongDevice + ChannelDevice; unit tests.
3. Frame/seq plumbing the 303 needs: the sound-model flag to the devices (msg 36 byte 21), trunc(pos) for slots 2 and 3, the ctor-time slots 7 (track quantisation of ctrl 0/1 = 32) and 8 (tick-0 events of the ctor parameters); a device set with the two 303s.
4. Corpus: render every diag song at N=9088, compare with the reference (sha256; rbcmp first differing sample on failure); trace differences. Derive the 303-only song list with tools/rbs.py (drums silent, effects off). Add a corpus test asserting the SHA-256 of each 303-only song.
5. docs/initial_investigation.md section 6 and docs/export_state.md 7.2: corrected field map and formulas with addresses.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a4cac5dfb27c7ac80

RE (idalib on a scratch copy of ida/ReBirth.dll.i64, read-only; plus a capstone-based symbolic x87 tracer over original/ReBirth.dll, scratch x87trace.py, which turns a straight-line range into SSA lines with every f32 store marked and register-form arithmetic decoded from the opcode bytes per the Intel SDM):
- CTB303Device vtable 0x10033FD4: slot1 step 0x100253A0, slot2 relocate 0x100255E0, slot3 start 0x100255B0, slot4 0x100106C0 (-> slot 6), slot5 reset 0x10026CC0, slot6 silence 0x10025630, slot7 0x10026C40, slot8 0x10025650, slot9 song mode 0x10026B60, slot10 host msg 0x100256E0 (msg 39 at 0x10025CB0), slot11 setParam 0x10026180, slot12 MIDI 0x100261F0, slot13 panel update 0x10026700, slot14 audio 0x10023F10.
- Field map (corrects docs/export_state.md 7.2, which called +0xCA8 the amp): +4 patStart, +0x30 pattern-mode params[12], +0x3C patterns 32x98, +0xC7C live params[12] (byte 9 = +0xC85 the last step), +0xC88..+0xC93 panel "changed" flags, +0xC94 playing pattern, +0xC95 latched enable, +0xC96 wrap/play flag, +0xC98 gate-off position (-1 none), +0xC9C last pattern position, +0xCA0 retrigger flag, +0xCA1 slide flag, +0xCA4 phase, +0xCA8 envelope E, +0xCAC envelope decay, +0xCB0/+0xCB4 target/gliding pitch, +0xCB8 accent amount, +0xCBC accent envelope, +0xCC0/+0xCC4 cutoff smoother, +0xCC8/+0xCCC amp input/feedback, +0xCD0 gate (0/1 f32), +0xCD4 amp, +0xCDC/+0xCE0 y1/y2, +0xCE4 pre-filter s, +0xCE8 DC blocker o, +0xCEC accent feedback, +0xCF4..+0xD00 2.0 coefficients (c0, c1, c2*G, c3). +0xCD8, +0xCF0 are zeroed by ctor/reset and never read.
- Step 0x100253A0: v = (u32)(t' - base) % (48*len) with t' = shuffled if pattern[0] else tick, base = patStart in song mode else 0; wrap when lastPos - v > 24 (signed) -> pending gate-off fires, +0xC96 = 1; gate-off pending: enable latch 0 clears the slide flag, v >= gateOff -> note-off; else a new step (step != +0xC85 or wrapped) triggers: gateOff = v + 24, at (tick/48)&15 == 0 (UNshuffled tick, C remainder) the bar line latches the live pattern (patStart = 768*(tick/768)) and the enable; then note-on(step) or note-off; +0xC85 = step. The step index comes from the old pattern's length.
- Note-on 0x10023CD4: accCoef = f32(f32((reso*(1/12700)+0.99)*0.999)^(2^n)), n = 6 (2.0) / 2 (Vintage), by fmul st,st from the stored f32; decay = f32(0.99999 - (1 - decay*f32(1/127))^2*0.0003) (no accent) or 0x3F7FEBAF (accent, accAmt = f32(accent*3/127)), squared the same way. Rest steps also update the coefficients, the pitch and the slide flag; a played step sets gate 1.0, amp in/fb (2.0: 1.0/0; Vintage 0.1/0.9) and, unless the previous step slid, E = 1.0 and the retrigger flag. pitchTarget = ((tune<<6) + ((i16)(note+12up-12down)<<6) + 0xC00) << 4.
- Note-off 0x10023C74: skipped while +0xCA1; gate 0, in/fb 0.063/0.937 (2.0) or 0.1/0.9 (Vintage).
- 2.0 engine 0x100248C4 traced: once per block cs = (A*cs1 + cut*g) - b2*cs2; amp1 = (amp+1e-6)*fb + gate*in; E0 = 1e-6 + E; acc = coef*accEnv + E0*((1-coef)*accAmt), accEnv = f32(acc); G = amp1*(acc+1); kA = 0.10166.. - (reso*f64 1/127)*(1/15); if retrig: start coefficients at C(E0) with c2*G = 0; targets at C(E0*decay); C = (kA*f32(acc)) + (((E-0.4)*em) + cs), clamped, rounded to f32; coefficients from sub_10023800 (rows l = trunc(reso*f32 0.0078732288*7) and l+1, f32(lo + (hi-lo)*frac)); deltas (target - start)/32 (c2 target times G), stepped after every sample pair. Vintage inline loop traced: per 4 samples E' = E*decay, acc from the old E, amp step, C = ((E'-0.4)*em + cs) + acc*kA (acc unrounded), G = (acc+1)*amp, FILT row reso>>4, index trunc(C*1023).
- Shared sample: ph += inc, > 1 -> -2; x = ph or (ph <= 0.0457763671875 ? -0.54931640625 : 0.54931640625); s = s*c3 + x; y = (s*g + y1*c0) - y2*c1; o = (y - y1) + o*0.985853374; out f32(o). All f64 inside the block, f32 stores at the block end only (and accEnv every control step).
- Slots 7/8 from the song ctor (0x10011941, 0x10011985 with tick 0): the 303's slot 7 sets its track's quantisation of controllers 0 and 1 to 32 units (sub_10021560 -> sub_10014600: list+0xD+ctrl), slot 8 inserts tick-0 events of controllers 0..8 with the ctor parameters (sub_1001F900 -> sub_10014750). Ctor params (sub_10018C20): 1,0,64,20,90,20,100,64,0.
- Slot 2 and slot 3 ignore their argument and read trunc(pos) (sub_1001F870 at 0x100255E9, 0x100255B9): the SongDevice trait now passes it.

Corpus, first render (Devices with the two 303s, everything else placeholder): 0 of the 24 303 songs exact, all within +-1 LSB, always ours = ref - 1 (1300 samples per 2.0 song, 600 per Vintage song). Failed hypothesis 1: the 303 state differs at export start. rbhost --dump-state on 303_saw_lowcut: both 303s' fields equal ours bit for bit at export start (phase be2e1c20, s c2b623ba/c2142f49, cs1, coefficients, amp/env 0, in/fb 0: the untitled-song activation leaves the ctor's amp constants, so the idle 303 output is exactly 0). Failed hypothesis 2: the drum placeholder's guard (1e-6) is too small: the real idle 808/909 zero their bus (0x100280BD, 0x1002B6FD) and inactive voices skip (active byte tests 0x1001E4D7, 0x1001E707, 0x1001E9A7); the dump has the drum channels' peaks 0.0 where ours had 1e-6; zero-filled drums made it worse (3500 samples off). Cause found with a new rbhost option --dump-float (the export's float output before int16, unk_100E012C/unk_100F012C): at frame 0, before any note, ref = 1.2676785e-6 = 2e-6*master, ours 1.0355913e-6 = (1e-6 + 2*1e-6*0.3169)*master. The extra 1e-6 is the delay: every diag song has the delay enabled (template: enabled=1, steps 3, feedback 0), and an enabled delay reads the guard-filled send bus and adds its output into the master (0x1000F940 enabled path); only the disabled path (0x1000FAD1) leaves L/R alone. So every corpus reference carries the delay's ~1e-6 DC, and no song can match before the delay (task-026) exists.

Check without the delay: scratch nodelay.py writes each song with the delay disabled (chunk byte and the delay track's tick-0 'enabled' event), rbhost (this branch, --idle-blocks 9088) renders them, the engine renders them with the drum buses zero: all 24 303_* songs (17 2.0, 7 Vintage) and the 12 other songs with silent drums and no inserts (delay_303, loop_bar1, loop_bar2, loop_events_inside, mixer_level_sweep_303, mixer_master_low, mixer_pan_sweep_303, shuffle_303_g0, shuffle_303_g127, shuffle_303_g64, shuffle_flag_off, silence) are byte-identical (cmp of the whole WAV): 36 of 36.
Song classification (scratch classify.py over tests/songs/diag/songs with tools/rbs.py): drums silent (no step set in any 808/909 pattern) and no insert routed (mixer comp/pcf id 0, no channel dist, in chunk and track): the 36 above; all 145 have the delay enabled.

Implementation: crates/rb-engine/src/tb303.rs (the device, both sound engines) + tb303/tests.rs (14 unit tests); SongDevice gains relocate(tick, now) / start(now) (now = trunc(pos)), set_sound_model (msg 36 byte 21, Engine::set_globals) and init_track (slots 7/8, run by Engine::new on every device with its track); Track::set_quant; Devices::implemented() (two Tb303s, placeholders for the rest). The Silent placeholder now outputs 0, what an idle 808/909 outputs (was the 1e-6 guard; with the guard the delay-off renders differ). corpus_frame's silence test still passes.
rbhost: new option --dump-float DIR (tools/rbhost/rbhost.c, run.py --dump-float, README): writes the export's float output (unk_100E012C/unk_100F012C after every chunk) as f32 L,R pairs; the render path is unchanged (303_saw_lowcut and 303_vintage_saw rendered with it give the reference sha256 58a4f47e... and 77e0b263...). rbhost.exe sha256 with it: f199ffad610e.
Corpus test: crates/rb-engine/tests/corpus_tb303.rs. tb303_songs_without_the_delay_match_rbhost renders the 36 songs with the delay disabled in the engine model (delay params[0] and the delay track's ctrl-0 events set to 0, the same edit nodelay.py made on the .rbs) and asserts each WAV sha256 against crates/rb-engine/tests/fixtures/tb303_nodelay.json (the rbhost renders of the variants): passes, 36 of 36, 7 of them Vintage. tb303_songs_match_their_references (the same songs against tests/refs/manifest.json) is #[ignore]d: 35 differ (all but silence) by the missing delay DC; task-049 (depends on task-026) removes the ignore.
Split: AC #1 as written (the corpus references) needs the delay; it moves to task-049. This task's AC #1 is restated as the delay-off check, which the code meets.
Not checked: the drums' slots 7/8 (they may also insert tick-0 events; for task-024/025); the 303's MIDI slot 12 (0x100261F0) and panel slot 13 (out of scope). The +0xC88 changed-flags are not modelled (panel only).
Reproduce: cargo test --release -p rb-engine --test corpus_tb303; the variants: scratch nodelay.py OUT tests/songs/diag/songs/NAME.rbs, then tools/rbhost/run.py --idle-blocks 9088 -o REF OUT/*.rbs.
Tests: cargo test 93 pass, 1 ignored (release and debug); cargo clippy --all-targets -D warnings clean; tools unittest (tools, tools/rbhost) OK.

Review round 1 (send back): (1) the residual was not shown to be the delay; (2) wrong numbers; (3) no committed fixture generator, rbhost build not reproducible; (4) drum slots 7/8 to be carried to task-024.
(1) Traced the enabled delay: update 0x1000F754 (every 16 blocks, counter +0x30 & 15 == 0 before the increment; the +0x48/+0x50 averaging is overwritten by +0x4C = tempo from sub_1001FCF0), q = mode==0 ? 3T/100 : 4T/100, D = min(26460000*steps/q, 0x1FFFF), targets f32(1e-6f + PAN[127][pan]), fb = f32(fb*(1/127)); per block g = g*0.9 + t*0.1 stored f32, the loop uses the unrounded value; per sample x = buf[(w-D)&0x1FFFF], L = f32(x*gL + L), R = f32(R + x*gR), buf[w] = f32(x*fb + send[i]), send[i] = 1e-6. rbhost --dump-state at export start of 303_saw_lowcut: D 16537 (120 BPM, steps 3, 16ths: 79380000/4800), w 57344, counter 9088, gains f32 1.0000005, targets 1.000001 (the ~16k boundary the reviewer found is D). A test-only model of exactly that (TestDelay in crates/rb-engine/tests/corpus_tb303.rs, buffer filled with 1e-6 by the activation's slot 6) in place of the placeholder: all 36 TB-303-only songs equal their corpus references (whole-WAV sha256 against tests/refs/manifest.json), test tb303_songs_with_a_test_delay_match_their_references. So the whole difference is the delay; the 303 has no error. Command: cargo test --release -p rb-engine --test corpus_tb303. The split stays; task-049 replaces TestDelay by the engine's delay.
(2) Corrected numbers (the frame-0 figure predated the Silent change): engine without a delay at frame 0 = 6.338391e-7 = 1e-6*master (ref 1.2676785e-6 = 2e-6*master); flips: 303_saw_lowcut 3516 frames (7032 samples), 303_vintage_saw 1656 frames (3312 samples), all -1 LSB (the 1300/600 of the first notes were with the old 1e-6 drum placeholder). Fixed in docs/initial_investigation.md 8.4, task-049, the summary and the commit message.
(3) tools/nodelay.py (names the two bytes: DELY byte 0, the delay track's ctrl-0 events); its output equals the scratch variants byte for byte (36/36). tools/rbhost/build.sh links with -Wl,--no-insert-timestamp: two builds now give the same exe, sha256 cc7994f1e622 (before: differed at the PE timestamp, bytes 129 and 19461). The fixture was re-captured with that binary (tools/rbhost/run.py --idle-blocks 9088 on the nodelay.py output): same 36 hashes; the fixture records cc7994f1e622 and the earlier build f199ffad610e.
(4) The drums' slots 7/8 are noted in task-049's description for task-024/025.
Tests: cargo test 94 pass, 1 ignored; clippy clean; tools unittests OK.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The two TB-303s are implemented from the disassembly (crates/rb-engine/src/tb303.rs): step function, note-on/off, glide, the ReBirth 2.0 engine 0x100248C4 with its resonance-row interpolation and coefficient ramp, and the Vintage engine inline in 0x10023F10, with the original's operand order and f32 store points (traced with a symbolic x87 tracer over the DLL bytes). The frame and sequencer pass what the 303 reads: trunc(pos) for slots 2/3, the sound-model byte, and the song ctor's slots 7/8 (bar quantisation of controllers 0/1 and tick-0 events of the ctor parameters). The drum placeholder outputs 0 like an idle 808/909.

Result: the 36 diagnostic songs whose sound is TB-303 and mixer only (the 24 303_* songs, 17 ReBirth 2.0 and 7 Vintage, plus delay_303, loop_bar1/2, loop_events_inside, mixer_level_sweep_303, mixer_master_low, mixer_pan_sweep_303, shuffle_303_g0/g64/g127, shuffle_flag_off, silence) are byte-exact two ways: with the delay disabled against rbhost (fixture from tools/nodelay.py and the now reproducible rbhost build cc7994f1e622), and as they are against their corpus references with a test-only model of the delay traced from 0x1000F754/0x1000F940. With the placeholder delay, 35 of 36 still differ from the references (303_saw_lowcut 3516 frames, 303_vintage_saw 1656 frames, all -1 LSB): every diag song has the delay enabled, which adds 1e-6 plus 1e-6 x the channels' signal D = 16537 samples later. The check with the engine's own delay is task-049 (after task-026); the original AC #1 moved there. Not checked here: the drums' slots 7/8 (noted in task-049 for task-024/025).

Facts added to docs/initial_investigation.md (5 track quantisation, 6.3 the 2.0 block, new 6.4 fields/step/slots, 7.1 idle drums output 0, 8.4 the enabled delay's contribution and its traced update and loop) and docs/export_state.md 7.2 (corrected field labels; the idle 303 output is exactly 0 until the export's first note-off under the corpus protocol; the msg-53 untitled activation does not call slot 6 on the new song). rbhost gained --dump-float and a reproducible build.

Branch worktree-agent-a4cac5dfb27c7ac80, one commit (amended in review round 1). Tests: cargo test 94 pass, 1 ignored (the corpus-reference test with the engine's devices, for task-049); clippy clean; Python tool tests OK.
<!-- SECTION:FINAL_SUMMARY:END -->

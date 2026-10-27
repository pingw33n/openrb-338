---
id: TASK-069
title: >-
  Bug: the PCF's wave step after returning to pattern mode while playing differs
  from the DLL
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 13:05'
updated_date: '2026-10-27 13:59'
labels:
  - engine
  - re
milestone: m-3
dependencies:
  - TASK-055
references:
  - crates/rb-engine/src/pcf.rs
  - crates/rb-player/tests/live_messages.rs
  - docs/song_load.md
priority: medium
type: bug
ordinal: 95000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-055 drove the original ReBirth.dll (rbhost --live, tools/rbhost) and rb-player with the same live session over the corpus song full_mix and found one difference, not in any panel message: with the PCF's wave set to a stepping pattern in pattern mode (controller 4 = 30, by msg 41 sub 114 or by a whole data block, sub 116, alike), playing, then song mode (msg 35 {0,1}) at call 300 and back to pattern mode ({0,0}) at call 400, the output differs from call 400 on (first frame 409602, 449 vs 448; 1024-frame calls, idle 0). Without the two mode switches, or with the song's own wave, the outputs are byte-identical. So rb's PCF step position (pcf::Pcf::control, quotient(ctx.tick, res), ctx.tick = trunc(pos) of sub_1001F870) or the sequencer's position after a return to pattern mode (seq set_song_mode, slot 2 relocate) does not follow the DLL there.

Reproduce: cargo test --release -p rb-player --test live_messages -- --ignored (the test pcf_wave_after_returning_to_pattern_mode holds rbhost's sha256 of that session, 4a50f82aacd9b7eaf98965197b93ad0a86437362e919484ae1c7f0a2a256172c); RB_LIVE_OUT=DIR writes DIR/<name>.rblv and .wav for rbhost.exe --live (README of tools/rbhost).

Do: find where the DLL's PCF step (0x10016756, sub_1001F870) and rb's diverge after msg 35 {0,0} while playing (dump the PCF's +0x80.. state, or rbhost --dump-float around call 400), fix it in rb-engine, and un-ignore the test. Check the corpus golden test stays green.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 pcf_wave_after_returning_to_pattern_mode passes (not ignored)
- [x] #2 cargo test --release passes, corpus golden included
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. ChannelDevice::stack_word(): the dword the device's audio slot leaves at its entry esp-0x1C: 808 0x10028278, 909 0x1002B8B4, 303 None (not established; their slot holds leftovers of earlier code).
2. BlockContext.stack_word, set per channel in Frame::block; the PCF block uses it as the stale var_1C for a resolution other than 8/16/32, falling back to its last quotient when None ([G], unchanged).
3. Unit tests (pcf wave 64 step after a 909 channel), un-ignore the live test.
4. Docs: initial_investigation 8.2, original_bugs OB-7 (now [C] for the 808/909 channels), pcf.rs docs.
5. cargo test --release, golden, rb compare 164/164; follow-up task for the 303 channels.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a21e83dfb758d90ca

Evidence (2026-10-27):
- full_mix's PCF wave is 64 (song data and track): PCF_DATA gives wave 64 len 16, res 0 (stale quotient, OB-7), all 16 steps 0; wave 30 len 16 res 16, steps 35,0,0,53,65,0,0,93,0,43,0,18,58,0,65,0. PCF routed to channel 4 (pcf_id 4, the 909).
- So in song mode (calls 300..399) the live wave is 64 again, and the step +0x74 becomes (stale var_1C) % 16. rb took the last quotient this PCF computed (wave 30 at call 299), the DLL something else; back in pattern mode at 400 the 'new step' test (0x10016828 cmp edx, ebp) and so the trigger differ.
- Measured with a scratch rbhost (not committed): a naked thunk patched into CPCFInsert vtable slot 15 (0x10033AD4+0x3C) logs [esp0-0x1C] (= var_1C, esp+0x48 after the 0x64-byte prologue) at every PCF block entry, plus the PCF object after each call; then a hardware write watchpoint (DR0 on that stack address, VEH) logs every writer's EIP. rbhost's WAV hash with the thunk stays 4a50f82a... (unchanged).
- Result: var_1C at PCF entry is 0x1002B8B4 in every block of the session (calls 0..599), and +0x74 is 4 = 0x1002B8B4 % 16 whenever the wave is 64 (calls 0..105, 300..399). The DLL's PCF does write q there (0x100167A4 etc.) but the next block rewrites it before the PCF runs.
- The writers per block: sub_1001EF20 (seq tick, 0x1001F3B7, tick blocks only), the 808 audio 0x100280B0's voice calls (return addresses 0x1002810D..0x10028278), the 909 audio 0x1002B6F0's voice calls (0x1002B74D..0x1002B8B4), the PCF (q), the delay 0x1000F940 (0x1000FA9E, 64 per block), the mixer update 0x10012C14 (0x10012C3E). The 303s and the distortion never write it.
- Static check: 0x1002B6F0 ends with an unconditional call sub_1001E9A0 at 0x1002B8AF made at esp0-0x18 (4 pushes, push esi 0x1002B6F8, push edi 0x1002B8A5), so its return address 0x1002B8B4 lands at esp0-0x1C; 0x100280B0 the same with call sub_1001E4D0 at 0x10028273 (return 0x10028278). Device slot 14 is called with one arg from the same esp as the inserts' slot 15 (0x10013580/0x1001359B, 0x100135C3/D9/EF), so the PCF's esp0 equals the device's. The distortion 0x10010A50 (3 pushes, sub esp 30h) has no local at esp0-0x1C and calls only below its esp: it leaves the slot. Failed hypothesis: the mode-switch copies (slot 9, 0x10017290) or sequencer relocation (sub_1001FBE0) - both match rb.

Cause: at call 400 the DLL's PCF had +0x74 = 4 (0x1002B8B4 % 16, from the song-mode wave 64) and wave 30's first step back was 4 (tick ~966, /48 = 20, % 16 = 4): no new step, no trigger. rb had +0x74 = 12 (its stand-in, the last wave-30 quotient at call 299: 60 % 16) and triggered velocity 65 there; first difference frame 409602.
Fix: ChannelDevice::stack_word() (tr808 0x10028278, tr909 0x1002B8B4, 303 None) and BlockContext::stack_word set per channel in Frame::block before the channel's inserts; Pcf::control uses it as the quotient for a resolution other than 8/16/32, else keeps its last quotient (303 channels, [G], follow-up task-072).
Results: live_messages pcf_wave_after_returning_to_pattern_mode now has rbhost's hash 4a50f82a... (before ce9fdb76...), un-ignored; the_session_equals_rbhost unchanged. cargo test --release (RB_STANDARD_MOD=target/Standard.rbm): 331 pass, 0 fail, 0 ignored. rb compare --manifest tests/refs/manifest.json --standard-mod target/Standard.rbm: 164 of 164 equal. clippy -D warnings clean.
Reproduce the measurement: scratch copy of tools/rbhost/rbhost.c with the thunk/watch (not committed, a debugging aid), built with build.sh's flags, run as rbhost.exe --live pcf.rblv full_mix.rbhm out.wav under one redc-win-run permit in C:\re\rb\task069 (deleted after); RB_LIVE_OUT=DIR cargo test --release -p rb-player --test live_messages writes pcf.rblv.

Review round 1 (send back): (1) the stale words are return addresses valid at the preferred base 0x10000000, moving with the load address; said so in OB-7, initial_investigation 8.2 (and re-wrapped the over-long line there), pcf.rs, tr808.rs, tr909.rs. (2) task-072: type task, priority low and references pcf.rs, tr808.rs, tr909.rs, original_bugs.md set via the CLI (type, priority and two refs were already in the worktree's file; the 808/909 refs added). Commit amended.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Fixed. The difference was OB-7, the PCF's stale stack quotient: full_mix's own PCF wave is 64 (resolution 0), so in song mode the step came from the uninitialised slot var_1C. Measured in the DLL (rbhost --live with a vtable thunk and a hardware write watchpoint): on the 909's channel the slot holds 0x1002B8B4 at every PCF block, the return address of the 909 audio's last call (0x1002B8AF); on the 808's channel it is 0x10028278 (0x10028273). rb used its last quotient, so back on wave 30 at call 400 it saw a new step and triggered where the DLL did not. rb-engine now passes the channel's word to the PCF (ChannelDevice::stack_word, BlockContext::stack_word); the 303 channels keep the [G] stand-in (follow-up task-072). docs: initial_investigation 8.2, OB-7 (now [C] for 808/909). Tests: the live test passes un-ignored with rbhost's hash; cargo test --release 331 pass; corpus compare 164/164 equal; golden green. Branch worktree-agent-a21e83dfb758d90ca, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

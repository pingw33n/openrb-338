---
id: TASK-069
title: >-
  Bug: the PCF's wave step after returning to pattern mode while playing differs
  from the DLL
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-27 13:05'
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
- [ ] #1 pcf_wave_after_returning_to_pattern_mode passes (not ignored)
- [ ] #2 cargo test --release passes, corpus golden included
<!-- AC:END -->

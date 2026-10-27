---
id: TASK-055
title: 'RE + engine: the panel''s live device messages (msg 41 sub ids)'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-27 07:34'
labels:
  - re
  - engine
milestone: m-3
dependencies:
  - TASK-043
references:
  - docs/ui_spec.md
  - docs/song_load.md
  - crates/rb-player/src/lib.rs
priority: high
type: feature
ordinal: 50000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the panel (docs/ui_spec.md) changes the engine with device messages (song msg 41 with a device's own sub ids), not only with the controllers of slot 11 and the msg-41 data block. rb models only those two (Engine::set_param, Engine::device_data) plus the mixer's master and mute (MixerDevice::set_master, set_enabled). The real-time player (task-043, crates/rb-player) exposes Command::Param, Master, ChannelEnabled and DeviceData; a pattern edit can be sent only as a whole data block, which in pattern mode also resets the device's live parameters to the block's. task-045 (pattern editing) needs the original's edit messages.

What is already read (task-043 notes, headless ReBirth.dll.i64):
- TB-303 handler sub_100256E0, jump table flt_10153BD8 + 4*sub (subs 31..46, entries from 0x10153C54). Sub 40 0x10025CFC {ctrl, v}: ctrl 0..8 store the live byte +0xC7C+ctrl, then the record hook sub_10012BC0 -> sub_10021340 (acts only with song mode and record on), ctrl 9 nothing, ctrl 10 stores +0xC86; then clears the changed flag +0xC88+ctrl. Sub 45 0x10025FC4 {pattern, step, field 0..5, v}: byte +0x3E + 98*pattern + 6*step + field. Sub 42 0x10025F25 {pattern, 0|1, v}: +0x3C or +0x3D + 98*pattern (the pattern's first two bytes). Not read: subs 43 (sub_100191E0), 44 (sub_10018DC0), 46 (sub_10019B00), and the queries 31..38.
- Mixer handler sub_10015520, table at 0x10151ECC + 4*sub (subs 137..148 from 0x101520F0): 144 the block into the pattern copy only; 145 {id, v}: 0 master, 1 comp id, 2 PCF id; 147 {c, k, v}: k 0 enabled, 1..4 level, pan, send, dist (the record hook on 5+6c+k).
- To read: the TR-808 (data sub 56, handler around 0x1002884C), TR-909 (73), delay (86, 0x1000FB40), distortion (94), PCF (116), compressor (104) handlers; the exe's senders (ui_spec.md: sub_43EC83 303 knobs, sub_43ED1D 303 steps, sub_441C82 808, and the 909, effect and pattern-section polls).

Do: list every device message the panel sends (exe side) with its DLL handler and effect, with addresses, in docs/song_load.md (a new section on the live messages) or a new docs file; implement each in rb-engine as an Engine entry point (numerics untouched: these are byte stores); add the matching rb_player::Command variants; unit tests per message. Messages that only query or reply need no command.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every device message the panel sends (docs/ui_spec.md) is listed with its exe sender and DLL handler, with addresses, in docs/
- [ ] #2 Each message that changes engine state has an rb-engine entry point and an rb_player::Command, with unit tests
- [ ] #3 cargo test --release passes, including the corpus golden test and crates/rb-player/tests/no_alloc.rs
<!-- AC:END -->

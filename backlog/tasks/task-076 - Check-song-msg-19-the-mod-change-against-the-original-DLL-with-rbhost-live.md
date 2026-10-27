---
id: TASK-076
title: Check song msg 19 (the mod change) against the original DLL with rbhost --live
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-27 14:33'
labels:
  - tests
  - rbm
  - engine
milestone: m-3
dependencies:
  - TASK-056
references:
  - docs/rbm.md
  - tools/rbhost/README.md
  - crates/rb-player/tests/live_messages.rs
  - crates/rb-player/tests/mod_change.rs
priority: medium
type: task
ordinal: 100000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-056 modelled the mod change from the disassembly (docs/rbm.md section 14: song msg 19 {u16 BE id} is forwarded to the idle loop as top-level 78 {slot, 19, id_hi, id_lo}; sub_10011D84 runs slot 6 on the 808 and the 909, then replaces their sample table +0x18D8 / +0x18DC) and checked it only with rb's own tests. It was never run against the DLL.
Do: extend the live-session check of task-055 (crates/rb-player/tests/live_messages.rs writes an RBLV file of msg-78 records; tools/rbhost rbhost.exe --live SESSION.rblv SONG.rbhm OUT.wav renders it in the original DLL; RB_LIVE_OUT=DIR compares) with a mod-change record mid-song: a drum song playing with one mod loaded (msg 55 via rbhost --mod or a second loaded mod), then at call K a 78 {slot, 19, id} record switching to another loaded mod (both must be loaded with msg 55 first, the built-in kit is id 411 via msg 56), compared byte for byte with rb-player sending Command::Mod at the same call. Use recrypted mods (RB_MOD_DIR) and the standard mod; rbhost runs take one redc-win-run permit (doc-001). Add rbhost support for loading two mods if it lacks it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 an rbhost --live render with a msg-19 record mid-song equals rb-player's render with Command::Mod at the same call, byte for byte, for at least one 808 and one 909 song
- [ ] #2 the comparison is a test (env-gated like live_messages.rs) and the result is recorded in docs/rbm.md section 14
<!-- AC:END -->

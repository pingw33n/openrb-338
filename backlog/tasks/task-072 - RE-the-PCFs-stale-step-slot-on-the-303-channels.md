---
id: TASK-072
title: 'RE: the PCF''s stale step slot on the 303 channels'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-27 13:49'
updated_date: '2026-10-27 13:59'
labels:
  - re
  - engine
milestone: m-3
dependencies:
  - TASK-069
references:
  - crates/rb-engine/src/pcf.rs
  - crates/rb-engine/src/tr808.rs
  - crates/rb-engine/src/tr909.rs
  - docs/original_bugs.md
priority: low
type: task
ordinal: 97000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-069 found that the PCF block (0x100166D0) with a wave whose resolution is not 8, 16 or 32 (waves 55..127, OB-7 in docs/original_bugs.md) takes its step from the uninitialised stack slot var_1C (entry esp-0x1C), and that between PCF blocks the code before it at the same stack depth rewrites that slot. On the 808 and 909 channels it is the channel's audio slot (return addresses 0x10028278, 0x1002B8B4; rb: ChannelDevice::stack_word). The 303 audio (0x10023F10 / 0x100248C4) did not write the slot in the measured session, so with the PCF on a 303 channel (pcf_id 1 or 2) the slot holds older stack contents: in task-069's watch log the other writers per block were the sequencer clock sub_1001EF20 (0x1001F3B7, tick blocks only), the delay 0x1000F940 (0x1000FA9E, 64 writes per block, pointers like 0x100F012C + 0x100*k + 0xFC), the mixer update 0x10012C14 (0x10012C3E), the 808/909 audio, and code outside the block loop (the pump's 0-frame calls, the start of each render call). rb keeps the PCF's last quotient there, a [G] stand-in.

Do: confirm statically that the 303 audio never writes esp0-0x1C (both sound models), then measure var_1C at PCF entry with the PCF on channel 1 and 2 (rbhost --live with a patched vtable thunk and a DR0 write watchpoint, as in task-069's notes: the thunk is a naked function that reads [esp-0x1C] and jumps to 0x100166D0, the watch a VEH that sets Dr0/Dr7 from a RaiseException), in the live path and in the export path (2048-frame chunks), and model it in rb-engine (BlockContext::stack_word) if it is deterministic; otherwise document it as not reproducible. Only matters for a junk wave with a trigger, or a junk wave followed by a real one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The value of var_1C at the PCF's entry on the 303 channels is documented with addresses in docs/initial_investigation.md 8.2 and OB-7
- [ ] #2 rb reproduces it or OB-7 says why it cannot
- [ ] #3 cargo test --release and the corpus compare stay green
<!-- AC:END -->

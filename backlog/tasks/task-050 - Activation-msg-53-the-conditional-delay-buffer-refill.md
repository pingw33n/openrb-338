---
id: TASK-050
title: 'Activation (msg 53): the conditional delay-buffer refill'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 21:40'
labels:
  - engine
  - re
  - docs
milestone: m-1
dependencies:
  - TASK-026
references:
  - docs/export_state.md
priority: low
type: task
ordinal: 29500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From task-026's review (2026-10-26). In the original, msg 53 (song activation, sub_1001BDA4) refills the process-global delay buffer through the old song's devices' slot 6 only conditionally: when replacing a current song, slot 6 is called at 0x1001BDD0 only if the byte at sub_10013DA0()+1 is zero (test at 0x1001BDC7..0x1001BDCC); otherwise the routine returns at 0x1001BE03 without the refill. With no current song, 0x1001BDED..0x1001BDF8 calls sub_10013D40 on the new song, which runs slot 6 on each device (0x10013D47..0x10013D6A). rb's Engine::activate (crates/rb-engine/src/frame/mod.rs, the ReplacesCurrent path) and the test msg53_replacing_a_song_refills_the_delay_line assume the refill is unconditional. The corpus captures take the zero branch, so no render changes today.

Work: identify what the byte at sub_10013DA0()+1 is (which object sub_10013DA0 returns and what sets that byte; e.g. the transport's play/record state), model the condition in Engine::activate if the engine has that state (otherwise expose it as a parameter with the corpus default) and add a test for both branches; record the fact with addresses and [C]/[G] in docs/export_state.md (the activation section) and fix the comment in frame/mod.rs. The corpus tests must stay exact.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The condition at 0x1001BDC7..0x1001BDCC is identified and documented in docs/export_state.md with addresses and [C]/[G]
- [ ] #2 Engine::activate models it (or takes it as a parameter with the corpus default), with a test for both branches; the corpus tests stay exact
<!-- AC:END -->

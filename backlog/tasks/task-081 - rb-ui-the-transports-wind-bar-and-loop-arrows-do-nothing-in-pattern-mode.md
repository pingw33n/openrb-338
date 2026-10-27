---
id: TASK-081
title: 'rb-ui: the transport''s wind, bar and loop arrows do nothing in pattern mode'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-27 16:29'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - docs/ui_spec.md
  - crates/rb-ui/src/model.rs
priority: low
type: bug
ordinal: 103000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found by task-057 in the exe's transport poll sub_42A058 (IDA Rebirth.exe.i64): the whole block of the Rewind (0x42A091), Fast forward (0x42A0E8), Bar up (0x42A13E), Bar down (0x42A171), Record (0x42A1C7), Loop start up/down (0x42A1FC, 0x42A22F) and Loop length up/down (0x42A261, 0x42A294) presses is inside 'if (this+44)' (0x42A074), the transport's song-mode byte: in pattern mode those buttons do nothing (no locate, no record-off message). docs/ui_spec.md section 11 says the same for Rewind/Fast forward and Record. task-057 gated Record in crates/rb-ui/src/model.rs (press_transport); Panel::arrow still winds and moves the bar and loop in pattern mode and sends their msg 35 (and the numpad keys of docs/ui_spec.md section 10 reach the same code). Gate them on transport.song_mode; check the tempo arrows and the loop switch stay active (they are outside the block).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 In pattern mode, Rewind, Fast forward, Bar up/down, Loop start up/down and Loop length up/down (buttons and their numpad keys) send no message and change nothing, as in the original
- [ ] #2 crates/rb-ui/tests (wired.rs, keyboard.rs) check it; cargo test passes
<!-- AC:END -->

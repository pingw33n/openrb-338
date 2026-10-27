---
id: TASK-081
title: 'rb-ui: the transport''s wind, bar and loop arrows do nothing in pattern mode'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 16:29'
updated_date: '2026-10-27 16:57'
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
- [x] #1 In pattern mode, Rewind, Fast forward, Bar up/down, Loop start up/down and Loop length up/down (buttons and their numpad keys) send no message and change nothing, as in the original
- [x] #2 crates/rb-ui/tests (wired.rs, keyboard.rs) check it; cargo test passes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Gate Panel::arrow (the one path of mouse buttons, held-repeat and numpad keys) on transport.song_mode for rewind, ff, bar_*, loop_start_*, loop_length_*; tempo arrows and loop switch stay. Verify the gate in Rebirth.exe (0x42A074 test of this+0x2c, je 0x42A2B5 skips the block; tempo/loop poll after it). Tests in wired.rs and keyboard.rs; docs ui_spec s11 / initial_investigation 5.3.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a99e745323b6dd288

Verified in original/Rebirth.exe (llvm-objdump): sub_42A058 tests this+0x2c at 0x42A074 and 'je 0x42A2B5' skips the whole block; Rewind 0x42A091, FF 0x42A0E8, ... Loop length down 0x42A294 are inside; 0x42A2B5 starts the loop switch (msg 3) outside it. Fix: Panel::arrow (model.rs) returns early in pattern mode for rewind, ff, bar_*, loop_start_*, loop_length_*; mouse, held-repeat and numpad keys all reach it. Numpad 1/2 locate (Panel::locate_loop) is not in the task's list and not checked against the exe; left unchanged. Existing tests that wound in pattern mode (interaction.rs, keyboard.rs wind-held) now switch to song mode first. New tests: wired.rs the_wind_bar_and_loop_arrows_send_nothing_in_pattern_mode, keyboard.rs the_wind_bar_keys_do_nothing_in_pattern_mode. docs/ui_spec.md s11 records the addresses.

Review round 1: numpad 1/2 locate checked (sub_42D1D3 cases 5/6 at 0x42D24C/0x42D269 -> 0x429DBF, msg 8 at 0x429D77; this+0x2C not tested), so it stays ungated; the earlier 'not checked' note is superseded. Recorded as [C] in docs/ui_spec.md s10 and model.rs locate_loop. Commit Testing line now 'cargo test --release 430 pass'.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Panel::arrow now ignores Rewind, Fast forward, Bar up/down and Loop start/length up/down in pattern mode (the song-mode block of sub_42A058, gate at 0x42A074); tempo arrows and the loop switch stay active. Tests added in wired.rs and keyboard.rs (mouse and keyboard, pattern and song mode); two older tests now enter song mode first. Branch worktree-agent-a99e745323b6dd288, one commit. cargo test --release whole workspace passes.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-050
title: 'Activation (msg 53): the conditional delay-buffer refill'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-26 21:40'
updated_date: '2026-10-27 03:46'
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
- [x] #1 The condition at 0x1001BDC7..0x1001BDCC is identified and documented in docs/export_state.md with addresses and [C]/[G]
- [x] #2 Engine::activate models it (or takes it as a parameter with the corpus default), with a test for both branches; the corpus tests stay exact
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. RE sub_10013DA0 and the writers of its +1 byte (llvm-objdump of original/ReBirth.dll, IDA headless copy).
2. The byte is in the OLD song, which is not in the new song's Engine: give Activation::ReplacesCurrent a field for the current song's play flag (corpus default: stopped); when it is set, activate does nothing.
3. Tests for both branches; update export.rs; cargo test --release (golden 164/164), clippy, fmt.
4. docs/export_state.md section 3 and 6.3; fix the frame/mod.rs comment. Not a defect, so no original_bugs.md entry.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-adeee021a61cb2ed7

Evidence (disassembly, llvm-objdump of original/ReBirth.dll and the IDA copy):
- sub_10013DA0 = lea eax,[ecx+0x158]; ret: the song's globals struct (song+0x158, the same struct whose +0x15 is the sound model byte). Its ctor sub_10018AC0 (called at 0x100113F4 from the song ctor) zeroes +1 at 0x10018AD6.
- So the test reads song+0x159 = globals byte 1, the play flag. Writers: 0x1001F80A (sub_1001F7E4, play: if 0, set 1 and slot 3 on every device) and 0x100218CA (sub_100218A0, stop: if set, clear and call sub_10021840 = slot 4). sub_1001F7C0(bool) dispatches to the two. Msg 20 reads it at 0x1001202E (the play query reply). In rb this is seq.globals.play (seq.rs play/halt).
- sub_1001BDA4: esi = transport+0x16C (the current song); pending +0x170 = new song (0x1001BDB6); if current && current.play != 0 -> return at 0x1001BE03, neither slot 6 nor sub_1001BD24 (the new song does NOT become current then). The pending song is picked up by sub_1001C520 (0x1001C534: if transport+0x160 and +0x170, locate the pending song to 0 at 0x1001C549 and call sub_1001BE74 at 0x1001C55A), which stops the old song (0x1001BEB1 sub_100218A0), slot 6 on the old (0x1001BECB) and starts the new (0x1001BEEC sub_1001F7E0). When sub_1001C520 runs is not traced: [G].
- Not a defect (switching songs on the fly while playing is deliberate), so docs/original_bugs.md unchanged.
- The old song's play flag lives in another song object, not in the activated Engine: modeled as Activation::ReplacesCurrent { current_playing } with the corpus value false (rbhost's untitled song and the app's 'Untitled song' are never played before the export's activation).

Results: cargo test --release 207 pass, 0 fail (golden: diag 145 + app 11 + shipped 8 = 164/164 exact); new test frame::tests::msg53_over_a_playing_song_leaves_the_delay_line (playing branch: line and gate untouched), msg53_replacing_a_song_refills_the_delay_line now runs the stopped branch and NoneCurrent. clippy -D warnings clean, fmt clean. Branch already on main 08b08d7, no rebase needed. IDA copy closed with save=false.

Review round 1 (send back, docs only): sub_1001BE74 was described as unconditional. Rechecked in the disassembly and fixed in docs/export_state.md §3: with no old song (0x1001BE87), old == new (0x1001BE8F) or the old song stopped (sub_1001FD50 reads its play flag at 0x1001FD63; test 0x1001BE9F..0x1001BEAA) bh = 0 and it jumps to 0x1001BED0, skipping the stop and slot 6; else bh = 1 (0x1001BEAF), stop 0x1001BEB1, sub_1001BE14(old,0), slot 6 0x1001BECB. Then if transport+0x15C == 0x2E: bh set -> start the new song 0x1001BEEC + sub_1001BE14(new,1), bh clear -> only transport+0x151 = 1 (0x1001BF17); if != 0x2E: always start the new song (0x1001BF23) + sub_1001BE14(new,1). Every path stores current = new at 0x1001BF07 (so old == new does not return early, it skips the stop). Correction to the earlier note: sub_1001F7E0 is an entry that loads its argument and falls into the play body sub_1001F7E4. Added to the doc and the frame/mod.rs comment: rb does not model the pending-song switch; current_playing = true leaves the new song not current, so a live player must. Commit amended.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The byte at sub_10013DA0()+1 is the current song's play flag: sub_10013DA0 returns song+0x158 (the globals struct), so the byte is song+0x159, zeroed by the ctor sub_10018AC0 (0x10018AD6), set by play sub_1001F7E4 (0x1001F80A), cleared by stop sub_100218A0 (0x100218CA). When it is set, msg 53 (sub_1001BDA4) only stores the pending song and returns at 0x1001BE03: no slot 6, no switch; the switch is done later by sub_1001C520 -> sub_1001BE74 (stops the old song, slot 6 on it at 0x1001BECB, starts the new one), and when that runs is [G]. This is the on-the-fly switch while playing, not a defect, so original_bugs.md is unchanged.

Change: Activation::ReplacesCurrent gets a current_playing field (the old song's flag, which is not in the activated Engine); Engine::activate does nothing when it is true; export_song passes false (the corpus default); the frame/mod.rs comments are fixed; there is a test for each branch; docs/export_state.md sections 3 and 6.3 are updated with the addresses and [C]/[G].

Tests: cargo test --release 207 pass (golden 164/164 exact), clippy clean. Branch worktree-agent-adeee021a61cb2ed7, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

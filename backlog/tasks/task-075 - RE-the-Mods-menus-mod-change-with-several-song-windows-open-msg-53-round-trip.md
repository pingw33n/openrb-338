---
id: TASK-075
title: >-
  RE: the Mods menu's mod change with several song windows open (msg 53 round
  trip)
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-27 14:33'
updated_date: '2026-10-27 20:16'
labels:
  - re
  - rbm
  - ui
milestone: m-3
dependencies:
  - TASK-056
references:
  - docs/rbm.md
  - docs/export_state.md
priority: low
type: spike
ordinal: 99000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: on a mod change sub_42D80D destroys the song window (sub_47350E) and builds a new one (sub_42DA72). With other song windows open, Windows may activate another song's window in between (MDI activation sub_42D7BE -> sub_40D163 -> top-level msg 53 for that song) before the new window activates this song again (msg 53 back, 0x42E5AE). On stopped songs that runs slot 6 on both songs (the shared delay line refilled, docs/export_state.md section 3); on a playing one the other song becomes pending. docs/rbm.md section 14.2 marks this [G]. rb's UI has one song, so this matters only if rb ever opens several songs.
Do: confirm or refute the round trip, statically (MDI destroy/activate order in the exe's window code) or on the test machine (two songs open, a mod change on one, with an rbhost-style message trace or by ear/export), and record the result in docs/rbm.md section 14 with addresses.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/rbm.md section 14 states, with addresses or a test-machine run, whether msg 53 is sent for another song during a mod change with several songs open
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Static: trace sub_42D80D's destroy (sub_47350E, sub_473401) to the window destructor and the Win32 call it makes; find which window message reaches vtable slot 14 (sub_42D7BE) in the window procedure.
2. The rest is user32's MDI behavior (whether destroying the active child activates the next one and sends it WM_NCACTIVATE TRUE), not in the exe: check it on the test machine with a small Win32 probe (mdiprobe.c, scratchpad, llvm-mingw) that mimics the sequence: two MDI children, WM_MDIDESTROY of the active one, then a new child; with the frame active (Mods menu) and with an owned popup active (the Mod Selector dialog).
3. Record the result in docs/rbm.md 14.1/14.2 with addresses; docs/export_state.md 3 if needed; docs/original_bugs.md if a defect.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a9163fad85394058a

Static (Rebirth.exe.i64 headless copy, disassembly):
- sub_47350E only sets [this+18h]=1; sub_42D80D then calls sub_473401 (0x42D892), which deletes each marked window of the list dword_4DFA60 through vtable slot 0 (0x4734D8..0x4734E2). CReBirthWindow (vtable 0x4BBFB4) dtor chain sub_42F2D0 -> sub_42E6CA -> sub_498085 -> sub_49629E -> sub_49617E: unlinks (sub_472A09, 0x49620F), then for type 1 (+0x3C, MDI child) push 221h (WM_MDIDESTROY) to the MDI client, call [dword_4DFB24+4] at 0x49626B. All before sub_42DA72 (0x42D8BE).
- Window proc sub_49395B: 0x86 -> sub_494DF1 (call at 0x493E2C); wParam TRUE -> sub_473376(hwnd), call [edx+38h] at 0x494E20 = slot 14 = sub_42D7BE (0x4BBFEC) -> sub_40D163(doc) at 0x42D7DD -> msg 53 if doc != app+0x2544. WM_MDIACTIVATE (0x222, sub_494DCF) only goes to DefMDIChildProc.
- Failed hypothesis: sub_47350E is DestroyWindow; it is a deferred-delete mark.
- The rest (does WM_MDIDESTROY give the next child WM_NCACTIVATE 1?) is user32 behavior, so not settled by the exe.
Test machine (all 6 permits of redc-win-run held, released; C:\re\rb\t075 deleted): probe mdiprobe.c (scratchpad, i686 llvm-mingw), MDI frame, children A and B (B active), WM_MDIDESTROY B, create B2.
- Frame active: B NCACTIVATE 0, B MDIACTIVATE, A NCACTIVATE 1, A SETFOCUS, A MDIACTIVATE, B WM_DESTROY; then B2 creation: A NCACTIVATE 0, B2 NCACTIVATE 1.
- Owned popup active (frame disabled, and frame enabled): A gets only WM_MDIACTIVATE, no NCACTIVATE 1.
- First probe run crashed (0xC0000005): GetWindowTextA in every child message recursed through WM_GETTEXT; fixed by naming only the logged messages.
Result: Mods menu (frame active) sends 53 {other} then 53 {this} (0x42E5AE); Mod Selector OK (CModalDialog, sub_495B0B(2,...) at 0x49838F, top-level type 2, active) sends none. Recorded in docs/rbm.md 14.5 (14.1, 14.2, 14.4 pointers), docs/export_state.md 3, docs/original_bugs.md Q-4 (side effect, not claimed a defect).

Review round 1 (send back), fixed:
1. rbm.md 14.1: sub_47350E now 'marks for deletion' (song window and dialog); the deletion is sub_473401 at 0x42D892.
2. rbm.md 14.5: marks split. The exe code is [C], the MDI order is [C for the test machine's Windows, from a probe, not Rebirth.exe], and the composed Mods-menu sequence is [G: inferred]. 14.1/14.2 say 'inferred'.
3. Mod Selector OK: the lead and Q-4 now say the probe tested an owned popup being active; the OK conclusion is [G].
4. Q-4 addresses re-read with ~/llvm-mingw/bin/llvm-objdump -d --x86-asm-syntax=intel on original/Rebirth.exe: 0x42D886 call 0x47350e, 0x42D892 call 0x473401, 0x42D8BE call 0x42da72, 0x49625C push 0x221, 0x49626B call [edx+0x4], 0x494E20 call [edx+0x38], 0x42D7DD call 0x40d163, 0x42E5AE call 0x40d163. All match.
5. 'an audio callback may render the other song' marked [G] in rbm.md 14.5 and Q-4.
6. rbm.md probe bullet reworded ('the frame being the active window') and wrapped.
7. The probe ran only from C:\re\rb\t075 (uploaded, run and deleted there); nothing was written under C:\ReBirth RB-338, and Rebirth.exe was not started.
8. Assignee set to @agent-complex.

Review round 2 (send back), fixed: A. Final summary rewritten with the split marks (exe code [C], MDI order measured with the probe, Mods-menu sequence [G: inferred], OK result [G: inferred, not traced]). B. 14.5 rewrapped: the 'So, for the Mods menu' bullet, the Mod Selector OK bullet and the destructor-chain line now fit the line width.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Result, with split marks (docs/rbm.md 14.5, docs/original_bugs.md Q-4):
- The exe code [C, disassembly; Q-4 addresses re-read with llvm-objdump]. sub_42D80D's destroy is synchronous: sub_473401 deletes the marked window, and its CABCWindow destructor sub_49617E sends WM_MDIDESTROY (0x49626B) before sub_42DA72 builds the new window. WM_NCACTIVATE 1 runs sub_494DF1 -> slot 14 sub_42D7BE -> sub_40D163, which sends top-level 53 for the window's song when it is not current.
- Windows' MDI order [C for the test machine's Windows, measured with a Win32 probe that mimics these calls, not with Rebirth.exe]. With the frame being the active window, WM_MDIDESTROY of the active child gives the next child WM_NCACTIVATE 1 before the old one's WM_DESTROY. With an owned popup being the active window, the next child gets no WM_NCACTIVATE 1.
- The Mods menu sequence [G: inferred from the two, not observed]: msg 19, then 53 {other song}, then 53 {this song} at 0x42E5AE.
- The Mod Selector's OK [G: inferred, not traced]: the dialog (a CModalDialog, a top-level window) is assumed active when OK runs, so no msg 53.
- DLL effect: none on a playing song; on a stopped one, slot 6 on both songs (the shared delay cleared).
Docs: docs/rbm.md 14.5 (new) and pointers in 14.1/14.2/14.4, docs/export_state.md 3, docs/original_bugs.md Q-4. rb opens one song: nothing to model. Branch worktree-agent-a9163fad85394058a, one docs commit; tests: none (docs only).
<!-- SECTION:FINAL_SUMMARY:END -->

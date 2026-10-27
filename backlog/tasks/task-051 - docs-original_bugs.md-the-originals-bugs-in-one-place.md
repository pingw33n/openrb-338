---
id: TASK-051
title: 'docs/original_bugs.md: the original''s bugs in one place'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 03:04'
updated_date: '2026-10-27 03:14'
labels:
  - docs
  - re
milestone: m-1
dependencies: []
references:
  - docs/initial_investigation.md
  - docs/song_load.md
  - docs/export_state.md
priority: high
type: docs
ordinal: 28800
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The user wants the bugs of the original ReBirth 2.01 (Rebirth.exe and ReBirth.dll) tracked in one place: `docs/original_bugs.md` (requested 2026-10-27). rb reproduces the original's behavior, bugs included, so each bug needs: what goes wrong, the address(es), the trigger (which song data or action reaches it), the effect on audio or files, whether any corpus song reaches it (name the song), how rb models it (the file and function, and any deliberate deviation such as a clamp where the original reads run-time memory), and a [C]/[G] mark.

Known candidates, collected from docs/ and the task notes of tasks 018-027 (verify each against the docs and the cited addresses, keep only real defects of the original, and list quirks that are not bugs separately or leave them out):
- PCF wave byte used as a pattern index with no bounds check (waves 55..127 read past the 55 patterns into PCF_DATA, the vtable and relocated pointers); waves 55, 111 and 113..127 have pattern length 0 and divide by zero at 0x10016811; SafeSoundCallback (0x1001AE40, handler 0x1001B055) swallows the exception and leaves the output buffer unwritten, so the export is silence (diag pcf_lp_wave127, pcf_bp_wave127; task-039; rb's frame::Fault and the CLI warning); waves 55..126 with a resolution other than 8/16/32 use a stale stack quotient [G].
- The export buffer is never cleared, so a faulting chunk repeats the previous chunk (sub_427988 at 0x427C2F, sub_4280D5).
- Distortion amount/shape above 127 index past the 5x5 grid; delay pan above 127 indexes past PAN row 127 (rb clamps; task-026).
- PCF decay and resonance bytes past their tables (task-027 clamps).
- The 909 song loader copies RBS byte 13 into slot 16 (0x433558, docs/song_load.md 4.8).
- The old (4.0/4.1) 808 track's tom tune applied before the range check, so a file value of 190 becomes 127 and is accepted (sub_432E1B at 0x437139 before sub_42FD00 at 0x4371A3).
- Track events: the exe accepts ticks up to 767976 (sub_42FD60) but the DLL drops events above 767231 (0xBB4FF at 0x1001477B); an event exactly at the loop-end tick never fires.
- The export frame count truncates in floating point (e.g. Fast Forward 18899999 frames for an exact 18900000; one bar at 120 BPM gives 88199) — decide whether this is a bug or just the original's arithmetic, and say so.
- The delay's 25-update tempo average is dead code (0x1000F7E2 overwrites +0x4C before 0x1000F7EC reads it).
Search docs/ and the backlog task notes (`grep -ri` for "bug", "dead", "past", "out of range", "no check", "quirk", "clamp") for more.

Also: add `docs/original_bugs.md` to the knowledge-document list in AGENTS.md with a one-line rule ("a newly found defect of the original goes into docs/original_bugs.md as well as its home document"), and the same line to the agent brief doc-001 if it lists the documents. Each entry in the other docs that describes one of these bugs should link to its entry in original_bugs.md.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/original_bugs.md lists every known defect of the original with addresses, trigger, effect, corpus songs that reach it, rb's model or deviation, and a [C]/[G] mark
- [x] #2 Each address in it is checked against original/Rebirth.exe or original/ReBirth.dll (llvm-objdump), and the home documents link to the entries
- [x] #3 AGENTS.md (and doc-001 if it lists documents) names docs/original_bugs.md with the rule for new findings
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Read the candidates' home sections; re-read every cited address with llvm-objdump on original/; write docs/original_bugs.md (OB-1..OB-12, Q-1); link from the home docs; update AGENTS.md and doc-001.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-aa4718377bc192ddd

Addresses checked with llvm-objdump (intel syntax): DLL 0x10016811 idiv ecx; 0x1001477B cmp eax,0BB4FFh + jg; 0x1000F7E2 mov [ebx+4Ch],eax before 0x1000F7EC reads it (note the jne at 0x1000F7EA skips that read when [ebx+36h]!=0); SafeSoundCallback 0x1001AE40, handler 0x1001B055 returns 0x1001B05B -> jmp 0x1001B03D. EXE 0x433558 mov dl,[ecx+0Dh] / mov [eax+10h],dl; 0x437139 call 0x432E1B; 0x4371A3 call 0x42FD00; sub_42FD60 cmp 0BB800h; 0x427C2F after push 2000h/new; 0x4281C3 call [4DA5C0]; 0x4281F2 call [edx+0Ch]; chunk cap cmp 0x800 at 0x428186. Frame count: re-ran the loop in IEEE double (n=trunc((end-pos)/(t*(192/2646e6))), pos+=(t*n)*(192/2646e6), chunks of 2048): 88199 for one bar at 120 BPM, 18899999 for 250 bars at 140 BPM; listed as quirk Q-1, not a defect. Not confirmed instruction by instruction: OB-9/10/11 (no bounds check on dist amount/shape, delay pan, PCF decay/resonance) rest on task-026/027 notes; marked [G]. Corpus songs per entry only where the notes name them (pcf_*_wave127, pcf_*_wave64); others 'none known'. Dropped from candidates: nothing; the PCF unchecked-resolution stale quotient is kept as [G] OB-7.

Review round 1 (send back), fixed: (1) OB-6/OB-8 home link now export_state.md section 13; (2) CORRECTION of my earlier note: the delay tempo average is dead on EVERY path, not only [ebx+36h]==0: 0x1000F7E2 runs on every update, both length reads (0x1000F7EC x3 formula, 0x1000F83F x4 formula) see it, the jne at 0x1000F7EA only picks the formula; OB-12 and initial_investigation 8.4 rewritten; (3) back-links added in initial_investigation sections 5, 8.1, 8.4; (4) OB-9/10/11 marked [C] for the unchecked index (0x10010917/0x10010932/0x100109DC; 0x1000F8A5..0x1000F8B1; 0x1001684C..0x1001685E, 0x1001691B..0x10016941), [G] only for reachability from a file; all re-read with llvm-objdump; (5) OB-4 now [G]; (6) OB-8 0x4281F2 described as a virtual call (writer, [G]); (7) Q-2 added for the state carry-overs (303 filter coefficients, snare noise counters).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added docs/original_bugs.md: 12 defects (OB-1..OB-12; song loading, sequencer, devices) and quirk Q-1 (export frame count), each with what/where/trigger/effect/corpus/rb/mark. Home docs (song_load.md, initial_investigation.md, export_state.md) link to the entries; AGENTS.md and doc-001 name the file with the rule for new findings. Addresses re-read with llvm-objdump. Docs only, no tests.
<!-- SECTION:FINAL_SUMMARY:END -->

---
id: TASK-051
title: 'docs/original_bugs.md: the original''s bugs in one place'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-27 03:04'
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
- [ ] #1 docs/original_bugs.md lists every known defect of the original with addresses, trigger, effect, corpus songs that reach it, rb's model or deviation, and a [C]/[G] mark
- [ ] #2 Each address in it is checked against original/Rebirth.exe or original/ReBirth.dll (llvm-objdump), and the home documents link to the entries
- [ ] #3 AGENTS.md (and doc-001 if it lists documents) names docs/original_bugs.md with the rule for new findings
<!-- AC:END -->

---
id: TASK-005
title: 'Tool: rbs.py, an RBS 4.2 reader and writer'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 10:47'
labels:
  - tools
milestone: m-0
dependencies:
  - TASK-002
references:
  - docs/RBS42.txt
priority: high
type: feature
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The tests need songs made for them (one instrument, one effect, a tempo) and a way to read the songs that ship with ReBirth. Write `tools/rbs.py`, a stdlib-only Python reader and writer of the RBS 4.2 format (docs/RBS42.txt: IFF CAT RB40 with HEAD, GLOB, USRI, CAT DEVL, CAT TRKL; TRAK delta positions MIDI-style), exposing a typed model (dataclasses) and a CLI: `rbs.py dump FILE` (human-readable), `rbs.py json FILE` (JSON of the model), `rbs.py build SPEC.json OUT.rbs` (write a song from a JSON spec, every field defaulted from a template song). Unit tests in `tools/rbs_test.py` (`/usr/bin/python3 -m unittest`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every .rbs song shipped with the installation (docs/test_machine.md) reads and writes back byte-identical
- [x] #2 build writes a song from a minimal JSON spec, and the original ReBirth opens it without an error (screenshot in the notes)
- [x] #3 tools/rbs_test.py passes and covers the delta-position encoding edge cases (0, 127, 128, 16383, 16384, 31976)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. tools/rbs.py: IFF reader/writer, dataclass model (unmodeled bytes kept in a hidden raw so the round trip is byte-identical), JSON model, build = JSON spec merged over a template song, dump, check.
2. tools/rbs_test.py: delta encoding edge cases, 18 shipped round trips, build, validation.
3. Test machine: build a minimal song and open it in the original ReBirth.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a20c04d8c251161dd

Findings while reading the 18 shipped songs:
- USRI (712 bytes) layout differs from RBS42.txt: title at 0, text at 41 with room for 401 bytes (Transfusion's text is 212), web page at 442, show-on-open byte at 543, bytes 544..711 junk (kept raw). Added to docs/song_load.md section 10. Derived from the data, not from the exe.
- HEAD version bytes: 17 songs are 4.1 (byte 6 = 01), 'ReBirth 1.0 Default Song.rbs' is the 4.2 one.
- Alloy Cowboy Resurrection.rbs has TRAK positions up to 41024, beyond the format's 31976 and the engine's 32000 (tick 768000); recorded in docs/song_load.md section 10. rbs.py reads such songs; validate() (used by build) rejects them.
- 'ReBirth 1.0 Default Song' / 'Silent Default Song' have song_mode 0; the 16 demo songs song_mode 1.
- Every chunk, including odd sized ones (1097, 6239, odd TRAK), is followed by an IFF pad byte; CAT sizes include it.
- Design: build merges the spec over the template song (default tests/songs/shipped/Silent Default Song.rbs, 4.1); output version defaults to 4.2 and the template's 808 tom tunings are converted v/2+32 (sub_432E1B, from docs/song_load.md 4.7; not re-verified in the exe). Not verified: that other 4.1 vs 4.2 differences do not matter for an empty template.
- Delta position 31976 encodes as 81 F9 68 (my first hand calculation, 81 F3 68, was wrong; the test uses the RBS42.txt algorithm too).
Test machine run (permits: all 6 of redc-win-run, owner worktree-agent-a20c04d8c251161dd, released after): 'rbs.py build spec.json test_build.rbs' (spec: tempo 140, song mode, 303 #1 pattern 0 with 3 notes, window title 'rbs.py build test'), rcs put to C:\re\rb\test_build.rbs, started Rebirth.exe (PID 9704), Enter on the MIDI box, Ctrl+O, typed C:\re\rb\test_build.rbs, Enter. The app opened it with no error: song window 'test_build - rbs.py build test', tempo 140, 303 #1 on. Screenshot: docs/test_machine/rbs_build_opened.png. Stopped only PID 9704 with taskkill; confirmed gone; permits released.
Reproduce: /usr/bin/python3 -m unittest tools/rbs_test.py (17 tests); /usr/bin/python3 tools/rbs.py check tests/songs/shipped/*.rbs (18 ok).

Review round 1: fixed the Alloy Cowboy wording in docs/song_load.md; removed unused import copy; MAX_POS kept at 31976 (RBS42.txt's limit) with a comment that the tool is deliberately stricter than the exe (accepts < 32000). Re-ran: unittest 17 OK, check 18 ok.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
tools/rbs.py (reader, writer, typed dataclass model, JSON, dump, build, check) and tools/rbs_test.py (17 tests). All 18 shipped songs read and write back byte-identical. build merges a JSON spec over a template song; the original ReBirth opened a built song on the test machine without error (docs/test_machine/rbs_build_opened.png). The model uses the corrected meanings of docs/song_load.md section 10 (32nd-note positions, 909 mixer controllers 0x1A/0x1B, PCF ids 1..4, 303 rest bit, delay mode 0 = triplets). New facts (USRI layout, TRAK positions beyond 31976 in Alloy Cowboy) are in docs/song_load.md section 10. Branch worktree-agent-a20c04d8c251161dd, one commit. Tests: unittest 17 OK; check 18 ok.
<!-- SECTION:FINAL_SUMMARY:END -->

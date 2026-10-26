---
id: TASK-005
title: 'Tool: rbs.py, an RBS 4.2 reader and writer'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
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
- [ ] #1 Every .rbs song shipped with the installation (docs/test_machine.md) reads and writes back byte-identical
- [ ] #2 build writes a song from a minimal JSON spec, and the original ReBirth opens it without an error (screenshot in the notes)
- [ ] #3 tools/rbs_test.py passes and covers the delta-position encoding edge cases (0, 127, 128, 16383, 16384, 31976)
<!-- AC:END -->

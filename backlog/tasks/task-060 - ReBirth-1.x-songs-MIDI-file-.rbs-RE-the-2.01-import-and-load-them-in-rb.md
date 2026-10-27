---
id: TASK-060
title: 'ReBirth 1.x songs (MIDI-file .rbs): RE the 2.01 import and load them in rb'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-27 08:19'
updated_date: '2026-10-27 08:58'
labels:
  - re
  - engine
  - cli
milestone: m-2
dependencies: []
references:
  - docs/song_load.md
  - docs/rbm.md
priority: medium
type: feature
ordinal: 53000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-058 found that 129 of the 193 distinct songs of the user's collection (~/rb_stuff/songs, read in place from RB_THIRDPARTY_DIR, never committed) are ReBirth 1.x songs: a Standard MIDI File (MThd/MTrk) with the .rbs extension, not RBS 4.2 (IFF CAT RB40). rb refuses them (rbs::Song::parse: 'not a valid .rbs song'); tools/rbs.py likewise. The original 2.01 opens them.

What the exe does (task-058, Rebirth.exe IDA, read-only copy):
- File > Open builds the song with sub_425C4D (docs/song_load.md section 2). It first calls sub_43037D (0x425DE4), which opens the file as a MIDI file (sub_4A0B38, the strings MThd/MTrk at 0x4D2AB0/0x4D2AC0) and reads its version with sub_431341: it walks the first track's events until a meta text event FF 02 whose text is 'File version %s' (sscanf at 0x431481; major and minor must be < 100, else XBadFormat).
- If that succeeds, the version is checked by sub_47DA1C(&unk_4DAD60) (else XUnsupportedFormat, 0x42613B), the song gets the built-in mod (dword_4DA784, no Mods lookup) and is loaded by sub_430BA0 (0x425FCE) instead of the RBS reader sub_434D05.
- The collection's 1.x files carry 'File version 3.1.0 (c)1997 Propellerhead Software, all rights reserved' in their first track.

Steps: RE sub_430BA0 and its callees (the MIDI track layout of a 1.x song: tempo, patterns, song events, the device set of 1.x: two 303s and an 808, no 909/effects?) into a new section of docs/song_load.md with addresses; implement the reader (crates/rbs or a new module) and the conversion to the engine model in rb-engine/load; render a sample of 1.x songs with the original (rbhost cannot load a MIDI song directly: either convert through the app by opening the song and saving it as RBS 4.2 on the test machine, or feed rbhost the messages the import sends) and show rb byte-exact.

Context: the env-gated check crates/rb-cli/tests/thirdparty.rs (task-058) classifies these songs as RBS1X and expects rb to refuse them; it must be updated to expect a render.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/song_load.md documents the 1.x import (sub_430BA0 and callees) with addresses
- [ ] #2 rb render loads a ReBirth 1.x MIDI-file song as the original does; the thirdparty check renders all 129 1.x songs of the collection without a panic
- [ ] #3 At least 5 1.x songs render byte-exact against the original (through rbhost or the app's export)
<!-- AC:END -->

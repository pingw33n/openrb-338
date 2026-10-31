---
id: TASK-148
title: Repair or remove the three unreadable songs in the song collection
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-29 14:57'
updated_date: '2026-10-31 10:08'
labels:
  - tests
dependencies:
  - TASK-129
references:
  - tests/refs/thirdparty_songs.json
  - docs/rbm.md
  - docs/song_load.md
priority: low
type: task
ordinal: 166000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked on 2026-10-29 for a task of their own for the songs of ~/rb-songs/songs that neither the original ReBirth nor orb opens. task-129 lists them as skipped in tests/refs/thirdparty_songs.json, with the original's reason: the exe's sub_434D05 -> sub_49F3B4 (0x434D56) requires the file to be exactly one FORM, CAT or LIST chunk (docs/song_load.md 2.1). A fourth, CaveysRebirthStuff/Mod Demo Songs/AXIOM peace.rbs, was only unreadable to orb (a 5-byte delta), and task-129 fixed it.

The three files, with what is known:
- CaveysRebirthStuff/ReBirth 2.x Songs/Pack34/yawn.rbs: a CAT chunk of 18018 bytes in a file of 18020 bytes. The 2 extra bytes at the end break the exactly-one-chunk rule. Cutting those 2 bytes may give a valid song: check what they are first (padding, or a sign the file is damaged).
- CaveysRebirthStuff/Battle of the Best/Top 25/005/Wax and Rain.RBS: a BinHex text, the classic Mac encoding (starts with "(This file must be converted with BinHex"). Decoding it, for example with `binhex` on macOS, StuffIt Expander, or Python's old binhex module, likely gives the real .RBS.
- CaveysRebirthStuff/Battle of the Best/ChemicalPack/thefur~1.rbs: no chunk header at offset 0. It may be another format, a compressed or truncated file, or a mis-named file; look at its first bytes.

What (the user's work):
- For each file: repair it (replace it with the repaired song), or delete it from the collection, or leave it as a known-bad file.
- A repaired or deleted file changes the collection, so the task-129 manifest must be regenerated with tools/rbhost/thirdparty_songs.py on the test machine afterwards (ask the supervisor). A repaired song is then rendered and pinned like the others; the gated songs test fails until the manifest is updated.
- If a repaired file opens in ReBirth but not in orb, that is an orb bug: tell the supervisor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each of the three files is repaired, removed, or kept as known-bad, and recorded here
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor, 2026-10-29, a look at the bytes (read only):
- yawn.rbs: the 2 extra bytes are 0C 1A. 1A is the DOS end-of-file mark (Ctrl-Z), so they are probably junk from a transfer, and cutting them likely gives the original song.
- Wax and Rain.RBS: it starts with '(This file must be converted with BinHex 4.0)', a BinHex 4.0 text; decode it.
- thefur~1.rbs (63766 bytes): the first bytes (7A 79 7C 86 9E 78 59 73 ...) hover around 0x80. That looks like raw 8-bit unsigned audio, not a song: probably a sample saved with the wrong name. It is likely not repairable as a song.

2026-10-31, the user (rb-songs commits e54e60d and 4af1075):
- Battle of the Best/ChemicalPack/thefur~1.rbs is removed from the collection.
- Top 25/005/Wax and Rain.RBS is repaired. orb info: RBS 4.2, built-in kit.
- ReBirth 2.x Songs/Pack34/yawn.rbs is repaired. orb info: RBS 4.2, mod AXIOM peace.
AC #2 (the manifest matches and the songs test passes) moved to task-168, which regenerates the manifests.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Closed by the user on 2026-10-31. Two of the three unreadable songs are repaired (Wax and Rain.RBS, yawn.rbs) and one is removed (thefur~1.rbs), in ~/rb-songs commit 4af1075. The manifest regeneration and the gated songs test moved to task-168.
<!-- SECTION:FINAL_SUMMARY:END -->

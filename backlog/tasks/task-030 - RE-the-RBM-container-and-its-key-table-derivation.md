---
id: TASK-030
title: 'RE: the RBM container and its key table derivation'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 10:12'
labels:
  - re
  - rbm
  - docs
milestone: m-2
dependencies: []
references:
  - docs/initial_investigation.md
priority: high
type: spike
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
RBM mod samples are scrambled (docs/initial_investigation.md section 7.4): the DLL descrambles each EMBF chunk past byte 1024 with `buf[n] += table[(key&1)*2048 + n % ((17*key % 1925) + 123)]`, but the 4096-byte key table is derived in obfuscated code in Rebirth.exe (0x4158A9, 0x4149EC): sums of the chunk's first 1024 bytes as big-endian u32 with a stride from anti-tamper globals (dword_4DAE58) and an API value, indexing 2048-byte halves read from a file (app field this[325]) at offset 256 + sum % (0x8000000-2048). Find which file that is, the exact stride in an untampered run, and the INFO and HEAD chunk layouts; document the full RBM format in `docs/rbm.md`. If static analysis stalls, propose a dynamic check (a debugger session on the test machine dumping the table for a known mod) in the notes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/rbm.md documents the RBM container (RBM, INFO, HEAD, EMBF), the sample lookup by name and the full descrambling including the key table's source and stride
- [ ] #2 The descrambling is shown correct on one real mod: the samples it yields parse as AIFF and match the original's loaded samples (dumped by rbhost or a debugger)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-003 (2026-10-26, docs/song_load.md section 9): the file behind app field this[325] is Rb20ful.dat on the ReBirth CD (volume "ReBirth Win"), found and validated by sub_406A79 / sub_4063DF (magic 0x03E1F11E, string RB338V20, major version 4; strings stored encoded, decode s[j] -= s[j+1] from the end); sub_4065F2 fills 2048-byte pages from it at offsets derived from runtime values. The test machine has the CD mounted (drive E:, seen in Explorer on 2026-10-26); read it with rcs by absolute path (doc-001).
<!-- SECTION:NOTES:END -->

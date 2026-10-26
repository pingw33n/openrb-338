---
id: TASK-030
title: 'RE: the RBM container and its key table derivation'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 09:06'
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

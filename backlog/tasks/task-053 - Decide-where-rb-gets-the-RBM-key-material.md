---
id: TASK-053
title: 'Decide: where rb gets the RBM key material'
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-27 04:58'
updated_date: '2026-10-27 07:44'
labels:
  - decision
  - rbm
milestone: m-2
dependencies: []
references:
  - docs/rbm.md
priority: high
type: spike
ordinal: 31500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
RBM mods are scrambled with a 4096-byte key table: two 2048-byte pages of RB20FUL.DAT (ReBirth CD; 256-byte header + 2^27 bytes of key material) at offsets 256 + sum[j] % (0x8000000 - 2048), the sums fixed by each mod's own bytes (task-030, docs/rbm.md section 5). How should rb get that material?

Options:
1. The user's RB20FUL.DAT at run time (a path option, an environment variable, a default location), like the drum samples from ReBirth.dll (task-016). Nothing of Propellerhead's is shipped.
2. A one-time import that decrypts a .rbm with the user's RB20FUL.DAT into a clear mod rb stores; rb then needs no key material.
3. Bundle the 4096 bytes each known mod needs (16 KB for the four shipped mods).
4. Bundle the whole 128 MiB file.

Recommendation: option 1.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-27: per the user, task-059 embeds the minimum RB20FUL.DAT key data in rb and adds `rb recrypt`, so recrypted mods load without the CD; this supersedes this decision's "nothing bundled" for that embedded minimum only. Other mods still read the user's RB20FUL.DAT at run time.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Decided by @pingw33n on 2026-10-27: option 1. rb reads the RBM key pages at run time from the user's own RB20FUL.DAT (a path option such as --rb20ful PATH, an environment variable, then a default location), as it reads the drum samples from the user's ReBirth.dll (task-016). No key material is bundled, imported or committed; no import step for now. Tests read the file from an environment variable (RB_RB20FUL) and skip when it is not set.
<!-- SECTION:FINAL_SUMMARY:END -->

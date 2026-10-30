---
id: TASK-037
title: Bundle a free drum kit and UI art so the original is not required
status: Done
assignee: []
created_date: '2026-10-26 14:40'
updated_date: '2026-10-30 08:03'
labels:
  - engine
  - ui
  - rbm
  - tools
dependencies: []
references:
  - docs/initial_investigation.md
priority: low
type: feature
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Make orb usable without a copy of the original ReBirth: ship a freely licensed default drum kit and UI art of our own, so a user who has no ReBirth CD (task-036 seeds from one) can still load and play .rbs songs.

Drum kit. The engine plays samples into 46 slots (docs/initial_investigation.md section 7.3/7.4): 808 BD, SD1-5 (clean, switched by Tune), N1-4 (noise, chosen by the fixed SEQ808 order), CB, CH, CL, CP, CY, HC, HT, LC, LT, MA, MC, MT, OH, RS; 909 bd1-4 (switched by Tune), sd1-5, sdn1-3, lt, mt, ht, rs, hc, chh, ohh, ccy, rcy; plus the startup sound. The kit must fill every slot with 44.1 kHz 16-bit mono samples under a license that allows redistribution (CC0 or similar; record source and license per file), respecting the slot semantics (the snare variants of one instrument the same length; tunable toms/cymbals played by speed). Songs rendered with this kit are not comparable to the original's references; the golden tests keep using the original samples. Packaging: the same mod-like layout as an RBM so the engine loads it through the m-2 mod path, or a plain folder; decide in the task.

UI art. The future UI (after m-2) needs panel graphics with a similar look and feel; the original skin is Propellerhead's (task-036 copies it from a CD when present). Draw or source an original set under a redistributable license.

RBM keys (to consider, needs a decision). Third-party mods (.rbm) have their samples scrambled with a key table derived from RB20FUL.DAT on the ReBirth CD (2^27 bytes of key material after a 256-byte header; docs/initial_investigation.md section 7.4, task-030, task-036). Reading third-party mods without the CD needs that material. Options: bundle the whole file (128 MiB, Propellerhead's data); bundle only the 2048-byte pages known mods use (if the derivation shows each mod needs few pages: compute the page set over a corpus of mods); or require the CD (task-036). A further option avoids keeping key material at all: an import step that, given the user's CD (or its RB20FUL.DAT), decrypts a third-party .rbm once into an unscrambled form orb stores (a plain mod container with clear AIFF samples), so the app itself runs with no key material and loads only imported mods; the CD is needed only at import time. Whether redistributing any of it is acceptable is the user's decision: open a Decide task for @pingw33n with what task-030 learns before doing it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A redistributable drum kit fills all 46 sample slots with per-file source and license recorded, and orb renders every shipped song with it without the original installed
- [ ] #2 An original UI art set exists under a redistributable license, covering the panels the UI needs
- [ ] #3 A Decide task records the user's choice on bundling RBM key material, with task-030's findings on how many pages known mods need
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-26: the UI art part is covered by milestone m-3 (bundled SVG UI, our own art); this task keeps the drum kit and the RBM-key decision.

Supervisor 2026-10-27: AC #3 is settled by task-053 (the user chose run-time RB20FUL.DAT; nothing bundled).

Supervisor 2026-10-27: task-036 no longer seeds from a CD; per the user it builds a standard RBM (the built-in kit and the standard UI skin) from the user's ReBirth.dll with `orb mod pack-standard`. References to task-036 seeding from a CD in this description are out of date.

Supervisor 2026-10-27: the command is named `orb mod pack-standard` (not standard-rbm), per the user.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Rejected by @pingw33n on 2026-10-30: not implemented. The project keeps the built-in standard kit (assets/Standard.rbm, ReBirth's own samples and panel, the user's decision of 2026-10-27, task-067) and its own SVG panel art (task-115), so a free replacement kit and art are not needed. The acceptance criteria are left unchecked on purpose.
<!-- SECTION:FINAL_SUMMARY:END -->

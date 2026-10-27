---
id: TASK-036
title: 'rb standard-rbm: a standard RBM built from the user''s ReBirth.dll'
status: To Do
assignee:
  - '@agent-complex'
created_date: '2026-10-26 14:02'
updated_date: '2026-10-27 08:06'
labels:
  - rbm
  - cli
  - re
milestone: m-2
dependencies:
  - TASK-059
references:
  - docs/rbm.md
priority: medium
type: feature
ordinal: 33800
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Rescoped by the user on 2026-10-27: this task is only about building a standard RBM from the user's own ReBirth installation. The earlier scope (seeding rb from a CD/ISO image: ISO reader, Wise installer carving, CAB/LZX extraction, RB20FUL.DAT copy) is dropped; the CD facts it collected are in this task's history (git log of this file).

`rb standard-rbm --rebirth-dll PATH [-o FILE]` writes a .rbm that holds the original's built-in kit as a mod, with the usual components of a mod (docs/rbm.md sections 2-4):
- the IFF CAT/PRBM container with HEAD, INFO (1280 bytes: a non-empty menu name, the version whose major the app checks at 0x415178, the text, ftp and web fields as the shipped mods carry them) and the EMBF chunks;
- the 46 built-in samples from ReBirth.dll's "AIFF" resources 1000..1045 (crates/rb-engine/src/samples.rs reads them; embed them under the 46 names the lookup uses, docs/rbm.md section 3), unchanged byte for byte;
- the standard UI skin: the panel graphics a mod carries (the shipped mods embed `12017.JPG` ... and `splash.jpg`, 59 JPEGs; docs/rbm.md section 2). Find where the original keeps its standard skin (ReBirth.dll or Rebirth.exe resources: bitmaps by id), how the ids map to the mods' JPEG names, and convert them to the format and names mods use; if the skin is in Rebirth.exe, the command also takes `--rebirth-exe PATH`;
- a Demo.rbs if the shipped mods' layout calls for one (e.g. the ReBirth 1.0 Default Song), or say why not.
The mod is scrambled the same way as the mods task-059 produces (the embedded minimum key window and its padding chunk), so rb reads it without RB20FUL.DAT and the original app can load it too.

The menu name: "Standard ReBirth" is the name the app and rb treat as the built-in kit (sub_4166E5; crates/rb-cli/src/mods.rs). Pick a distinct name (e.g. "ReBirth Standard Kit") so the mod is never confused with the built-in kit; say what you chose.

Nothing of Propellerhead's is committed: the command builds the mod from the user's files at run time; tests read ReBirth.dll from original/ (and Rebirth.exe from original/ if needed) and write the mod to a temporary directory.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb standard-rbm --rebirth-dll PATH writes a .rbm with HEAD, INFO, the 46 built-in samples (byte-identical to ReBirth.dll's AIFF resources) and the standard UI skin in the mods' format and names, scrambled as task-059's mods are
- [ ] #2 rb loads the produced mod without RB20FUL.DAT, and the corpus songs rendered with it equal their renders with the built-in kit
- [ ] #3 The produced mod loads in the original app on the test machine (it appears under its menu name, shows its skin) and an export with it equals the export with the built-in kit for at least two songs
- [ ] #4 docs/rbm.md documents the standard skin's source (resource ids and the mapping to the JPEG names) and the mod's layout
<!-- AC:END -->

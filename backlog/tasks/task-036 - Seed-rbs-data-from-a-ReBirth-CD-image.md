---
id: TASK-036
title: Seed rb's data from a ReBirth CD image
status: To Do
assignee: []
created_date: '2026-10-26 14:02'
labels:
  - tools
  - rbm
  - ui
  - cli
dependencies: []
references:
  - docs/initial_investigation.md
  - docs/song_load.md
priority: medium
type: feature
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Let a user seed rb from their own ReBirth RB-338 CD: give the app (CLI first, later the UI) an ISO image of the CD, or a mounted CD, and it copies out everything it needs to run into its data folder, so nothing of Propellerhead's is redistributed with rb:
- the built-in drum samples (the "AIFF" resources 1000..1045 of ReBirth.dll; task-016 decision: read from the user's ReBirth.dll);
- the RBM key material (RB20FUL.DAT, needed to descramble mod samples; task-030);
- the shipped standard skin graphics for the future UI (in the program's resources; where exactly is to be found);
- the shipped mods (.rbm) and songs.

What is known about the CD (the 2001 reissue on the test machine, drive E:, examined 2026-10-26):
- Root: Autorun.inf, "Install ReBirth RB-338.exe" (20,151,808 bytes), "Install ModPacker.exe", RB20FUL.DAT, RB20UPD.DAT, rebirth.ico, ReadMe.wri, and the folders Acrobat Reader 5.0/, DirectX/, HLD/, "Sounds Good promo/".
- RB20FUL.DAT: 134,217,984 bytes = 256-byte header + 2^27 bytes of key material. Header: magic 03 E1 F1 1E (0x03E1F11E, big-endian), then BC 01 01 00 00 (meaning not established), then "(c)1997 Propellerhead Software, all rights reserved." zero-padded. The body is read in 2048-byte pages (copy protection, docs/song_load.md section 9; the RBM key derivation reads at 256 + sum % (0x8000000 - 2048), docs/initial_investigation.md section 7.4).
- RB20UPD.DAT: 128 bytes, magic 07 F1 E1 3B, then BC 01 and zeros; role not traced.
- "Install ReBirth RB-338.exe" is a Wise for Windows Installer package: its PE section .WISE (file offset 0x3400, 0x1333A00 bytes) holds an MSI database (OLE header at file offset 0x2DD26C) and three Microsoft CAB archives (MSCF at 0x12BE4, 0x17E4E8, 0x30266C). The first two carry the Windows Installer runtime. The third (offset 0x30266C, 16,773,032 bytes, header reserve 0x1000 bytes, one folder, compression 0x1503 = LZX window 21, 32 files) carries the program: Product_Registration.pdf, ReadMe.wri, ReBirth.cnt, Rebirth.exe, ReBirth.hlp, ReBirth_Manual.pdf, ReBirth_1.0_Default_Song.rbs, ReWire.dll, MSVCRT.DLL, ReBirth.dll and more. Extracted with Windows expand.exe: ReBirth.dll SHA-256 4c3a6441a2cd50f797d99b1c5b6cf0aaf198c7ad8ca2648196b530c2e3e58b12 (identical to original/ReBirth.dll); ReWire.dll 760ea386... (identical to the installed ReWire.dll_); Rebirth.exe e69fef4153856f4ad985797a3f312a0027633c00f835c3348415b031bbd87004 (1,606,656 bytes; the installed one in original/ differs because it is a build patched to run on Windows 11).
- So seeding needs: an ISO 9660 reader, the Wise .WISE section carving (or a scan for MSCF), a CAB reader with LZX decompression (pure Rust; or the MSI's file table to map names), a PE resource reader for the samples (and the skins), and plain copies of RB20FUL.DAT and the mods. Other editions of the CD (the 1998 original) may lay this out differently: find out what to support (Decide task if needed).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Given an ISO (or a mounted CD) of the ReBirth 2.01 CD, a command copies into rb's data folder ReBirth.dll's drum samples, RB20FUL.DAT, the shipped mods and songs, and the standard skin graphics, without running Windows code
- [ ] #2 The extracted ReBirth.dll matches SHA-256 4c3a6441...e58b12 and the samples equal those loaded from original/ReBirth.dll (test)
- [ ] #3 The supported CD editions and what happens with an unknown image are documented
<!-- AC:END -->

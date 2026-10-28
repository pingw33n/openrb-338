---
id: TASK-065
title: rb recrypt and rb standard-mod read the ReBirth CD image (.iso) directly
status: To Do
assignee: []
created_date: '2026-10-27 10:02'
updated_date: '2026-10-28 09:33'
labels:
  - rbm
  - cli
milestone: m-2
dependencies:
  - TASK-036
  - TASK-064
references:
  - docs/rbm.md
priority: low
type: feature
ordinal: 91000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Requested by the user on 2026-10-27. Unassigned, not in a milestone.

Let `rb recrypt` and `rb standard-mod` (task-036) take the ReBirth CD image itself (`--iso PATH`, an ISO 9660 image of the CD) and read what they need directly from it, without extracting files first and without running Windows code:
- `rb recrypt --iso CD.iso ...`: RB20FUL.DAT is read from the image's root (in place of `--rb20ful`; the SHA-256 check of task-064 applies), and with no mod inputs given, the shipped mods on the CD are the default input (find where they sit on the CD: in the installer, see below, or as plain files).
- `rb standard-mod --iso CD.iso`: ReBirth.dll (for the 46 built-in samples and, if task-036 finds it there, the standard skin) and Rebirth.exe if task-036 needs it, read from inside the installer on the image (in place of `--rebirth-dll`).

What is known about the CD (the 2001 reissue on the test machine, drive E:, examined 2026-10-26; from task-036's earlier scope, in its git history):
- Root: Autorun.inf, "Install ReBirth RB-338.exe" (20,151,808 bytes), "Install ModPacker.exe", RB20FUL.DAT, RB20UPD.DAT, rebirth.ico, ReadMe.wri, and the folders Acrobat Reader 5.0/, DirectX/, HLD/, "Sounds Good promo/".
- RB20FUL.DAT: 134,217,984 bytes, header magic 03 E1 F1 1E.
- "Install ReBirth RB-338.exe" is a Wise for Windows Installer package: its PE section .WISE (file offset 0x3400, 0x1333A00 bytes) holds an MSI database (OLE header at file offset 0x2DD26C) and three Microsoft CAB archives (MSCF at 0x12BE4, 0x17E4E8, 0x30266C). The third (offset 0x30266C, 16,773,032 bytes, header reserve 0x1000 bytes, one folder, compression 0x1503 = LZX window 21, 32 files) carries the program: Rebirth.exe, ReBirth.dll, ReWire.dll, ReBirth_1.0_Default_Song.rbs and more. Extracted with Windows expand.exe, ReBirth.dll has SHA-256 4c3a6441a2cd50f797d99b1c5b6cf0aaf198c7ad8ca2648196b530c2e3e58b12 (identical to original/ReBirth.dll); Rebirth.exe e69fef41...87004 (1,606,656 bytes; the one in original/ differs because it is patched to run on Windows 11 — check that task-036's skin extraction works with the CD's exe).
- So this needs: an ISO 9660 reader (Joliet for the long names), finding the CABs in the installer (the .WISE section, or a scan for MSCF), a CAB reader with LZX decompression (a pure-Rust crate if one is sound; pin it), and the file names (from the CAB's file table, or the MSI's File table). Other editions of the CD (the 1998 original) may differ: report an unknown image clearly (and a DAT with another hash, task-064).

The user's ISO: ask the supervisor for its path; never commit the image or anything extracted from it. Tests are env-gated (e.g. RB_CD_ISO) and skip when it is unset; they check that the files read from the image equal the known hashes and that `rb recrypt --iso` and `rb standard-mod --iso` give the same bytes as the `--rb20ful` / `--rebirth-dll` forms.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb recrypt --iso reads RB20FUL.DAT (and by default the shipped mods) from the CD image, giving the same bytes as the --rb20ful form
- [ ] #2 rb standard-mod --iso reads ReBirth.dll (and Rebirth.exe if needed) from the installer inside the image, giving the same mod as the --rebirth-dll form
- [ ] #3 An unknown or different image is reported clearly; the reading code needs no Windows code; env-gated tests (RB_CD_ISO) check the extracted files' hashes
<!-- AC:END -->

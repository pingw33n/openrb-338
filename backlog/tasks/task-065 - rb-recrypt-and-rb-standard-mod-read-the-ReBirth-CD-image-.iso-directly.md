---
id: TASK-065
title: rb recrypt and rb standard-mod read the ReBirth CD image (.iso) directly
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-27 10:02'
updated_date: '2026-10-28 11:40'
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
- [x] #1 rb recrypt --iso reads RB20FUL.DAT (and by default the shipped mods) from the CD image, giving the same bytes as the --rb20ful form
- [x] #2 rb standard-mod --iso reads ReBirth.dll (and Rebirth.exe if needed) from the installer inside the image, giving the same mod as the --rebirth-dll form
- [x] #3 An unknown or different image is reported clearly; the reading code needs no Windows code; env-gated tests (RB_CD_ISO) check the extracted files' hashes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. crates/rb-cli/src/cd.rs: own small ISO 9660 reader (Joliet UCS-2BE names preferred, else primary names without ';1'; case-insensitive path lookup; SubReader = Read+Seek window over the image file; no crate is needed for ~150 lines and the common crates are unmaintained or pull unneeded deps), CdImage: rb20ful() reader, installer CABs found by scanning 'Install ReBirth RB-338.exe' for MSCF headers (checked against the cabinet size), file by name through the cab crate. Unknown image / missing file / hash mismatch give clear errors (SHA-256 of ReBirth.dll and Rebirth.exe checked).
2. rb mod recrypt --iso CD.iso (conflicts with --rb20ful; inputs optional with --iso: default input the 4 shipped mods from the CD; output then a folder), the DAT read from the image, same verify_dat.
3. rb mod pack-standard --iso CD.iso (conflicts with --rebirth-dll/--rebirth-exe): both files from the installer.
4. Env-gated tests (RB_CD_ISO): hashes of the files read, recrypt --iso == --rb20ful, pack-standard --iso == assets/Standard.rbm; unit tests with a synthetic ISO and a synthetic CAB (always run). 5. docs/rbm.md section 16, assets/README.md, original_bugs not needed.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
The user's CD image (2026-10-28): '/Users/pingw33n/Downloads/Rebirth 2.01 (2001 version).iso' (570,923,008 bytes). Read it only; never copy it into the repo or commit anything extracted from it. Command names changed since this task was written: rb recrypt is now rb mod recrypt (task-097, task-064's checks and bulk mode), rb standard-mod is now rb mod pack-standard (task-097), and the standard mod is committed and embedded (task-067: assets/Standard.rbm, built from ReBirth.dll plus the CD's Rebirth.exe for the splash; its gated reproduction test reads RB_CD_REBIRTH_EXE). With --iso, rb mod pack-standard should reproduce assets/Standard.rbm byte for byte (ReBirth.dll and Rebirth.exe both from the image), and that reproduction test can take the image instead of the extracted exe.

Branch: worktree-agent-a89369a0301602c31. Findings: the image is 2048-byte sectors, ISO 9660 with Joliet (bsdtar lists the long names). RB20FUL.DAT is at the root. The shipped mods are NOT plain files: Metallicon.rbm, MSM2.rbm, Orbit2.rbm, PBE2.rbm sit in the third CAB (file offset 0x30266C of the installer) next to Rebirth.exe and ReBirth.dll (32 files). 'Install ModPacker.exe' has a CAB of unpacked mod parts (not .rbm), not used. cab 0.6.0 (2024-04-05, pure Rust, LZX through lzxd; its unsafe is test-only code) is the CAB reader; lzxd resolves to 0.2.7 (2026-06-25, >2 weeks old).

Evidence (2026-10-28, the user's image): rb mod pack-standard --iso gives 6355b5f1...5fbe = assets/Standard.rbm (0.46 s). rb mod recrypt --iso (no input) recrypts Metallicon, MSM2, Orbit2, PBE2 (108 -> 109 chunks each, 0.98 s with the SHA-256 of the DAT), byte-identical to the recrypted-mods set. cargo clippy --all-targets -D warnings clean; cargo test --release -q: 508 passed, 0 failed, 2 ignored (awk sum over all 'test result' lines) with RB_CD_ISO, RB_RB20FUL, RB_MOD_ORIG_DIR and RB_MOD_DIR set; gated tests that ran: cd_image (the real-image hashes, shipped mods == RB_MOD_ORIG_DIR, DLL == original/ReBirth.dll, pack-standard --iso == Standard.rbm, recrypt --iso == --rb20ful form), standard_mod::the_command_reproduces_the_committed_mod (exe from RB_CD_ISO), recrypt and mod_pack gated tests. Not confirmed: the 1998 original CD (not available); the LZX path is exercised only by the gated tests (cab cannot write LZX). New deps: cab =0.6.0 and, via Cargo.lock, lzxd 0.2.7 (2026-06-25), time 0.3.55 (2026-08-01), deranged 0.5.9 (2026-10-13), num-conv 0.2.2, powerfmt 0.2.1 (2026-10-05), time-core 0.1.9: all at least 14 days old. Own ISO 9660 reader (cd.rs) instead of a crate: about 150 lines, Joliet plus primary, no crate needed. Docs: docs/rbm.md section 16, assets/README.md.

Review round 1 (2026-10-28): fixed (1) path traversal: recrypt.rs cd_jobs refuses any CAB mod name that fails rbm::pack::safe_path, before anything is written; test unsafe_mod_names_in_the_cab_write_nothing (../x.rbm, /abs/x.rbm, a/b.rbm, ..\x.rbm, C:x.rbm, CON.rbm: exit 1, nothing created inside or outside -o); (2) docs/rbm.md section 16 rewrapped to 100 columns; (3) Window::read_all no longer pre-allocates from the image's size, and (4) a truncated installer is an error (test a_truncated_installer_is_an_error); docs note that cab/lzxd are not hardened against malformed CABs (a panic, not memory unsafety). Re-ran fmt, clippy, cargo test --release -q with all gates: 510 passed, 0 failed, 2 ignored.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
rb mod recrypt --iso CD.iso and rb mod pack-standard --iso CD.iso read the ReBirth CD image in place: crates/rb-cli/src/cd.rs holds an own ISO 9660 reader (Joliet, else primary) and finds the installer's CAB archives by an MSCF scan, read with the pinned cab =0.6.0 crate (LZX via lzxd). recrypt --iso reads RB20FUL.DAT from the image (SHA-256 checked) and, with no input mod, recrypts the four shipped mods in the installer's third CAB (-o a folder; mod names from the CAB must pass rbm::pack::safe_path or the run fails before writing); pack-standard --iso unpacks ReBirth.dll and Rebirth.exe in memory, checks their SHA-256, and gives assets/Standard.rbm byte for byte. Unknown images, truncated reads, missing files and hash mismatches have clear errors; options conflict with --rb20ful / --rebirth-dll / --rebirth-exe (exit 2). Tests: crates/rb-cli/tests/cd_image.rs (12 tests; synthetic ISO and CAB always on, real image gated on RB_CD_ISO); standard_mod.rs reproduction test accepts RB_CD_ISO. cargo test --release -q: 510 passed, 0 failed with all gates set; clippy clean. Docs: docs/rbm.md section 16, assets/README.md. Branch worktree-agent-a89369a0301602c31, one commit (amended after review round 1).
<!-- SECTION:FINAL_SUMMARY:END -->

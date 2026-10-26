---
id: TASK-001
title: Install ReBirth 2.01 on the test machine
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-26 09:20'
labels:
  - testmachine
milestone: m-0
dependencies: []
priority: high
type: chore
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The reference renders come from the original ReBirth RB-338 2.01 running on the Windows test machine (reached with ~/dev/dcre/common/tools/rcs_client.py, see the agent brief doc-001). Install it there so that the agents can drive it. The app checks its registration ("The ReBirth Engine is online but not registred" in Rebirth.exe), so this needs the user's installer and registration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ReBirth 2.01 is installed on the test machine, registered, and starts to its main window
- [x] #2 Rebirth.exe and ReBirth.dll on the machine have the same SHA-256 as original/Rebirth.exe and original/ReBirth.dll (or the difference is noted in the task)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Installed by the user in C:\ReBirth RB-338, outside the rcs server's root (C:\re): rcs ls/get/put/--cwd refuse paths there (HTTP 403), but exec of cmd.exe, certutil and tasklist with absolute paths works.

Contents (cmd /c dir /s): Rebirth.exe, ReBirth.dll, ReBirth.hlp/.cnt, ReWire.dll_, ReWire.reg, ReadMe.rtf, ReBirth manual.pdf; Default Songs/ (ReBirth 1.0 Default Song.rbs, Silent Default Song.rbs, Read Me.txt); Demo Songs/ (16 .rbs); Mods/ (Metallicon.rbm, MSM2.rbm, Orbit2.rbm, PBE2.rbm).

Hashes (certutil -hashfile SHA256): Rebirth.exe 6a651e8164b8444a5f0639cb6a2fef2aa43c5500bd9c5288627c2030b047f876, ReBirth.dll 4c3a6441a2cd50f797d99b1c5b6cf0aaf198c7ad8ca2648196b530c2e3e58b12: both equal original/.

Start check (holding all 6 permits of redc-win-run, owner rb-task-001): rcs exec cmd.exe /c start "" /D "C:\ReBirth RB-338" "C:\ReBirth RB-338\Rebirth.exe". The app first shows a modal "Failed opening MIDI input. No MIDI will be available..." box (window title "ReBirth RB-338", not in the foreground); rcs input --window "ReBirth RB-338" key:enter dismisses it, then the main window "Untitled song" appears with no registration message. Closed with taskkill /PID; permits released. The MIDI box is a preference for task-002 (MIDI input port setting) to make startup unattended.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
ReBirth 2.01 is installed in C:\ReBirth RB-338 (not under the rcs root as first planned), with exe and DLL identical to original/, the shipped songs and four mods. It starts to its main window, registered, after a MIDI-input warning box that task-002 should turn off. Checked 2026-10-26.
<!-- SECTION:FINAL_SUMMARY:END -->

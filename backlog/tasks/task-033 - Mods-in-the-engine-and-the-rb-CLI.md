---
id: TASK-033
title: Mods in the engine and the rb CLI
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-26 09:06'
updated_date: '2026-10-27 05:15'
labels:
  - rbm
  - cli
milestone: m-2
dependencies:
  - TASK-032
  - TASK-031
  - TASK-029
priority: medium
type: feature
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Use mods in the engine and the CLI: the song's GLOB mod name ("Standard ReBirth" means the built-in kit) selects a mod found in `--mod-dir DIR` (by the INFO name or file name, as the original resolves it), `--mod FILE` overrides it, and a missing mod is an error unless `--standard-kit` is given (the original offers Download/Standard/Cancel). The golden suite covers the mod songs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rb render uses the song's mod by name from --mod-dir, --mod overrides it, and a missing mod behaves as described
- [ ] #2 Every mod song of the corpus passes the golden suite
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Supervisor 2026-10-27, per task-053: the rb CLI takes --rb20ful PATH (and RB_RB20FUL, then a default location, documented in --help, the same search style as --rebirth-dll), and errors clearly when a song names a non-standard mod and no RB20FUL.DAT or .rbm is found.

Supervisor 2026-10-27, from task-032's review: crates/rb-engine/src/samples/aiff.rs tests the COMM rate for zero on its raw bytes, while the DLL tests the truncated integer after fistp (sub_100016F4). A rate that is non-zero but truncates to 0 (below 1 Hz) is accepted by rb and rejected by the DLL. Align it here if cheap (a unit test with such a rate).
<!-- SECTION:NOTES:END -->

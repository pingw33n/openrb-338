---
id: TASK-054
title: >-
  Decide: computer-keyboard mapping of rb's UI (original virtual keys or
  physical positions)
status: Done
assignee:
  - '@pingw33n'
created_date: '2026-10-27 07:04'
updated_date: '2026-10-27 07:30'
labels:
  - decision
  - ui
milestone: m-3
dependencies: []
references:
  - docs/ui_spec.md
  - docs/original_bugs.md
priority: medium
type: spike
ordinal: 44500
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: task-041 found how the original maps the computer keyboard (docs/ui_spec.md section 10). Rebirth.exe translates Windows virtual keys to its own key codes (sub_492312); the letters follow the US layout, but the TB-303 Note/Pause key and the drum AC key are VK_OEM_PLUS, which is '=' on a US keyboard, while the manual documents '-' (OB-13 in docs/original_bugs.md). The manual also says the keys are physical positions ('regardless of which letters are printed'), which is true on the Mac but only partly on the PC, where virtual keys move with the layout. rb's UI (task-045) has to choose; the UI does not have to match the original exactly (root AGENTS.md).

Options:
A. Reproduce the original: Windows-style virtual keys, Note/Pause and AC on '=' (VK_OEM_PLUS), '-' unmapped.
B. Physical key positions (scancodes) of a US keyboard as in the manual's pictures: Note/Pause and AC on the key right of '0' ('-' on US), every other key at the US position whatever the layout.
C. B plus '=' as an alias for Note/Pause and AC, so users of the original's habit also work.

Recommendation: C. It matches the manual's documented and pictured keys on every layout, keeps the original's '=' working, and costs one extra binding. The transport numpad keys follow the program, not the manual (numpad 4 rewind, 5 fast forward), in every option.

task-045 should depend on this decision.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The user's choice is recorded in the final summary with the date
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Decided by @pingw33n on 2026-10-27: option C. rb's UI maps keys by physical position (scancode) of a US keyboard as pictured in the manual, whatever the layout: the TB-303 Note/Pause key and the drum AC key are on the key right of '0' ('-' on US). '=' is an alias for both, so the original's habit (VK_OEM_PLUS, OB-13) also works. The transport numpad keys follow the program (numpad 4 rewind, 5 fast forward).
<!-- SECTION:FINAL_SUMMARY:END -->

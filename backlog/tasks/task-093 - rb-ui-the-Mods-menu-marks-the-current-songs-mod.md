---
id: TASK-093
title: 'rb-ui: the Mods menu marks the current song''s mod'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 05:51'
updated_date: '2026-10-28 06:45'
labels:
  - ui
  - rbm
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/menu.rs
  - docs/ui_spec.md
  - docs/rbm.md
priority: low
type: feature
ordinal: 113000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that the Mods menu show the currently selected mod. The menu lists one item per mod in the mod folder (docs/ui_spec.md sections 12 and 18; the original's items 5000 + i, sub_40C163) but nothing marks the one the current song uses. The original checks the active song's mod in this list (check it in sub_40C163 or the menu-update code around it and record the address in docs/ui_spec.md; if it does not, say so there; the user's requirement holds either way).

What: show the current song's mod checked (a check mark or a selected radio) in the Mods menu list; the standard mod too if it is listed. Match by the same identity the mod change uses (the name or file the song refers to). When the song refers to a mod that is not in the folder, nothing is checked, and the menu may show the missing mod's name disabled.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Mods menu checks the mod the current song uses, and the check follows a mod change and a song open
- [x] #2 With a song whose mod is missing from the folder no item is checked
- [x] #3 A test covers the checked item
- [x] #4 docs/ui_spec.md section 18 describes it (with the original's behavior and address if found); cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Read sub_40B977/sub_40C163 to see how the original checks the Mods items. 2. App::mod_checks compares the song's mod name with the INFO names; menu draws checks from it. 3. Test, ui_spec section 18.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-afa48ad8d4eca0362

IDA (Rebirth.exe, read-only copy): sub_40C163 only handles the choice (5000+i -> sub_42D80D, dword_4DA788). The check is in the menu update sub_40B977: per mod object (sub_415578(i), name at +1011) the item 5000+i gets checked flag [ebp-0DDh]=1 (0x40BF14) when a song window is active and its mod object (sub_426F83, 0x40BE23) equals the item's (cmp 0x40BF0C). rb compares names (App::mod_checks). Test the_mods_menu_checks_the_songs_mod: start, use_kit, song open with a named mod, song open with a missing mod. cargo test --release: 456 pass. Also fixed stale comment app.rs keyboard (Ctrl or Cmd).

Review round 1: (1) mod_checks now matches the resolved kit (Kit::Mod(p) -> entry with path p; Standard -> the Standard item only under the name Standard ReBirth); tests added for the file-name fallback (mod_name 'pbe2') and a copy of PBE2.rbm outside the folder (nothing checked). (2) the test RAN, not skipped, with RB_MOD_DIR=.../scratchpad/recrypted-mods and RB_STANDARD_MOD=/Users/pingw33n/dev/rb/target/Standard.rbm (1 passed, no 'skipped' line). (3) Disassembly (IDA disasm, not only Hex-Rays) read: 0x40BE9A..0x40BF3C: 'mov eax,[ebp+var_C4]; add eax,1388h' (item id 5000+i, 0x40BEDA), 'call sub_415578' (0x40BEA8) then 'add ecx,3F3h' (name at +1011, 0x40BEB9), 'cmp edx,[ebp+var_134]; jnz' (0x40BF0C) then 'mov [ebp+var_DD],1' (0x40BF14), after 'test ecx,ecx; jz' on the song-active byte var_12C. NOT read: sub_426F83's body (only its call at 0x40BE23 and its prologue); marked [G] in the doc that it returns the song's mod object. (4) the menu borrows the list, no clones. (5) table row shortened. fmt, clippy clean, cargo test --release 456 pass.

Review round 2: the final summary was stale (name matching, [C] for sub_426F83); rewritten to match the code (resolved-kit matching) and the docs ([G] for sub_426F83's return).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The Mods menu checks the current song's mod by the resolved kit (App::mod_checks, crates/rb-ui/src/menu.rs): Kit::Mod(p) checks the list entry whose file is p, so a mod a song names by file name (mods::find_in_dir fallback) is checked; a copy of a folder mod from outside the folder (Choose file...) checks nothing even when its INFO name equals a folder mod's; Kit::Standard checks 'Standard ReBirth' only under that name, so a song whose mod is missing (built-in kit under another name) checks nothing. Computed from the document each frame, so it follows song open and mod change. The menu borrows the list (no clones). Original: sub_40B977 sets the checked flag of item 5000+i (0x40BEDA) when a song is active and the item's mod object (name at +1011, 0x40BEB9) equals the song's (cmp 0x40BF0C, flag 0x40BF14) [C, read in the disassembly]; that sub_426F83 (called at 0x40BE23) returns the song's mod object is [G], its body was not read. Recorded in docs/ui_spec.md section 18. Test the_mods_menu_checks_the_songs_mod (start, mod change, song open, missing mod, file-name fallback, outside-folder copy) ran with RB_MOD_DIR set. fmt, clippy -D warnings clean, cargo test --release 456 pass. Branch worktree-agent-afa48ad8d4eca0362.
<!-- SECTION:FINAL_SUMMARY:END -->

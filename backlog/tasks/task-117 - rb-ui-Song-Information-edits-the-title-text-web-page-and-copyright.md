---
id: TASK-117
title: 'rb-ui: Song Information edits the title, text, web page and copyright'
status: Done
assignee:
  - '@agent-complex'
created_date: '2026-10-28 12:57'
updated_date: '2026-10-29 12:05'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - docs/RBS42.txt
  - crates/orb-ui/src/info.rs
  - crates/orb-ui/src/document.rs
priority: low
type: feature
ordinal: 137000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that some fields of File > Info (the Song Information window, crates/orb-ui/src/info.rs) be editable: the title, the text, the web page and the copyright. These editable fields are grouped together, come right after the file and folder rows, and are visually separated from the rest of the window's facts, which stay read-only. The task was created unassigned at the user's request.

The fields (docs/RBS42.txt; the rbs crate; the INFO-like chunks of a song):
- Title: the song info chunk's 'Window title', ZSTRING(41), so at most 40 bytes.
- Text: the song info chunk's 'More text', ZSTRING(201), at most 200 bytes. 0x0d is the line break in the file; the editor shows real line breaks, and \r\n and \n are stored as 0x0d.
- Web page: the song info chunk's 'Web page', ZSTRING(101), at most 100 bytes.
- Copyright: the version info chunk's 'Copyright', ZSTRING(129), at most 128 bytes.

What:
- Single-line text edits for the title, web page and copyright, and a multi-line edit for the text.
- The strings are stored as Latin-1 (the rbs reader's get_z). The editor accepts only characters Latin-1 can encode, and limits input to each field's byte length; show the remaining count or refuse longer input. Say what happens to a pasted character outside Latin-1: refused, or replaced with '?'. Choose one and document it.
- An edit changes the open document (rbs::Song in the Document, the source that document::compose saves from) and marks it modified, so the title bar shows the '*', the unsaved-changes prompt applies, and Save writes the new values. Undo is not required.
- The web page row keeps its link behavior (info::web_url) in a read view, or shows an 'open' link next to the edit when the text is a valid link. Choose one; editing must not be blocked by the link.
- The window keeps following the current song: opening another song refreshes the fields. An edit in progress applies on every change, not on window close.
- Tests:
  - an edit of each field changes the saved file's bytes in the right chunk and offset, read back with rbs::Song::parse;
  - the byte limits;
  - Latin-1 handling;
  - the modified flag;
  - 0x0d line breaks round-trip.
- Update docs/ui_spec.md section 18 (File > Info).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Title, text, web page and copyright are editable in Song Information, grouped after the file and folder rows and separated from the read-only facts
- [x] #2 Edits mark the song modified and Save writes them (round-trip test through rbs::Song::parse); byte limits and Latin-1 are enforced; 0x0d line breaks round-trip
- [x] #3 docs/ui_spec.md section 18 updated; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. info.rs: an InfoField enum (title, text, web page, copyright) with its chunk field and byte limit (40, 400, 100, 128); the text's limit counts a line break as 2 (CR LF, as the original holds it, sub_465D8D), so the original reads every text orb writes whole.
2. Sanitizing: CR LF and CR become a line break (the text) or a space (one-line fields); a character outside Latin-1, or a control character other than the text's line break, becomes '?' (choice: replace, so a paste keeps the rest). An edit past the limit is refused: TextEdit char_limit truncates typing and pastes at the limit, and a check after the edit keeps the old value when it still does not fit.
3. set_field writes doc.source (USRI title/text with 0x0d, web page; HEAD copyright) and the 1.x facts when present, and sets doc.modified. The window rebuilds the edit buffers from the document each frame, so it follows another open; an edit applies on every change.
4. Layout: File/Folder grid, separator, the four edits with a remaining-bytes count, an 'open' link next to the web page edit when info::web_url accepts it, a short note under the copyright (ReBirth ignores it on open, its Save writes its own), separator, the read-only facts.
5. The panel takes no keys while one of the fields has the keyboard focus (App::show).
6. Unit tests in info.rs (sanitize, limits, Latin-1, CR round trip, modified flag) and harness tests in tests/menu.rs (edit each field, save, rbs::Song::parse, chunk byte offsets; typing does not reach the panel).
7. docs: ui_spec.md 18 (File > Info), song_load.md (the copyright and the USRI limits, with addresses), RBS42 errata row in song_load section 10 if needed.
8. fmt, clippy, tests, screenshot.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Check before shipping the copyright edit: RBS42.txt says the version chunk's copyright is set to Propellerhead's string. Find out whether the original ReBirth checks it when it opens a song: search Rebirth.exe / ReBirth.dll in IDA for the string and its readers, and record the addresses and [C]/[G] in docs/song_load.md. If the original refuses or warns on another copyright, say so in the window next to the field, or ask the supervisor before allowing the edit.

Branch: worktree-agent-a102c41953df98f10

Copyright check (2026-10-29). The idalib MCP tools did not load in this session (ToolSearch 'idalib idb' found none, also after the supervisor's note that IDA was started), so the check was done on the bytes of original/Rebirth.exe and ReBirth.dll (read in place, read only) with a Python scan (scratchpad xref.py: every string, and every place in the file that holds its address as 4 bytes) and llvm-objdump disassembly.
- Rebirth.exe holds the copyright at 0x4BC010 ('(c)1997 Propellerhead Software, all rights reserved', no period), 0x4BB8C8 and 0x4BC058 (with a period), plus two copies in .rsrc (0x5694A9, 0x56CF75: version resources, no reference). ReBirth.dll has none.
- The six references to the three .rdata strings are all the copyright argument of the HEAD object constructor sub_465AF6 (magic, CVersion, copyright; it strcpys the string to +0x10): 0x4109D1 and 0x41110E (magic 0x3B445B64, the RBRT/QMAP file), 0x4380AE (the RBS writer sub_438025: magic '[T[T', CVersion(4,2) 0x4DB918, string 0x4BC010), 0x43AA8A (magic from 0x4CF1A4, string 0x4BC058), 0x41C22E and 0x41CAC2 (magic 0xFFEFEFE5, string 0x4BB8C8). No reference is a compare.
- HEAD serialize sub_465B9D writes the copyright at +9 with sub_465D0E(max 0x80) (0x465BEE); HEAD parse sub_465BFC reads it with sub_465D8D(max 0x80) (0x465C39) into the object's +0x10.
- The song open reads HEAD twice, both through sub_465AA2 (construct + parse): the peek sub_4349C7 at 0x434B26 (object at ebp-0x170, copyright at ebp-0x160) checks the magic at 0x434B2F and copies the version bytes (ebp-0x168..-0x164) at 0x434B77..0x434BB0, then destroys it (sub_465B87, 0x434BBD); sub_434D05 at 0x434ECC (object at ebp-0x188, copyright at ebp-0x178) checks the magic at 0x434ED5 and the version at 0x434F11..0x434F49 (sub_47DB0B), then destroys it at 0x434F93. Neither function reads ebp-0x160 / ebp-0x178 (grep of the disassembly of 0x4349C7..0x436000: 0 hits).
- So the original neither checks nor keeps a song's copyright on open, and its Save always writes its own string (0x4380AE). A changed copyright opens without a message.
- [C] for the two read paths and the writer (instruction by instruction in llvm-objdump). [G] that no other code reads it: without IDA an indirect reference (a pointer computed at run time) cannot be ruled out, though no 4-byte copy of any of the addresses exists elsewhere in the file.

Field limits, from the exe (the task's 200 for the text is RBS42.txt's, which the errata in docs/song_load.md section 10 already corrects to ZSTRING(401)): the USRI reader sub_434955 copies the title with max 0x28 (0x434960), the text with max 0x190 = 400 (0x434975) and the web page with max 0x64 (0x434993) through sub_465D8D, which writes a 0x0a after each 0x0d (0x465DDA..0x465DED) and counts it; the writer at 0x4348E4/0x4348F9/0x434911 uses sub_465D0E(max 0x28/0x190/0x64), which drops 0x0a bytes (0x465D55). So the original holds the text as CR LF in at most 400 bytes; a stored text whose bytes plus line breaks pass 400 loses its tail when the original opens it.

Text limit: 400, a line break counting 2 (the exe's 0x190 with CR LF in memory), not the task's 200 (RBS42.txt's ZSTRING(201), wrong per the errata of docs/song_load.md 10 and the reader at 0x434975). Choices: a pasted character outside Latin-1 (or a control character but the text's line break; a tab or a one-line field's line break is a space) becomes '?'; typing and pasting stop at the limit (egui char_limit), and set_field refuses an edit that still passes it unless it shortens a longer value. Web page: an 'open' link next to the edit when info::web_url accepts the text. Copyright: allowed, with a note under the field (the original ignores it on open and writes its own on Save; docs/song_load.md 13). The panel takes no keys while a text edit has the focus (App::show, ctx.text_edit_focused); checked by the harness test, which fails with the guard disabled (Space plays).

Tests run: cargo +nightly fmt --all --check clean; cargo clippy --all-targets -- -D warnings clean; debug cargo test -p orb-ui all pass (lib 65 incl. 4 new info tests, menu 49 incl. the new harness test); cargo test --release -q --no-fail-fast: 65 suites, 608 passed, 0 failed (ORB_CD_ISO and the third-party folders not set, so their tests skip).
Hidden option --info (main.rs, docs/building.md table) opens the Song Information window at start, as --about does, for the picture. Screenshot: target/debug/OpenRB-338 --size 624x510 --info --screenshot info.png tests/songs/shipped/Calm.rbs (scratchpad task117/info.png): the File/Folder rows, a line, the four edits with '33 left', '342 left', '81 left' and 'open', '77 left' and the note, a line, the read-only facts.
New defect of the original: OB-21 in docs/original_bugs.md (the text's line breaks count twice against 400 on open).

Review round 1 (of 5ddef7a, send back), fixed:
1. Cmd/Ctrl+F and +G were skipped while a text field had the focus: App::show (crates/orb-ui/src/app.rs) now consumes them before the guard, as NEW/OPEN/SAVE are; only the panel's keyboard() is under the guard (holds cleared), focus_click runs either way.
2. docs/ui_spec.md 18 names the shortcuts that still work (Cmd/Ctrl+N, O, S, F, G), Escape, and that the panel takes its keys again once the field lets go.
3. New harness test the_menu_shortcuts_and_escape_work_while_a_field_is_focused (tests/menu.rs): with the title focused Cmd+F/G switch the mode and type nothing, Escape closes the window, then Space plays; with the text focused Space does not play, after surrender_focus it plays (window open).
4. Nits: ui_spec says C0 and C1 (U+0080..U+009F) controls are control characters and become '?'. The count is in set_field's units: info::left_text, the text's count reads 'N left (a line break takes 2)' and at one byte left '1 left, too few for a line break (2)', so the Enter egui lets through and set_field refuses is explained; unit-tested at cost 399 (break refused, a letter taken).
Tests: fmt --check and clippy clean; debug cargo test -p orb-ui 218 passed, 0 failed; cargo test --release -q --no-fail-fast 65 suites, 609 passed, 0 failed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
File > Info edits the song information (branch worktree-agent-a102c41953df98f10, one commit, amended after review round 1).
- crates/orb-ui/src/info.rs: info::Field (title, text, web page, copyright), cost, sanitize, field_value, set_field and left_text; the window has three grids (file, the four text edits with the bytes left, the read-only facts) with lines between. An edit applies on every change to doc.source (USRI +0/+41/+442, HEAD +9; a 1.x song's facts follow) and sets doc.modified. Limits 40/400/100/128 bytes from the exe's readers and writers; a line break counts two in the text (the original holds CR LF in 400), and the text's count says so. Typing and pasting stop at the limit (char_limit); set_field refuses what still passes it unless it shortens a longer value. Outside Latin-1 or a control character (C0 or C1) becomes '?' (a tab, or a one-line field's line break, a space); CR LF/CR/LF are stored as 0x0d. Web page: an 'open' link next to the edit when web_url accepts it. Copyright: editable, with a note under it.
- crates/orb-ui/src/app.rs: no panel keys while a text edit has the focus; Cmd/Ctrl+N, O, S, F, G still work.
- crates/orb-ui/src/main.rs: hidden --info (docs/building.md).
- Copyright finding: the original's song open reads HEAD twice (0x434B26, 0x434ECC) and checks only the magic and the version, never the copyright; its Save writes its own string (0x4380AE). [C] for these paths from llvm-objdump; [G] that no other code reads it (idalib was not available, so no IDA cross references). docs/song_load.md 13, OB-21 in docs/original_bugs.md, docs/ui_spec.md 18.
- Tests: 4 unit tests in info.rs, 2 harness tests in tests/menu.rs (each field typed and pasted, Save read back with rbs::Song::parse, another open refreshes; the shortcuts, Escape and Space around the focus). fmt, clippy clean; debug orb-ui 218 pass; release workspace 609 pass.
<!-- SECTION:FINAL_SUMMARY:END -->

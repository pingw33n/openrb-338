---
id: TASK-117
title: 'rb-ui: Song Information edits the title, text, web page and copyright'
status: To Do
assignee: []
created_date: '2026-10-28 12:57'
updated_date: '2026-10-29 08:28'
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
- [ ] #1 Title, text, web page and copyright are editable in Song Information, grouped after the file and folder rows and separated from the read-only facts
- [ ] #2 Edits mark the song modified and Save writes them (round-trip test through rbs::Song::parse); byte limits and Latin-1 are enforced; 0x0d line breaks round-trip
- [ ] #3 docs/ui_spec.md section 18 updated; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Check before shipping the copyright edit: RBS42.txt says the version chunk's copyright is set to Propellerhead's string. Find out whether the original ReBirth checks it when it opens a song: search Rebirth.exe / ReBirth.dll in IDA for the string and its readers, and record the addresses and [C]/[G] in docs/song_load.md. If the original refuses or warns on another copyright, say so in the window next to the field, or ask the supervisor before allowing the edit.
<!-- SECTION:NOTES:END -->

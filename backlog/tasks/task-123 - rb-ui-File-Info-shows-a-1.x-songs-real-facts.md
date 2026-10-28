---
id: TASK-123
title: rb-ui File > Info shows a 1.x song's real facts
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 14:36'
updated_date: '2026-10-28 14:40'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/document.rs
  - crates/rb-cli/src/info.rs
priority: low
type: bug
ordinal: 143000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
rb-ui's document for a ReBirth 1.x song uses rbs::Song::empty() as its source (crates/rb-ui/src/document.rs source_of_1x), so File > Info shows the empty song's format version ("4.2"), copyright, tempo, loop, title and shuffle instead of the file's (rb-ui/src/info.rs SongInfo::of reads that stand-in). rb info (task-122) reads them with rb_cli::info::SongFacts::of_1x(&Song1x, &EngineSong). Make the UI take all of a 1.x song's facts from SongFacts, as rb info does (keep the facts on the document, or fill the source's head, glob and usri from them), with a test on a 1.x song built as crates/rb-cli/tests/info.rs does.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test covers it; cargo test, clippy and nightly fmt --check pass
- [ ] #2 File > Info of a 1.x song shows the format version, copyright, tempo, loop, mode, shuffle, synth sound, title, text and web page rb info prints for it
<!-- AC:END -->

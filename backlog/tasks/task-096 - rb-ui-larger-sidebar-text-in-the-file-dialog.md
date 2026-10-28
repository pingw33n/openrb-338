---
id: TASK-096
title: 'rb-ui: larger sidebar text in the file dialog'
status: Done
assignee:
  - '@supervisor'
created_date: '2026-10-28 07:16'
updated_date: '2026-10-28 07:16'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/browser.rs
priority: low
type: enhancement
ordinal: 116000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for larger text in the file dialog's sidebar: the 'Places' and 'Recent folders' headings and the 'Use as songs folder' / 'Use as mods folder' buttons were in egui's Small style (about 9 pt).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The headings, 'none yet' and the two buttons use the normal body/button size
- [x] #2 The user reviewed it in the running app
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
crates/rb-ui/src/browser.rs: dropped .small() from the 'Places', 'Recent folders' and 'none yet' labels and replaced the small-text 'Use as songs folder' / 'Use as mods folder' buttons with ui.button, so they use egui's normal body/button size (about 12.5 pt instead of 9). The user reviewed it in the running app and asked to commit on 2026-10-28. cargo clippy -p rb-ui clean; cargo test --release -p rb-ui 136 pass.
<!-- SECTION:FINAL_SUMMARY:END -->

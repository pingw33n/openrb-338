---
id: TASK-115
title: 'rb-ui: UI clean-up pass with the Fable model (interactive with the user)'
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-10-28 11:57'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui-art/src/parts.rs
  - crates/rb-ui-art/src/layout.rs
  - assets/ui/README.md
  - docs/ui_spec.md
priority: low
type: enhancement
ordinal: 135000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user wants to try cleaning up the UI's look with the Fable model (claude-fable-5-1), in an interactive design session the user drives (2026-10-28). Agents do not start it on their own.

Scope, to be set in the session: the panel art and layout (crates/rb-ui-art: parts.rs, layout.rs, font.rs; regenerated assets/ui with `cargo run --release -p rb-ui-art -- gen`), and the egui windows and menus (the file browser, Song Information, Mods > Info, About, Keyboard Shortcuts, prompts). Examples: spacing and alignment, label sizes and colors, consistency between sections, window styling.

Constraints:
- The panel's controls keep their ids and behavior: assets/ui/layout.tsv drives the app, and the wired and interaction tests must still pass.
- Each change is reviewed by the user in the running app, with screenshots via OpenRB-338 --screenshot.
- Write the design decisions into assets/ui/README.md and docs/ui_spec.md.
- For the session, run Claude Code on Fable (/model, or claude --model claude-fable-5-1). Compare its suggestions against the current look before applying them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The user ran the session with the Fable model and accepted the changes in the running app
- [ ] #2 The art is regenerated, the wired/interaction tests pass, and assets/ui/README.md and docs/ui_spec.md record what changed
<!-- AC:END -->

---
id: TASK-116
title: 'rb-ui: the About window shows the panel''s wordmark logo'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 12:13'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui-art/src/parts.rs
  - crates/rb-ui/src/about.rs
priority: low
type: enhancement
ordinal: 136000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) that Help > About show the same logo as the main window's top-right corner, instead of the app icon it shows now (task-102, crates/rb-ui/src/about.rs, the app_icon part).

Today the wordmark is drawn inside the bg_transport part (crates/rb-ui-art/src/parts.rs, transport_bg, the group at (458, 1)):
- 'OPENRB-' in INK and '338' in ORANGE, cap 14;
- an orange underline 118 x 1.5;
- 'BASS LINE AND DRUM MACHINE' at cap 3.4.

What:
- Move the wordmark out of bg_transport into its own part, for example `logo`, about 134 x 48 like the logo click area (layout::on_logo). Lay it out on the panel at the same place, so the main window looks exactly as before (compare screenshots before and after).
- The About window draws that part, scaled up crisply: it is SVG, so render it at the window's pixel density.
- Regenerate assets/ui (`cargo run --release -p rb-ui-art -- gen`) and update assets/ui/README.md.
- Keep the part simple to split later: task-114 animates its pieces ('OPEN', 'RB-338', the underline, the subtitle) in the panel and in About. If cheap, emit the pieces as separate parts now (logo_open, logo_rb338, logo_line, logo_sub), with the composite or a placement table, so task-114 can position them.
- The app icon (app_icon) stays the window, Dock and package icon.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The About window shows the wordmark logo (the same art as the panel's top-right), crisp at any scale; the main window looks identical to before (screenshot compare)
- [ ] #2 The wordmark is its own part (or pieces) laid out on the panel; assets/ui regenerated and README updated; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

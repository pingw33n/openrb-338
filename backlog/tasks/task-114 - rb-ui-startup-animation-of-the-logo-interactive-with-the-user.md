---
id: TASK-114
title: 'rb-ui: startup animation of the logo (interactive with the user)'
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-10-28 11:15'
updated_date: '2026-10-28 12:13'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-116
references:
  - crates/rb-ui-art/src/parts.rs
  - crates/rb-ui/src/app.rs
priority: low
type: feature
ordinal: 134000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user wants a simple animation of the logo when the app starts (2026-10-28). The user does this task interactively with an agent: agents do not start it on their own; it is worked in a session the user drives, reviewing the look as it goes.

Today the wordmark is drawn into the bg_transport part (crates/rb-ui-art/src/parts.rs, transport_bg, the group at (458, 1)):
- 'OPENRB-' in INK and '338' in ORANGE, cap 14, at y 17;
- an orange underline rect at (8, 28), 118 x 1.5;
- 'BASS LINE AND DRUM MACHINE' at y 37, cap 3.4, dim.
layout::on_logo(474..608, 1..49) is its click area (the logo click plays the mod's startup sound).

The animation, as the user described it:
1. At start only 'RB-338' is visible, with the underline sized to that text.
2. Over 1-2 s, 'OPEN' slides in from the right, moving left together with the underline (which grows to the full wordmark) to its final position.
3. When it is in place, 'BASS LINE AND DRUM MACHINE' fades in over 0.5 s.

Notes for the implementer:
- The wordmark parts must become dynamic: either separate SVG parts (the 'OPEN', 'RB-338', underline and subtitle pieces) painted by the app at animated positions and opacity, or text drawn by egui. The panel's stroke font (rb-ui-art font.rs) favors separate parts so the look stays identical at rest.
- The final frame must match today's wordmark exactly, and bg_transport keeps no logo of its own.
- Use egui's time with request_repaint while it runs. An ease-out curve is suggested; the exact duration (1-2 s) and easing are the user's call during the session.
- It plays once per app start, not on New, Open or a mod change, and it does not block input. A click on the logo still plays the startup sound.
- --screenshot must capture the final state (skip the animation or wait for it), and the headless tests must not depend on it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 At start the logo shows RB-338 with a matching underline, OPEN slides in with the underline to the final position in 1-2 s, then the subtitle fades in over 0.5 s; the final state equals today's wordmark
- [ ] #2 The user approved the look in the running app
- [ ] #3 On the panel the animation runs once per start without blocking input; in Help > About it runs every time the window opens; --screenshot and the tests see the final state
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Addition from the user (2026-10-28): the animation also plays in the Help > About window, every time the window opens (not only once per app start as on the panel). task-116 makes the About window show the panel's wordmark as its own part (or pieces), which this task animates in both places.
<!-- SECTION:NOTES:END -->

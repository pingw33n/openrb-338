---
id: TASK-114
title: 'rb-ui: startup animation of the logo (interactive with the user)'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 11:15'
updated_date: '2026-10-29 10:39'
labels:
  - ui
milestone: m-3
dependencies:
  - TASK-115
references:
  - crates/orb-ui-art/src/parts.rs
  - crates/orb-ui/src/app.rs
priority: low
type: feature
ordinal: 134000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user wants a simple animation of the logo when the app starts (2026-10-28). The user does this task interactively with an agent: agents do not start it on their own; it is worked in a session the user drives, reviewing the look as it goes.

Today the wordmark is drawn into the bg_transport part (crates/orb-ui-art/src/parts.rs, transport_bg, the group at (458, 1)):
- 'OPENRB-' in INK and '338' in ORANGE, cap 14, at y 17;
- an orange underline rect at (8, 28), 118 x 1.5;
- 'BASS LINE AND DRUM MACHINE' at y 37, cap 3.4, dim.
layout::on_logo(474..608, 1..49) is its click area (the logo click plays the mod's startup sound).

The animation, as the user described it:
1. At start only 'RB-338' is visible, with the underline sized to that text.
2. Over 1-2 s, 'OPEN' slides in from the right, moving left together with the underline (which grows to the full wordmark) to its final position.
3. When it is in place, 'BASS LINE AND DRUM MACHINE' fades in over 0.5 s.

Notes for the implementer:
- The wordmark parts must become dynamic: either separate SVG parts (the 'OPEN', 'RB-338', underline and subtitle pieces) painted by the app at animated positions and opacity, or text drawn by egui. The panel's stroke font (orb-ui-art font.rs) favors separate parts so the look stays identical at rest.
- The final frame must match today's wordmark exactly, and bg_transport keeps no logo of its own.
- Use egui's time with request_repaint while it runs. An ease-out curve is suggested; the exact duration (1-2 s) and easing are the user's call during the session.
- It plays once per app start, not on New, Open or a mod change, and it does not block input. A click on the logo still plays the startup sound.
- --screenshot must capture the final state (skip the animation or wait for it), and the headless tests must not depend on it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 At start the logo shows RB-338 with a matching underline, OPEN slides in with the underline to the final position in 1-2 s, then the subtitle fades in over 0.5 s; the final state equals today's wordmark
- [x] #2 The user approved the look in the running app
- [x] #3 On the panel the animation runs once per start without blocking input; in Help > About it runs every time the window opens; --screenshot and the tests see the final state
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. New crates/orb-ui/src/logo_anim.rs: timing constants in one place, Frame::at(t) curve, paint_logo (pieces of LOGO_PIECES, OPEN clipped left of the R, tint for fades; the final state paints the whole logo part so it equals the static wordmark), tests for t=0, midpoints, end and the final state. 2. App: logo_anim flag (off in App::new so headless tests and --screenshot see the final state; main.rs sets it with one line), panel start time once per app start, About start time at each open, request_repaint only while running, icon fade in About. 3. Hidden dev freeze: env ORB_LOGO_T=seconds, used for screenshots at chosen moments. 4. Render frames to PNG and compare with the mockup frames. 5. fmt, clippy, tests, art gen, commit; AC #2 left for the user.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Addition from the user (2026-10-28): the animation also plays in the Help > About window, every time the window opens (not only once per app start as on the panel). task-116 makes the About window show the panel's wordmark as its own part (or pieces), which this task animates in both places.

User, 2026-10-29: the animation below was approved in the mockup session and replaces the earlier description's 'underline grows'. It is to be implemented by an agent after task-115, which makes the four-segment underline and the About layout. Approved look: mockups/anim_bar_a.gif and mockups/anim_about.gif (untracked, deleted after 114 and 115 merge). The generator mockups/gen/src/main.rs has anim_at and wordmark_anim. The task's existing notes stand: once per start on the panel, every time About opens, --screenshot, and the tests.

## Animation (task-114)

Timeline in seconds, 2.0 s total, ease-out cubic on the slide; `anim_bar_a.gif` and
`anim_about.gif` are the approved look:
- 0.0..1.0: OPEN slides left into place. It is masked to the left of the R of RB-338 (clip
  x < 8 + width("OPEN", cap 14) + one glyph gap, in the part's coordinates) and starts fully
  hidden (offset = that width), so it slides out from behind RB-338 at full opacity. RB-338 is
  in place from the start.
- 0.55, 0.75, 0.95, 1.15: the four underline segments fade in left to right, each over 0.25 s.
- 1.35..1.85: the subtitle fades in.
- About window only: the icon fades in over 0.0..0.6.
The transport plays it once per start; the About window every time it opens; the final frame
equals the static wordmark (task-114's existing notes on --screenshot and the tests stand).

Branch: worktree-agent-ada5ac4c2dc929553 (rebased on main c64a1cd). Implements the user-approved animation of 2026-10-29 (OPEN slides out from behind RB-338 over 1.0 s ease-out, four underline segments fade in at 0.55/0.75/0.95/1.15 s over 0.25 s, subtitle 1.35..1.85 s, About icon 0..0.6 s); it replaces the description's underline-grows version, so AC #1 is satisfied in that approved form.

Evidence: frames rendered with the hidden dev option ORB_LOGO_T=SECONDS (freezes the animation; panel and About) at t=0,0.25,0.5,0.7,0.9,1.2,1.6,2 into scratchpad task114/panel_T.png and about_T.png (ppp 3 and 2, debug build, --demo); contact sheets sheet_panel.png and sheet_about.png. They match the mockup frames: RB-338 alone at 0, OPEN emerging from behind the R, segments fading in left to right, subtitle last, About icon fading in over 0.6 s. Command: ORB_LOGO_T=0.5 target/debug/OpenRB-338 --demo --about --ppp 2 --screenshot OUT.png. AC #1 is satisfied in the approved 2026-10-29 form (OPEN slides from behind RB-338, segments fade in), not the description's underline-grows form. AC #3: App::new has the animation off (headless tests and --screenshot see the final state; enable_logo_anim, called by main.rs, turns it on except with --screenshot); panel start time set once on the first drawn frame; About start set at each open; request_repaint only while t < 1.85 s; nothing consumes input, the click area is unchanged. The final state paints the whole logo part, so it is the static wordmark by construction (a screenshot-vs-screenshot pixel compare was inconclusive: the window width differs by 4 px between runs, shifting the raster). Only main.rs change: one line app.enable_logo_anim(). AC #2 (user approval in the running app) is left unchecked and the status stays To Do.

2026-10-29: the user accepted the look in the running app ("accepted"); AC #2 checked, status Done.

Review round 1 (of 4f0ab89), all four fixed: (1) the pieces and the final frame go through app::paint_part_tinted, the capped path of paint_part_rotated (tint added, task-119 cap kept), About's icon too; (2) LOGO_PART/LOGO_W/LOGO_H live in logo_anim.rs only (about.rs, app.rs part_size and the backgrounds loop use them; a test checks them against orb-ui-art); (3) new test the_end_state_paints_the_static_wordmark runs the panel in one egui context with the animation off and with it past END_S and compares the recorded shapes (managed texture ids normalized, they differ between runs): equal; mid-animation differs; (4) no request_repaint when ORB_LOGO_T is set.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added crates/orb-ui/src/logo_anim.rs (timing constants in one place, Frame::at curve, paint_logo with OPEN masked left of the R; 6 unit tests: t=0, ease-out midpoints, segment/sub/icon fades, final state, travel equals the art's font metrics), wired into the panel (App::show, once per start) and the About window (every open, icon fade), dev option ORB_LOGO_T (documented in docs/building.md). The user accepted the look in the running app on 2026-10-29. Tests: nightly fmt, clippy, debug orb-ui/orb-ui-art tests, release cargo test --no-fail-fast all pass; orb-ui-art gen leaves no diff.
<!-- SECTION:FINAL_SUMMARY:END -->

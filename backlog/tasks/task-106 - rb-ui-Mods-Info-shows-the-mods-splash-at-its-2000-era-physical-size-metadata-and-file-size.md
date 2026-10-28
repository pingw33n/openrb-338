---
id: TASK-106
title: >-
  rb-ui: Mods > Info shows the mod's splash at its 2000-era physical size,
  metadata and file size
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:58'
updated_date: '2026-10-28 12:55'
labels:
  - ui
  - rbm
milestone: m-3
dependencies:
  - TASK-101
references:
  - crates/rb-ui/src/menu.rs
  - crates/rb-ui/src/app.rs
  - crates/rb-ui/src/session.rs
  - docs/rbm.md
priority: low
type: feature
ordinal: 126000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for Mods > Info. It opens a centered window with the current mod's splash screen and the mod's metadata and file size. The splash is scaled so it is physically as large as it was on a 17-inch CRT monitor of 2000.

The size:
- Every known mod's splash.jpg is 256 x 256 (task-098).
- A 17-inch CRT of 2000 had about 16 inches viewable, 4:3, so about 12.8 x 9.6 in (325 x 244 mm). It typically ran at 1024 x 768, which is about 80 px per inch, or 0.317 mm per px.
- So a 256 px splash was about 81 mm (3.2 in) wide.
- Display the image at width = image_px x 0.317 mm, converted to the current screen's points through the physical pixel density.
- egui does not know a monitor's physical size. Get it from the platform: on macOS, CGDisplayScreenSize of the window's display (mm) and its pixel size. Use the Windows/Linux equivalents if cheap, otherwise a fallback.
- Fallback when it is not known: assume 96 logical points per inch on Windows/Linux and 110 on macOS, and say so.
- Record the constants and the derivation in a doc comment and in docs/ui_spec.md.
- If the physical size cannot fit the window, scale down to fit and say so in the window.

What:
- Mods > Info opens a centered, closable window; Escape closes it.
- It shows the splash of the kit that plays (the standard mod's for the built-in kit; session::mod_file and the splash decoding of App::mod_view, shared, not duplicated) at the physical size above.
- It shows the mod's metadata from its INFO chunk (docs/rbm.md section 2): the name, the version as a.b.c, the description (line breaks kept), and the web and ftp addresses. Use the same link rule as File > Info (info::web_url): only http/https or host-like text is a link, and ftp is plain text.
- It also shows the file name and folder, the file size in bytes and in KB/MB, the number of embedded files, and whether the mod is recrypted.
- For a song whose mod is missing it shows the missing name and says it is missing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Mods > Info opens a centered window with the kit's splash, the INFO metadata, the file path and size, and the file count; Escape and the close button close it
- [x] #2 The splash's on-screen physical width is 256 x 0.317 mm (about 81 mm) on a display whose physical size is known; the fallback DPI is used otherwise; both are unit-tested from given DPIs, and the size on this Mac is recorded in the notes
- [x] #3 docs/ui_spec.md section 18 lists the item with the size derivation; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. session.rs: splash_image(kit) shared by App::load_splash (mod_view) and the new window; ModDetails::of(kit) (INFO, file name and folder, size, file count, recrypted), the embedded Standard.rbm for the built-in kit. 2. New mod_info.rs: physical-size math (pure, unit-tested from given DPIs), macOS CGDisplayScreenSize through extern C CoreGraphics (no new crate), the window (centered like about.rs, Escape consumed after the other windows), a scaled-down-to-fit note. 3. Mods > Info menu item (Cmd::ModInfo), tests in tests/menu.rs, docs/ui_spec.md section 18 with the derivation. 4. fmt, clippy, cargo test --release, measure this Mac, notes, ACs, Done, commit.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-ab0ade299c2fd6dc4

Measured on this Mac (cargo test --release -p rb-ui --lib measure_main_display -- --ignored --nocapture): main display CGDisplayScreenSize 345.44 mm, current mode 2992 px wide = 8.661 px/mm. A 256 px splash is 256 x 0.317 = 81.15 mm = 702.9 px = 351.4 points at pixels_per_point 2. No new crate: a minimal extern C CoreGraphics binding (CGGetDisplaysWithPoint, CGDisplayScreenSize, CGDisplayCopyDisplayMode). Windows and Linux use the 96 ppi fallback (no cheap query added). Failed hypothesis: Window::vscroll starts 400 high (default_area_size), so the facts were scrolled away; fixed with an inner ScrollArea and default_height. Tests: 4 unit tests (assumed DPIs 96/110/144 give 81.15 mm; measured 10 px/mm at ppp 2 and 1; scale-down; formats) and 3 window tests in tests/menu.rs (centered, Escape, close button, missing mod, real PBE2.rbm facts and size, menu item present). The headless harness runs at ppp 1 so the window test checks the width against physical_width of the basis it reports, not a fixed mm.

Review round 1: (1) the Recrypted row and ModDetails.recrypted are dropped: the kit that plays is always loadable (kit_samples requires slots_covered via load_mod; use_kit returns on its error, adopt_as falls back to Kit::Standard; doc.kit is set only in settle and use_kit), no path leaves a non-loadable mod as doc.kit. ui_spec says so. (2) mod_info_ui split into body/splash helpers, rustfmt formats it. (3) cargo test --release for the workspace with RB_MOD_DIR=.../recrypted-mods: 531 pass, 0 fail; the_mod_info_window_shows_a_mod_files_facts ran on PBE2.rbm and was not skipped (no 'skipped' line with --nocapture). (4) Correction: there are 5 unit tests in mod_info.rs plus the ignored measure_main_display test. (5) Not done: the 400 px reserved for the facts stays a constant; the window scrolls if the facts are taller.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Mods > Info (Mods menu) opens a centered Mod Information window (Escape and close button close it) with the splash of the kit that plays, the INFO name, version a.b.c, description with line breaks, web link (info::web_url) and ftp text, file name, folder, size in bytes and KB/MB and the embedded file count; a missing mod is named with a notice. The splash is drawn 256 x 0.317 mm wide: on macOS from CGDisplayScreenSize and the mode's pixel width (this Mac: 345.44 mm, 2992 px, so 351.4 points at ppp 2), else 110 (macOS) or 96 ppi assumed, and scaled down to fit with a note. Splash decoding is shared (session::splash_image, App::splash_texture, also used by mod_view). docs/ui_spec.md section 18 has the item and derivation. Branch worktree-agent-ab0ade299c2fd6dc4, one commit. clippy -D warnings clean; cargo test --release: 531 pass (RB_MOD_DIR set, real-mod test ran).
<!-- SECTION:FINAL_SUMMARY:END -->

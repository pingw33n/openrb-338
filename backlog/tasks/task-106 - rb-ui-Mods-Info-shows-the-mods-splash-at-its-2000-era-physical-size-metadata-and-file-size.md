---
id: TASK-106
title: >-
  rb-ui: Mods > Info shows the mod's splash at its 2000-era physical size,
  metadata and file size
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:58'
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
- [ ] #1 Mods > Info opens a centered window with the kit's splash, the INFO metadata, the file path and size, and the file count; Escape and the close button close it
- [ ] #2 The splash's on-screen physical width is 256 x 0.317 mm (about 81 mm) on a display whose physical size is known; the fallback DPI is used otherwise; both are unit-tested from given DPIs, and the size on this Mac is recorded in the notes
- [ ] #3 docs/ui_spec.md section 18 lists the item with the size derivation; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

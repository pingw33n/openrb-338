---
id: TASK-102
title: 'rb-ui: Help > About with the logo, version, git hash and project link'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:38'
labels:
  - ui
milestone: m-3
dependencies: []
references:
  - crates/rb-ui/src/menu.rs
  - crates/rb-ui/src/art.rs
priority: low
type: feature
ordinal: 122000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for a Help menu with an About item. It shows the app logo, the version with the short git hash, and a link to https://github.com/pingw33n/open-rb-338 (the repository does not exist yet; the current one will be renamed later, so use that URL as given).

What:
- A Help menu (last in the menu bar) with 'About rb-ui...' (or 'About').
- It opens a centered, closable window (Escape closes) showing the app logo (the app_icon part, rb_ui::art, rendered as an image), the name, the version (CARGO_PKG_VERSION) and the short git hash, and the link (egui hyperlink).
- The hash comes from a build script: git rev-parse --short HEAD at build time, with '-dirty' when the work tree has changes, and 'unknown' when git or the repo is absent; it must not fail the build. Rerun the build script when HEAD changes (cargo:rerun-if-changed on the git HEAD and the ref it points to).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Help > About opens a centered window with the logo, the version and short git hash, and the GitHub link; Escape and the close button close it
- [ ] #2 The build works without git (hash 'unknown'); the hash updates after a new commit without cargo clean
- [ ] #3 A test checks the version string format (version + hash) and that the window opens and closes headlessly
- [ ] #4 docs/ui_spec.md section 18 lists the Help menu; cargo test and cargo clippy --all-targets -- -D warnings pass
<!-- AC:END -->

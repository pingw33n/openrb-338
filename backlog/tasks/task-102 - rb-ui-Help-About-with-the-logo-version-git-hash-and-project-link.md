---
id: TASK-102
title: 'rb-ui: Help > About with the logo, version, git hash and project link'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:38'
updated_date: '2026-10-28 10:38'
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
Why: the user asked (2026-10-28) for a Help menu with an About item. It shows:
- the app logo;
- the version, which is the UTC build time and the git hash, as specified below (the user's format, 2026-10-28);
- a link to https://github.com/pingw33n/open-rb-338. The repository does not exist yet; the current one will be renamed later, so use that URL as given.

What:
- A Help menu, last in the menu bar, with 'About rb-ui...' (or 'About').
- It opens a centered, closable window; Escape closes it. Center it the way File > Info does (task-100: pivot CENTER_CENTER, default_pos at the content center, a fresh id per open).
- The window shows:
  - the app logo: the app_icon part from rb_ui::art, rendered as an image;
  - the name;
  - the version string;
  - the link, as an egui hyperlink.
- The version string is `<UTC build time> (<git>)`:
  - The time is `YYYY-MM-DD HH:MM:SS` in UTC.
  - `<git>` is the short hash, prefixed with `dev-` when the build is not of the main branch, and suffixed with `-dirty` when the work tree had uncommitted changes.
  - Examples: `2026-10-28 11:32:11 (7bd8876)` is main, clean. `2026-10-28 11:32:11 (dev-7bd8876-dirty)` is another branch, dirty. `(7bd8876-dirty)` is main, dirty. `(dev-7bd8876)` is another branch, clean.
  - `(unknown)` when git or the repository is absent. This must not fail the build.
- A build script computes the version string:
  - git rev-parse --short HEAD for the hash;
  - git status --porcelain (or diff-index) for dirty;
  - git rev-parse --abbrev-ref HEAD == main for the branch.
  - On a detached HEAD, as in GitHub Actions, use GITHUB_REF_NAME when set, else treat the build as not main.
  - The build time is taken when the build script runs. Honor SOURCE_DATE_EPOCH if set, for reproducible builds.
  - Rerun the script when HEAD, the ref it points to, or the index changes (cargo:rerun-if-changed), so a commit or a branch switch updates the string without cargo clean.
  - Say in the notes how fresh the time and the dirty flag are, for example that editing a file without staging it may not rerun the script, and choose a sensible trigger.
- Keep the formatting in a pure function unit-tested with the four example cases.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Help > About opens a centered window with the logo, the version and short git hash, and the GitHub link; Escape and the close button close it
- [ ] #2 docs/ui_spec.md section 18 lists the Help menu; cargo test and cargo clippy --all-targets -- -D warnings pass
- [ ] #3 The version string is '<UTC build time> (<git>)' with dev- for a non-main build and -dirty for a dirty tree (the user's four forms), '(unknown)' without git; the build never fails for lack of git; a commit or branch switch updates it without cargo clean
- [ ] #4 A unit test checks the four example forms and unknown; a headless test opens and closes the window
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Addition from the user (2026-10-28), for task-113's release builds: the build script takes the build time from SOURCE_DATE_EPOCH when it is set (already in the spec), and takes the branch from an override variable RB_BUILD_BRANCH when it is set (main or anything else), before looking at git or GITHUB_REF_NAME. A tag build in GitHub Actions is on a detached HEAD whose GITHUB_REF_NAME is the tag, so without the override a release would show dev-. Test both variables in the unit tests of the version function.
<!-- SECTION:NOTES:END -->

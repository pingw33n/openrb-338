---
id: TASK-102
title: 'rb-ui: Help > About with the logo, version, git hash and project link'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 08:38'
updated_date: '2026-10-28 12:09'
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
- [x] #1 Help > About opens a centered window with the logo, the version and short git hash, and the GitHub link; Escape and the close button close it
- [x] #2 docs/ui_spec.md section 18 lists the Help menu; cargo test and cargo clippy --all-targets -- -D warnings pass
- [x] #3 The version string is '<UTC build time> (<git>)' with dev- for a non-main build and -dirty for a dirty tree (the user's four forms), '(unknown)' without git; the build never fails for lack of git; a commit or branch switch updates it without cargo clean
- [x] #4 A unit test checks the four example forms and unknown; a headless test opens and closes the window
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. crates/rb-ui/src/version.rs: dependency-free pure functions (is_main with RB_BUILD_BRANCH > git branch > GITHUB_REF_NAME, format_utc, version_string), unit-tested; build.rs includes it with #[path] and computes RB_UI_VERSION from git, SOURCE_DATE_EPOCH, with rerun triggers on HEAD, the branch ref, packed-refs, the index, crates/, assets/, Cargo.toml/lock and the env vars.
2. src/about.rs: Help > About window (logo, name, version, hyperlink), centered like File > Info; Cmd::About; Help menu built from a small list so Keyboard Shortcuts (task-105) is one more item.
3. Tests: unit (version), headless open/close/Escape/centering in tests/menu.rs. 4. docs/ui_spec.md section 18 Help menu. 5. fmt, clippy, test.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Addition from the user (2026-10-28), for task-113's release builds: the build script takes the build time from SOURCE_DATE_EPOCH when it is set (already in the spec), and takes the branch from an override variable RB_BUILD_BRANCH when it is set (main or anything else), before looking at git or GITHUB_REF_NAME. A tag build in GitHub Actions is on a detached HEAD whose GITHUB_REF_NAME is the tag, so without the override a release would show dev-. Test both variables in the unit tests of the version function.

Branch: worktree-agent-aa270c02ea817f889 (on main dde38a5+).

Evidence. Version strings from release builds of this branch (grep of the binary, HEAD 0b5ae25, tree dirty because the work is uncommitted):
- plain: 2026-10-28 12:01:14 (dev-0b5ae25-dirty)
- RB_BUILD_BRANCH=main SOURCE_DATE_EPOCH=1793187131: 2026-10-28 11:32:11 (0b5ae25-dirty)
- plain again: 2026-10-28 12:01:28 (dev-0b5ae25-dirty) (a change of an env variable reruns the script: rerun-if-env-changed).
Freshness: build.rs reruns on HEAD, the branch ref (via git symbolic-ref; the worktree's own HEAD/index via --git-dir, refs and packed-refs via --git-common-dir), the index, anything under crates/ and assets/, Cargo.toml/Cargo.lock and the env variables SOURCE_DATE_EPOCH, RB_BUILD_BRANCH, GITHUB_REF_NAME. An unstaged edit of source under crates/ reruns it (directory mtime scan), so the dirty flag is fresh for source; an edit only of docs/ does not rerun it (the flag and the time stay as of the last rerun). The time is the time of the last rerun, i.e. of the last build whose inputs changed. No git: version_string gives (unknown), covered by the unit test; every git call returns None on failure so the build cannot fail.
Design: src/version.rs has no dependencies and is included by build.rs with #[path]; it holds is_main (RB_BUILD_BRANCH > git branch unless detached HEAD > GITHUB_REF_NAME), format_utc and version_string, with unit tests (four forms, unknown, UTC dates incl. leap day, the override and GITHUB_REF_NAME cases). Help menu items are the HELP_ITEMS list in menu.rs: task-105 adds a line.
Failed attempt: the first headless logo check counted Mesh( shapes and passed vacuously; egui draws an image as a RectShape with a brush, so the test compares the count of 'brush: Some(' with and without the window (+1).
Tests: cargo test --release -q for the workspace with RB_MOD_DIR: 515 passed, 0 failed; clippy --all-targets -D warnings clean; fmt run.

Review round 1 (rebased onto main c3b2be7). This SUPERSEDES the earlier Freshness note. Fixed:
1. packed-refs and the loose branch ref are watched only when the file exists (cargo treats a missing rerun-if-changed path as stale and reruns every build). A second cause found while proving it: the script's own 'git status' refreshed .git/index, which it watches, so one more rerun followed every rerun; git now runs with GIT_OPTIONAL_LOCKS=0. Proof: touch crates/rb-ui/src/lib.rs; build 1 'Compiling rb-ui'; build 2 with -v has 0 'Running' lines (Finished in 0.14s); build 3 -v: 'Fresh rb-ui' (Finished in 0.14s). Before the GIT_OPTIONAL_LOCKS fix the second build ran rustc for rb_ui again.
2. Triggers narrowed to crates/rb-ui, assets/ui, .git HEAD/index/packed-refs/branch ref (existing paths) and the three env variables. An edit of another crate or of docs is seen only once staged (index changes); touching crates/rb-engine/src/lib.rs rebuilds rb-ui as a dependency but does not rerun the script.
3. Dirty uses 'git status --porcelain --untracked-files=no' (as git describe --dirty); in docs/ui_spec.md.
4. docs/ui_spec.md section 18 updated to the new trigger set.
5. format_utc tests gain 2000-02-29 and 2100-03-01.
fmt, clippy -D warnings clean; cargo test --release -q workspace with RB_MOD_DIR: 515 passed, 0 failed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Help > About is done on branch worktree-agent-aa270c02ea817f889 (one commit). The Help menu is last in the bar (items in HELP_ITEMS); 'About OpenRB-338...' opens a centered window (pivot CENTER_CENTER, default_pos at the content center, a fresh id per open as File > Info) with the app_icon logo, the name, 'Version <UTC time> (<git>)' and the hyperlink https://github.com/pingw33n/open-rb-338; Escape and the close button close it. crates/rb-ui/build.rs computes RB_UI_VERSION with the pure functions of src/version.rs: SOURCE_DATE_EPOCH, RB_BUILD_BRANCH > git branch > GITHUB_REF_NAME, dev- and -dirty (git status --porcelain --untracked-files=no), (unknown) without git, never fails the build. It reruns on HEAD, the branch ref, packed-refs and the index (each only if the file exists), crates/rb-ui, assets/ui and the three variables; two builds in a row do not rerun it or recompile rb-ui (shown in the notes). Real strings: 2026-10-28 12:01:14 (dev-0b5ae25-dirty) plain; 2026-10-28 11:32:11 (0b5ae25-dirty) with RB_BUILD_BRANCH=main SOURCE_DATE_EPOCH=1793187131. docs/ui_spec.md section 18 lists the Help menu and the version rules. Tests: unit tests of version.rs (four forms, unknown, both variables, 2000/2100/leap dates) and the headless test the_about_window_opens_centered_and_closes; cargo test --release 515 pass, clippy and fmt clean.
<!-- SECTION:FINAL_SUMMARY:END -->

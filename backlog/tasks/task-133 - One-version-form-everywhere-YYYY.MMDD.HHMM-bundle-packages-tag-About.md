---
id: TASK-133
title: 'One version form everywhere: YYYY.MMDD.HHMM (bundle, packages, tag, About)'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 20:07'
updated_date: '2026-10-28 20:43'
labels:
  - tools
  - ui
milestone: m-4
dependencies:
  - TASK-132
references:
  - scripts/version.sh
  - crates/rb-ui/src/version.rs
  - crates/rb-ui/build.rs
  - docs/building.md
priority: medium
type: enhancement
ordinal: 152000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user decided on 2026-10-28 that the version is the build time in one form everywhere: YYYY.MMDD.HHMM (UTC, from SOURCE_DATE_EPOCH, zero-padded, e.g. 2026.1028.1930). It replaces both forms task-110 introduced: YYYY_MM_DD_HH_MM_SS in file names and YYYY.MMDD.HHMMSS in Info.plist.

Where the form applies:
- The app bundle: Info.plist CFBundleShortVersionString and CFBundleVersion are both YYYY.MMDD.HHMM; OpenRBBuildTime may stay as the full time.
- Package file names (scripts/package.sh, package-dmg.sh, release.yml): e.g. OpenRB-338-2026.1028.1930-universal-apple-darwin.dmg.
- The git release tag is v + the version, e.g. v2026.1028.1930. Task-113 still says v-YYYY_MM_DD_HH_MM_SS, regex ^v-[0-9]{4}(_[0-9]{2}){5}$. Replace it with ^v[0-9]{4}\.[0-9]{4}\.[0-9]{4}$, and SOURCE_DATE_EPOCH comes from the tag (minute precision, seconds 0). Update task-113's notes through the CLI.
- The UI's About box shows "2026.1028.1930 (abc1234)": the version, then the commit part as today, with dev- and -dirty for local builds (task-102, crates/rb-ui/src/version.rs and build.rs). Today it shows '<UTC time> (<hash>)'.
- The READMEs in the packages and docs/building.md show the same string.
- scripts/version.sh prints it.

One time per build: SOURCE_DATE_EPOCH, truncated to the minute, gives the same string in the binary (build.rs), the plist, the file names and the tag. Without SOURCE_DATE_EPOCH, build-all.sh still exports one time for the whole run. A local cargo build uses its own build time; the task-110 review noted that the dev bundle (macos-dev-app.sh) can then differ from the binary's time, so read the version from the built binary or document the difference.

Ordering: the integer parts compare correctly. MMDD and HHMM are zero-padded, so a later time always compares higher, both as strings and per component; note it in the docs. HHMM at most 2359 also keeps each component under 65535 for any future App Store validation; YYYY.MMDD.HHMMSS would not have.

Tests: the version.rs and build.rs tests for the new format, and a test or script check that version.sh, the plist and the About string agree for one SOURCE_DATE_EPOCH.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Info.plist, package file names, version.sh, the READMEs and docs use YYYY.MMDD.HHMM; the About box shows 'YYYY.MMDD.HHMM (<[dev-]hash[-dirty]>)'; all agree for one SOURCE_DATE_EPOCH (tested)
- [x] #2 Task-113's tag rule is v<version> (^v[0-9]{4}\.[0-9]{4}\.[0-9]{4}$) in its notes; docs say how a tag maps to SOURCE_DATE_EPOCH
- [x] #3 cargo +nightly fmt --check, clippy, cargo test --release, shellcheck and actionlint pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a248d30570db4b269 (rebased on main b472847).

Evidence (SOURCE_DATE_EPOCH=1793215800, RB_BUILD_BRANCH=main, scratch DIST): scripts/package-dmg.sh built OpenRB-338-2026.1028.1930-universal-apple-darwin.dmg; mounted: Info.plist CFBundleShortVersionString = CFBundleVersion = 2026.1028.1930, OpenRBBuildTime = 2026-10-28 19:30:00 UTC, README.txt first line "OpenRB-338 2026.1028.1930"; the binary in the image contains "2026.1028.1930 (b472847-dirty)" (dirty: uncommitted work). package.sh (SKIP_BUILD) gave OpenRB-338-2026.1028.1930-aarch64-apple-darwin.tar.gz with README line "OpenRB-338 2026.1028.1930 (aarch64-apple-darwin)". Linux/Windows packages not built (Docker not used). version.sh --epoch v2026.1028.1930 = 1793215800; it rejects v2026.1328.1930, 2026.1028.2560 and the old v-2026_10_28_10_10_22 form. Tests: rb-ui version::tests::version_sh_agrees_with_the_about_box runs scripts/version.sh and compares with version_number for six epochs (incl. the tag round trip); the menu.rs about test now checks YYYY.MMDD.HHMM (first run failed: it still expected the old 19-char time; fixed).

Review round 1: (1) the version.sh test is #[cfg(unix)]; Windows is covered only by the release job running version.sh in Git Bash (bash on windows-latest may be the WSL launcher). (2) --epoch now needs the v (^v[0-9]{4}\.[0-9]{4}\.[0-9]{4}$); the version without v is rejected, tested. (3) About format fixed in docs/ui_spec.md and about.rs. (4) GNU date verified (Docker daemon not running; brew coreutils 9.12 date first on PATH): default 2026.1028.1930; --time 2026-10-28 19:30:00; --epoch v2026.1028.1930 = 1793215800; exit 1 for 2026.1028.1930, v2026.1328.1930, v2026.1028.2560, v2026.0230.0000, v-2026_10_28_10_10_22. (5) reflow nits fixed; task-113 description and AC 2 rewritten to the new tag rule (its older notes keep the history).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The version is one form, YYYY.MMDD.HHMM (UTC build time, minute precision), everywhere. crates/rb-ui/src/version.rs has version_number() (format_utc is gone); the About box shows "2026.1028.1930 (abc1234)" with dev- and -dirty as before. scripts/version.sh prints the version, with --time (full time, for OpenRBBuildTime) and --epoch TAG (SOURCE_DATE_EPOCH of a tag v2026.1028.1930; rejects other text and impossible dates). macos-app.sh, package.sh, package-dmg.sh and release-env.sh use it: plist CFBundleShortVersionString and CFBundleVersion, file names OpenRB-338-2026.1028.1930-<target>.<ext>, README first lines. docs/building.md and AGENTS.md updated (table, ordering, tag mapping, dev-bundle caveat); task-113 notes carry the tag rule v<version>, regex ^v[0-9]{4}\.[0-9]{4}\.[0-9]{4}$. Tests: cargo test --release -q --no-fail-fast 585 passed, 0 failed, 3 ignored (64 result lines); cargo test -p rb-ui (debug) 197 passed; fmt, clippy -D warnings, shellcheck scripts/*.sh and actionlint clean. Built a real universal .dmg locally; its plist, file name and binary string agree (notes). Branch worktree-agent-a248d30570db4b269, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

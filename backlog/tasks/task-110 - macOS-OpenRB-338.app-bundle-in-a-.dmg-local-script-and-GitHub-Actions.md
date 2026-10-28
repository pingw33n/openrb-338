---
id: TASK-110
title: 'macOS: OpenRB-338.app bundle in a .dmg (local script and GitHub Actions)'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 10:12'
updated_date: '2026-10-28 19:57'
labels:
  - tools
  - ui
milestone: m-4
dependencies:
  - TASK-108
  - TASK-104
references:
  - crates/rb-ui/src/art.rs
priority: medium
type: feature
ordinal: 130000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for a macOS .dmg, built locally and by GitHub Actions. Double-clicking an unbundled binary in Finder opens Terminal, so the UI needs an .app bundle to launch like an app. A bundle also gives the Dock and Activity Monitor their name and icon properly (task-104 names the binary OpenRB-338 for now).

What:
- A script (scripts/, using task-108's build) makes OpenRB-338.app:
  - Contents/MacOS/OpenRB-338, a universal binary (aarch64 + x86_64 via lipo);
  - Contents/Info.plist: CFBundleName and CFBundleDisplayName OpenRB-338, a bundle identifier (propose one, e.g. io.github.pingw33n.openrb338), CFBundleShortVersionString from the workspace version, CFBundleVersion from the build, LSMinimumSystemVersion, NSHighResolutionCapable, and LSApplicationCategoryType music;
  - Contents/Resources/OpenRB-338.icns, generated from the app_icon part (rb_ui::art; render the iconset sizes 16..1024 with resvg, then iconutil).
- Include the CLI: put rb (later orb, task-068) in the .dmg next to the app, with a short note on putting it on PATH. Alternatively put it inside the bundle at Contents/MacOS/rb with a symlink hint. Choose one and say why.
- The .dmg: hdiutil (or create-dmg) with the app, an /Applications symlink for drag-install, a background or icon layout if cheap, and the license and readme. Name it OpenRB-338-<version>.dmg.
- release.yml builds the .dmg on a macos runner and uploads it.
- Signing: ad-hoc codesign (codesign -s -) so it runs locally. Developer ID signing and notarization need the user's Apple developer account and secrets. Leave hooks for them (env-gated steps that are skipped when the secrets are absent) and write down for the user what Gatekeeper shows for an unsigned download (right-click > Open, or xattr -d com.apple.quarantine).
- Check here: mount the .dmg, copy the app to a temp folder, launch it, and check ps and the Dock name; record in the notes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A local script produces OpenRB-338-<version>.dmg with a universal OpenRB-338.app (Info.plist, .icns from app_icon) and the CLI; the app launches from the mounted image
- [x] #2 docs/building.md describes it; validated with actionlint (or act)
- [ ] #3 release.yml builds and uploads the .dmg on a macos runner; no signing or notarization (ad-hoc codesign only where launch needs it); the unsigned-app behavior is documented
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. ui-art gets 'iconset DIR': renders assets/ui/app_icon.svg at 16..1024 (1x/2x names) with its own resvg (the resvg CLI is not installed here); iconutil makes the .icns.
2. scripts/macos-app.sh: shared builder of OpenRB-338.app from a UI binary and a CLI binary (Info.plist, icns, ad-hoc codesign). CFBundleVersion = git commit count; LSMinimumSystemVersion read from the binaries' LC_BUILD_VERSION.
3. scripts/macos-dev-app.sh: wraps target/release/OpenRB-338 into target/OpenRB-338.app for everyday runs.
4. scripts/package-dmg.sh: build.sh for both macOS targets (SKIP_BUILD=1 reuses dist/*/bin), lipo to a universal pair, app via macos-app.sh, CLI inside the bundle at Contents/MacOS/rb (drag-install carries it; one signature; PATH via ln -s), hdiutil UDZO dmg with /Applications symlink, LICENSE, README.txt -> OpenRB-338-<version>.dmg.
5. release.yml: dmg job on macos-15 after the package jobs (downloads bin-*-apple-darwin, SKIP_BUILD=1), uploads artifact dmg. docs/building.md: section and Gatekeeper text. actionlint, shellcheck.
6. Check: build the universal dmg here, mount, copy the app to a temp folder, open it, ps/lsappinfo, Activity Monitor icon, --screenshot of the bundled binary.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User decisions (2026-10-28): no signing or notarization at all: drop the signing hooks, keep only ad-hoc codesign if the app needs it to launch on Apple Silicon (it does for arm64 binaries), and document Gatekeeper's right-click > Open. Both architectures: the universal app covers amd64 and arm64 in one .dmg; if a universal binary is impractical, make two .dmgs (-arm64, -x86_64).

The user (2026-10-28): with the unbundled binary (task-104) the Dock tooltip and Activity Monitor name show OpenRB-338, but Activity Monitor shows the generic executable (terminal) icon: it takes the icon from the bundle (CFBundleIconFile), which an unbundled binary does not have; the Dock icon comes from eframe's runtime icon. Check in this task that Activity Monitor shows the app icon for the bundled app, and also provide a dev script (e.g. scripts/macos-dev-app.sh) that wraps target/release/OpenRB-338 into a local OpenRB-338.app for everyday runs, so the icon is right without building the .dmg.

Branch: worktree-agent-a741dbb450f783719 (rebased on main 1386e7e)

Built here with SOURCE_DATE_EPOCH=1793181022: scripts/package-dmg.sh -> dist/universal-apple-darwin/OpenRB-338-0.1.0.dmg, 34302871 B (hdiutil verify ok). Contents: OpenRB-338.app, Applications link, LICENSE (35149 B), README.txt. lipo -archs of Contents/MacOS/OpenRB-338 and Contents/MacOS/rb: 'x86_64 arm64'. codesign: Signature=adhoc, verify --deep --strict ok. Info.plist (plutil -p): names OpenRB-338, id io.github.pingw33n.openrb338, 0.1.0, CFBundleVersion 209 (commit count), LSMinimumSystemVersion 11.0 (minos of the binary), NSHighResolutionCapable, category music. .icns 202445 B ('ic12', from ui-art iconset: new subcommand rendering assets/ui/app_icon.svg at 16..1024 with resvg; resvg CLI is not installed here). Looked at the 512@2x PNG: the knob icon.
Check: mounted the .dmg, copied the app to /private/tmp/claude-501/-Users-pingw33n-dev-rb/8f24950b-b10f-47b2-be09-4ad0d95914c5/scratchpad/task110/copy, opened it with 'open -n': it ran (pid 13063). lsappinfo for the bundled app: bundleID=io.github.pingw33n.openrb338, bundle path = the .app, Version=209. For the earlier unbundled run of target/release/OpenRB-338: bundleID=[ NULL ], bundle path = the binary, Version=[ NULL ]: that is why Activity Monitor shows the generic icon. I did not screenshot Activity Monitor itself (the icon column cannot be read headless); the evidence is the bundle identity and CFBundleIconFile present. Also ran 'OpenRB-338.app/Contents/MacOS/OpenRB-338 --screenshot shot.png --demo': wrote a 1.0 MB PNG. Killed only my pids. rb --version does not exist (rb has no such flag); not added.
Dev script: scripts/macos-dev-app.sh dist/aarch64-apple-darwin/bin/OpenRB-338 -> target/OpenRB-338.app, launched with open -n: lsappinfo shows the bundle id and Version 209.
Decisions: CLI inside the bundle (Contents/MacOS/rb) with an ln -s hint in README.txt, not next to the app: drag-install carries both, one signature, nothing left behind. No dmg background or icon layout (Finder scripting, not reproducible in CI). 'git rev-list --count HEAD' gives CFBundleVersion. release.yml dmg job (macos-15, needs prepare+package, actions/download-artifact pinned to v7.0.0 = 37930b1c2abaa49bbe596cd826c3c89aef350131, checked with the GitHub API; v7 pairs with the upload-artifact v7 in use) lays the two bin-*-apple-darwin artifacts out as dist/<target>/bin, chmod +x (artifacts lose the mode), runs SKIP_BUILD=1 scripts/package-dmg.sh (cargo is needed on the runner only for the icon: ui-art) and uploads artifact 'dmg'.
CI-only (unverified): the dmg job itself (download-artifact layout of a pattern download is dist-in/bin-<target>/, assumed from the action's docs; hdiutil on the runner), and a Gatekeeper prompt on a quarantined download. actionlint ok, shellcheck ok on scripts/*.sh.
Gates: cargo +nightly fmt --check ok; clippy --all-targets -D warnings ok; cargo test --release -q --no-fail-fast: 582 passed, 0 failed, 3 ignored.

User decisions (2026-10-28), applied in review round 1: (A) the built-in kit assets/Standard.rbm may ship in the public packages (tar.gz, zip, dmg, later installer and AppImage); docs/building.md says so. (B) the bundle identifier is net.emphased.openrb338 (replaced everywhere). (C) no Cargo version (0.1.0) anywhere a user sees a version: scripts/version.sh now prints the UTC build time YYYY_MM_DD_HH_MM_SS from SOURCE_DATE_EPOCH (the release-tag form); package.sh names packages OpenRB-338-<time>-<target>; the dmg is OpenRB-338-<time>-universal-apple-darwin.dmg; package.sh, package-dmg.sh and build-all.sh export SOURCE_DATE_EPOCH once so the name and the About box agree; Info.plist CFBundleShortVersionString and CFBundleVersion are YYYY.MMDD.HHMMSS (both the time, not the commit count: it orders builds across checkouts and branches and matches the file name), plus OpenRBBuildTime '2026-10-28 19:30:00 UTC'; READMEs say 'built 2026-10-28 19:30:00 UTC'. Neither rb nor OpenRB-338 has a --version flag (rb --version: 'unexpected argument'), so nothing prints 0.1.0 and nothing was added.
Review round 1 (all fixed in the amended commit; items 1 and 2 were the user's decisions above): 3 ui-art iconset requires DIR (usage error and exit 2 otherwise). 4 Gatekeeper text: macOS 15 Sequoia System Settings > Privacy & Security > Open Anyway, xattr -dr com.apple.quarantine, right-click > Open kept for macOS 14 and older (docs/building.md and the dmg README). 5 reproducibility: cp -p and touch -t from SOURCE_DATE_EPOCH on the bundle and the image's files. Two runs of SKIP_BUILD=1 package-dmg.sh: the mounted file trees are identical (same sha256 per file) but the .dmg bytes differ (cmp: first difference at byte 5, the UDIF header; hdiutil writes a fresh volume UUID and date, no option to fix it), so the dmg is not byte-reproducible; stated in docs/building.md. 6 rb is signed explicitly, then the bundle without --deep; docs and script text corrected; codesign --verify --deep --strict passes. 7 build.sh sets MACOSX_DEPLOYMENT_TARGET=11.0 for the macOS targets: otool minos 11.0 for both slices; macos-app.sh fails when no minos is found (the 11.0 fallback is gone); documented. 8 package-dmg.sh checks that the archs are exactly arm64 and x86_64. 9 macos-app.sh runs cargo with --manifest-path. 10 NSHumanReadableCopyright is 'Copyright (C) 2026 the OpenRB-338 authors. GPL-3.0-only.'. 11 test iconset_has_the_ten_sizes in crates/rb-ui-art/tests/art.rs (the ten names and pixel sizes, render_size output size and non-empty); the file list is ui-art's iconset_files(). 12 AGENTS.md line kept.
AC1 check redone from the mounted image: hdiutil attach, open -n '/Volumes/OpenRB-338/OpenRB-338.app': ps shows /Volumes/OpenRB-338/OpenRB-338.app/Contents/MacOS/OpenRB-338; lsappinfo: bundleID=net.emphased.openrb338, bundle path /Volumes/OpenRB-338/OpenRB-338.app, Version=2026.1028.193000, Arch ARM64 (the Dock and Activity Monitor name come from this bundle; not seen on screen). codesign --verify --deep --strict ok on the mounted app. Rebuilt with SOURCE_DATE_EPOCH=1793215800: dist/universal-apple-darwin/OpenRB-338-2026_10_28_19_30_00-universal-apple-darwin.dmg, 34302750 B. The aarch64 .tar.gz package name and README checked (OpenRB-338-2026_10_28_19_30_00-aarch64-apple-darwin.tar.gz). Gates: nightly fmt, clippy -D warnings, cargo test --release 583 passed 0 failed 3 ignored, actionlint, shellcheck. AC3 (the CI job) is unchecked until a CI release run; the dmg job and a quarantined download's Gatekeeper behavior were not run.

User, 2026-10-28: keep the copyright string 'Copyright (C) 2026 the OpenRB-338 authors. GPL-3.0-only.' as it is.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added scripts/macos-app.sh (OpenRB-338.app: Info.plist, .icns from app_icon, rb then bundle ad-hoc signed without --deep), scripts/package-dmg.sh (lipo universal UI and CLI, CLI inside the bundle, Applications link, LICENSE, README.txt, hdiutil UDZO -> OpenRB-338-<YYYY_MM_DD_HH_MM_SS>-universal-apple-darwin.dmg), scripts/macos-dev-app.sh (target/OpenRB-338.app for everyday runs), 'ui-art iconset DIR' with iconset_files() and a test, MACOSX_DEPLOYMENT_TARGET=11.0 for the macOS builds, a dmg job in release.yml (macos-15, reuses the macOS bin artifacts and the prepare build time), docs/building.md (the version-is-build-time scheme, what may ship, the app and dmg, Gatekeeper on macOS 15 and older) and an AGENTS.md line. User decisions applied: bundle id net.emphased.openrb338; the built-in kit may ship; packages are versioned by UTC build time (scripts/version.sh, all package names, plist, READMEs). Built here with SOURCE_DATE_EPOCH=1793215800: OpenRB-338-2026_10_28_19_30_00-universal-apple-darwin.dmg, 34302750 B, universal, ad-hoc signed, launched from /Volumes/OpenRB-338 (lsappinfo: bundle id, bundle path, Version). The dmg bytes differ between runs (hdiutil UUID), the mounted files are identical. CI-only: the dmg job (AC3 left unchecked) and a quarantined download. Checks: fmt, clippy, 583 tests pass, actionlint, shellcheck. Branch worktree-agent-a741dbb450f783719, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

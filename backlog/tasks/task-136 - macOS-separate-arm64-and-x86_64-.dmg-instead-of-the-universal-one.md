---
id: TASK-136
title: 'macOS: separate arm64 and x86_64 .dmg instead of the universal one'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-29 08:06'
updated_date: '2026-10-29 09:44'
labels:
  - tools
milestone: m-4
dependencies:
  - TASK-068
references:
  - scripts/package-dmg.sh
  - scripts/macos-app.sh
  - .github/workflows/release.yml
  - scripts/release-assets.sh
  - docs/building.md
priority: medium
type: enhancement
ordinal: 155000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user decided on 2026-10-29 to ship one .dmg per architecture, Apple silicon (aarch64) and Intel (x86_64), instead of the universal (lipo) .dmg of task-110. Each is about half the size, and the universal one is dropped from releases.

What:
- scripts/package-dmg.sh TARGET builds OpenRB-338-<version>-<target>.dmg (target aarch64-apple-darwin or x86_64-apple-darwin) from dist/<target>/bin, with no lipo. Keep the app layout, ad-hoc signing, Info.plist (LSMinimumSystemVersion from that slice's minos), README, LICENSE and the Applications link. Remove the universal path and the universal-apple-darwin dist dir.
- release.yml: the dmg job becomes a matrix of the two targets (macos-15 for arm64, macos-15-intel for x86_64, or both on one runner with SKIP_BUILD, your choice; say why). Artifacts are dmg-<target>.
- scripts/release-assets.sh: the expected list has two .dmg files instead of one, making 12 packages plus SHA256SUMS.txt. Update the draft-release job's download pattern, docs/building.md, and the task-134 checklist criteria that name the universal dmg (through the CLI).
- Check locally on this Mac: build both, check each app's binaries with lipo -archs (exactly one arch each), run codesign --verify, open the arm64 app from its mounted image, and record the sizes. The x86_64 app can be checked with arch -x86_64 if Rosetta is installed; otherwise say so.
- This depends on task-068 (the rename), because the CLI inside the bundle becomes orb. Rebase after it lands.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 package-dmg.sh builds one single-arch .dmg per macOS target (no lipo, no universal); both build and verify locally (lipo -archs, codesign --verify), sizes recorded
- [x] #2 release.yml builds both dmgs; release-assets.sh expects both; docs/building.md and task-134's checklist updated; shellcheck and actionlint pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-aed6e9bb403980bd9

Local run (SOURCE_DATE_EPOCH=1793265978, version 2026.1029.0926): scripts/package-dmg.sh aarch64-apple-darwin and x86_64-apple-darwin, both built with --profile dist.
Sizes: aarch64 dmg 13,636,328 bytes, x86_64 dmg 14,610,880 (universal was 26,980,214). Binaries: aarch64 OpenRB-338 14,981,552, orb 5,418,736; x86_64 OpenRB-338 16,373,416, orb 5,749,832.
Mounted each image: lipo -archs gives arm64 for both binaries of the aarch64 app and x86_64 for both of the x86_64 app; codesign --verify --deep --strict ok for both; LSMinimumSystemVersion 11.0.
Opened the arm64 app from the mounted image with open -n: lsappinfo showed LSBundlePath inside the mount and CFBundleVersion 2026.1029.0926 (killed only that pid). Rosetta present (oahd running): arch -x86_64 on the x86_64 app ran orb --help and OpenRB-338 --screenshot (2862x1810 PNG); the arm64 app also made a screenshot.
Runner choice: matrix of two jobs on the native runners (macos-15-intel, macos-15), each downloads only its bin-<target>; no lipo, nothing shared between jobs, matches the package jobs.
Task-134: AC 4 (11 packages) replaced with the 12-package text and a new AC for the dmg matrix (#6); note appended; pattern now dmg-*.
Also updated AGENTS.md (package-dmg line).
Checks: shellcheck scripts/*.sh, actionlint, cargo +nightly fmt --check, clippy -D warnings, cargo test --release: 586 passed, 0 failed.

Review round 1: restored task-134 AC order (#4 draft release, #5 AppImage, #6 dmg matrix; diff against e60dace shows only the intended changes), reworded the build.sh minos comment (the macOS app, both targets), docs/building.md says about half; shellcheck scripts/build.sh ok.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
package-dmg.sh now takes a macOS target and builds dist/<target>/OpenRB-338-<version>-<target>.dmg, single architecture (lipo -archs checked, no lipo -create); the universal path is gone. release.yml: dmg is a two-job matrix on macos-15-intel and macos-15, artifacts dmg-<target>; the draft release downloads dmg-*; release-assets.sh expects 12 packages (two .dmg); docs/building.md, AGENTS.md and task-134 (AC and note) updated. Local: both dmgs built (13,636,328 aarch64; 14,610,880 x86_64), one arch each, codesign verify ok, arm64 app opened from the mounted image, x86_64 app ran under Rosetta. Only CI can check the matrix run itself and a real Intel Mac (task-134 AC 6). Branch worktree-agent-aed6e9bb403980bd9, one commit. Tests: shellcheck, actionlint, fmt, clippy, cargo test --release 586 pass.
<!-- SECTION:FINAL_SUMMARY:END -->

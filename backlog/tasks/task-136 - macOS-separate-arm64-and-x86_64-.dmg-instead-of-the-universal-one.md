---
id: TASK-136
title: 'macOS: separate arm64 and x86_64 .dmg instead of the universal one'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-29 08:06'
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
- [ ] #1 package-dmg.sh builds one single-arch .dmg per macOS target (no lipo, no universal); both build and verify locally (lipo -archs, codesign --verify), sizes recorded
- [ ] #2 release.yml builds both dmgs; release-assets.sh expects both; docs/building.md and task-134's checklist updated; shellcheck and actionlint pass
<!-- AC:END -->

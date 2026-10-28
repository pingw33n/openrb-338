---
id: TASK-134
title: 'First release run: verify the dmg, installer and Linux package jobs'
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-10-28 21:26'
labels:
  - tools
  - tests
milestone: m-4
dependencies:
  - TASK-111
  - TASK-112
  - TASK-113
references:
  - .github/workflows/release.yml
  - docs/building.md
priority: medium
type: task
ordinal: 153000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: several release.yml jobs have never run, since they run only on a release tag or workflow_dispatch, and no release run has happened yet. The tasks that added them left their CI acceptance criteria to this task:
- task-110: the dmg job (macos-15), never run.
- task-111: the Windows installer job (installer-x64 and installer-arm64). The arm64 installer has not been run on any machine. The native Windows build of rb-ui's embedded resources (build.rs, embed-resource with rc.exe) is checked only by ci.yml's windows-latest job.
- task-108/132: the Linux package jobs, and the local Docker path, never run with --profile dist.
- task-112 and task-113 add more jobs (the AppImage, the draft release).

What: on the first release run (a v<YYYY.MMDD.HHMM> tag or workflow_dispatch), check that every job passes and every artifact is produced, and spot-check the artifacts:
- the dmg mounts, the app opens, and Gatekeeper behaves as docs/building.md says;
- the x64 and arm64 installers install and uninstall cleanly (arm64 needs an arm64 Windows machine);
- the AppImage runs on a stock distribution;
- the draft release lists every package and SHA256SUMS.txt.
File a bug for each failure. Assigned to the user, because it needs a release run and real machines; agents may help with the checks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every release.yml job passes on the first release run and every artifact is produced (dmg, installers x64/arm64, tar.gz/zip, AppImages, draft release)
- [ ] #2 Spot checks on real machines done (macOS Gatekeeper, Windows x64 and arm64 install/uninstall, Linux AppImage); failures filed as bugs
<!-- AC:END -->

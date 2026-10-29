---
id: TASK-134
title: 'First release run: verify the dmg, installer and Linux package jobs'
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-10-28 21:26'
updated_date: '2026-10-29 05:56'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-112 (AppImage). Both AppImages were built and smoke tested locally on 2026-10-29 in Docker (docs/building.md 'The AppImage', Checked). Still unverified, for the first release run: (1) the appimage (x86_64) and appimage (aarch64) CI jobs pass on ubuntu-22.04 and ubuntu-22.04-arm, including the appimage-smoke.sh step and appimagetool 1.9.1 with --appimage-extract-and-run on the runners; (2) the x86_64 AppImage runtime itself: in Docker Desktop's amd64 emulation it cannot be exec-ed (Exec format error), so locally only its squashfs and AppRun were run, with ARGV0 set by hand; run it on a real x86_64 Linux host or the CI runner, including the symlink named rb (ARGV0 from the real runtime was verified on aarch64 only); (3) a real desktop: X11 and Wayland, icon and name in the shell (StartupWMClass=OpenRB-338), a real GPU's Vulkan driver (the container used lavapipe; WGPU_BACKEND=gl failed there), and audio output through libasound2; (4) the aarch64 AppImage on arm64 hardware (it ran natively on Apple silicon under Docker Linux, but not on a distribution of its own).

From task-112 round 2: item (2) above is the open x86_64 check: run the x86_64 AppImage through its own runtime (--appimage-extract-and-run, then --screenshot under xvfb, and a symlink named rb) on the CI runner or an x86_64 host; locally only its unpacked AppRun ran (OPENRB_APPIMAGE_EMULATED path, never set in CI).
<!-- SECTION:NOTES:END -->

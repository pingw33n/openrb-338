---
id: TASK-134
title: 'First release run: verify the dmg, installer and Linux package jobs'
status: To Do
assignee:
  - '@pingw33n'
created_date: '2026-10-28 21:26'
updated_date: '2026-10-29 12:24'
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
- task-111: the Windows installer job (installer-x64 and installer-arm64). The arm64 installer has not been run on any machine. The native Windows build of orb-ui's embedded resources (build.rs, embed-resource with rc.exe) is checked only by ci.yml's windows-latest job.
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
- [x] #1 Every release.yml job passes on the first release run and every artifact is produced (dmg, installers x64/arm64, tar.gz/zip, AppImages, draft release)
- [ ] #2 A v<YYYY.MMDD.HHMM> tag run builds every package with the tag's time: the About box/version string of each package equals the tag (v2026.1028.1930 -> 2026.1028.1930), file names carry it, and the hash has no dev- when the tag is on main (ORB_BUILD_BRANCH=main)
- [ ] #3 The tag run ends with a DRAFT release with all 12 packages (two .dmg: aarch64 and x86_64) and SHA256SUMS.txt (13 assets) and the changelog body; a re-run replaces the draft (still one release); a tag not matching the pattern (e.g. v1) fails in prepare and creates nothing; nothing is published automatically
- [ ] #4 On the first release run the x86_64 appimage smoke step passes its four screenshots, the backend asserted for each (WGPU_BACKEND=vulkan gives Vulkan, WGPU_BACKEND=gl gives Gl, automatic gives Vulkan, Vulkan ICDs hidden gives Gl); on a real Linux desktop with no Vulkan driver the AppImage opens through OpenGL (task-135)
- [ ] #5 Spot checks on real machines done (macOS Gatekeeper with both .dmg on the user's Mac, Windows x64 install and uninstall, the Linux UI and AppImage); failures filed as bugs. The arm64 Windows installer is built and checked by CI only, not on real hardware (user decision 2026-10-29)
- [x] #6 The dmg matrix of release.yml (task-136) passes for both targets, dmg (x86_64-apple-darwin) on macos-15-intel and dmg (aarch64-apple-darwin) on macos-15; each artifact dmg-<target> holds one OpenRB-338-<version>-<target>.dmg whose app has exactly one architecture. The x86_64 app is checked by CI and on the user's Mac, not on a real Intel Mac (user decision 2026-10-29)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-112 (AppImage). Both AppImages were built and smoke tested locally on 2026-10-29 in Docker (docs/building.md 'The AppImage', Checked). Still unverified, for the first release run: (1) the appimage (x86_64) and appimage (aarch64) CI jobs pass on ubuntu-22.04 and ubuntu-22.04-arm, including the appimage-smoke.sh step and appimagetool 1.9.1 with --appimage-extract-and-run on the runners; (2) the x86_64 AppImage runtime itself: in Docker Desktop's amd64 emulation it cannot be exec-ed (Exec format error), so locally only its squashfs and AppRun were run, with ARGV0 set by hand; run it on a real x86_64 Linux host or the CI runner, including the symlink named orb (ARGV0 from the real runtime was verified on aarch64 only); (3) a real desktop: X11 and Wayland, icon and name in the shell (StartupWMClass=OpenRB-338), a real GPU's Vulkan driver (the container used lavapipe; WGPU_BACKEND=gl failed there), and audio output through libasound2; (4) the aarch64 AppImage on arm64 hardware (it ran natively on Apple silicon under Docker Linux, but not on a distribution of its own).

From task-112 round 2: item (2) above is the open x86_64 check: run the x86_64 AppImage through its own runtime (--appimage-extract-and-run, then --screenshot under xvfb, and a symlink named orb) on the CI runner or an x86_64 host; locally only its unpacked AppRun ran (ORB_APPIMAGE_EMULATED path, never set in CI).

From task-113 (draft release job), still unverified, for the first tag run: (1) prepare accepts v<YYYY.MMDD.HHMM> and a tag like v1 fails there with the version.sh message and skips all jobs; (2) the release job downloads with the pattern {package-*,dmg,installer-*,appimage-*} and merge-multiple, and scripts/release-assets.sh finds exactly the 11 files (a missing or unexpected file means an artifact name differs); (3) gh release create --draft --verify-tag works with the job token and contents: write at job level, and the release is a draft with 12 assets (11 packages + SHA256SUMS.txt) and the changelog as body; (4) re-running the job deletes the old draft (found by gh api listing releases, since drafts have no tag lookup) and makes one new draft; a published release makes the job fail; (5) the binaries' About version equals the tag (v2026.1028.1930 -> 2026.1028.1930) and shows the hash without dev- when the tag is on main; (6) workflow_dispatch on a branch builds artifacts and skips the release job.

From task-135 (2026-10-29): scripts/appimage-smoke.sh now takes three screenshots (Vulkan, OpenGL, Vulkan ICDs hidden) and installs libegl1; only aarch64 in Docker was run. The x86_64 smoke (CI runner, or emulated locally) and a real desktop without a Vulkan driver are the open checks (new AC).

task-135 round 1: the smoke script now has four screenshots and asserts the backend from the ORB_GPU_INFO=1 line (Vulkan forced, Gl forced, automatic expects Vulkan, ICDs hidden expects Gl); the AC above covers the x86_64 run.

Correction (task-135 round 1): the earlier note saying three screenshots is superseded; there are four, with the backend asserted for each.

From task-136 (2026-10-29): the universal .dmg is gone; there are two, OpenRB-338-<version>-aarch64-apple-darwin.dmg and -x86_64-apple-darwin.dmg, from a dmg job matrix (artifacts dmg-<target>). The release job pattern is now {package-*,dmg-*,installer-*,appimage-*}, release-assets.sh expects 12 files (so the 11 in the notes above are 12, and 13 assets with SHA256SUMS.txt). Done locally: both images built, lipo -archs one arch each, codesign --verify --strict, arm64 app opened from the mounted image, x86_64 app ran under Rosetta (--screenshot). Open: the CI run of the matrix, and a real Intel Mac.

Supervisor, 2026-10-29, test release run (user's request): tag v2026.1029.1138 on e04aec6 (on origin/main, CI green), release.yml run 40866935635.
- AC #1: all 15 jobs passed (prepare, 6 package, 2 appimage, 2 installer, 2 dmg, draft release).
- AC #4 (partly): a DRAFT release with 13 assets: 12 packages (two .dmg: aarch64 13,541,173 B and x86_64 14,526,372 B) plus SHA256SUMS.txt, and a changelog body (237 commits, 125 listed). Not tried yet: re-running to replace the draft, and a non-matching tag such as v1.
- AC #3 (partly): every file name carries 2026.1029.1138. The aarch64-apple-darwin tar.gz passed its SHA256SUMS check, and its OpenRB-338 embeds '2026.1029.1138 (e04aec6)' with no dev- prefix; a --about --screenshot run worked. The other packages' version strings were not opened.
- AC #5 (first half): the x86_64 AppImage smoke step passed: WGPU_BACKEND=vulkan gives Vulkan, gl gives Gl, automatic gives Vulkan, and with no Vulkan ICD automatic gives Gl, all on llvmpipe; SMOKE-OK. A real Linux desktop without Vulkan is still to check.
- AC #6 (first half): both dmg jobs passed on their runners. The real Intel Mac check is still to do.
- AC #2 (real machines) is still to do.

User, 2026-10-29, real-machine checks of the v2026.1029.1138 draft:
- Work: both .dmg (aarch64 and x86_64) on the user's Mac; the darwin tar.gz on the same Mac; the x64 setup.exe on Windows 11; the Linux x86_64 UI binary (tar.gz) under WSL on Windows (WSLg).
- Fails: orb play of the Linux x86_64 binary under WSL. The output was "orb-player: audio output: ALSA function 'snd_pcm_avail_delay' failed with error 'I/O error (5)'", twice, then "orb: error: the audio output stopped with an error". Filed as task-139.
- Not yet: the AppImage (under WSL it needs libfuse2 or --appimage-extract-and-run), the arm64 installer, uninstall, and a real Intel Mac. The x86_64 .dmg was checked on the user's Mac; whether that is Apple silicon with Rosetta has not been recorded.

User, 2026-10-29:
- Under WSL, the Linux UI played sound and worked normally.
- The x86_64 AppImage was checked under WSL and works, sound included.
- The user has no Intel Mac and no arm64 Windows machine, and will not test those: the x86_64 .dmg on a real Intel Mac (AC #6) and the arm64 installer (AC #2) stay unchecked by hardware.

Supervisor, 2026-10-29, at the user's decision to accept CI for the hardware the user lacks:
- AC #6 (Intel Mac) and #2 (arm64 Windows) were reworded. Old #2 and #6 were removed and re-added as #5 and #6, so the old #3, #4 and #5 are now #2, #3 and #4.
- AC #1 is checked: run 40866935635, all 15 jobs passed.
- AC #6 is checked: both dmg jobs passed, each image is single-architecture, and the user checked both on their Mac.
<!-- SECTION:NOTES:END -->

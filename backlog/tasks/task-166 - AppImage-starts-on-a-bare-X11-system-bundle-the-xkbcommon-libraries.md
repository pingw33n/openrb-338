---
id: TASK-166
title: 'AppImage starts on a bare X11 system: bundle the xkbcommon libraries'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-31 08:05'
labels:
  - tools
milestone: m-5
dependencies: []
references:
  - scripts/package-appimage.sh
  - scripts/appimage-smoke.sh
  - scripts/docker/linux-build.Dockerfile
  - docs/building.md
  - README.md
  - 'https://github.com/AppImage/appimage.github.io/pull/11064'
priority: high
type: bug
ordinal: 182000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the AppImage catalog's maintainer opened github.com/AppImage/appimage.github.io/pull/11064 to list OpenRB-338.

Its test of OpenRB-338-2026.1030.1842-x86_64.AppImage failed on 2026-10-30. The test runs on an Ubuntu 22.04 GitHub runner under Xvfb with icewm, through firejail --net=none.

What the test log shows (run 41131157889 of AppImage/appimage.github.io):
- "thread 'main' panicked at xkbcommon-dl-0.4.2/src/x11.rs:59:28: Library libxkbcommon-x11.so could not be loaded".
- Our crash hook then saved a log, and the app exited without a window: "ERROR: The application exited within 11 seconds instead of showing a window".
- The runner's package list: sudo imagemagick libasound2-dev pulseaudio-utils alsa-utils alsa-oss libjack0 desktop-file-utils xmlstarlet xsel xterm xvfb icewm x11-utils x11-apps netpbm xdotool libgl1-mesa-dri libgl1-mesa-dev. It has no libxkbcommon-x11-0.

The AppImage bundles no library by design (scripts/package-appimage.sh:64, docs/building.md "The AppImage"). scripts/appimage-smoke.sh:41 installs libxkbcommon-x11-0 before it runs the app, so the case was never tested.

The test also warned that the README's first heading, "Features", is taken as the app's name: README.md starts with the logo picture and has no H1.

What:
1. Bundle the X11 keyboard libraries that winit loads with dlopen in the AppImage: libxkbcommon.so.0, libxkbcommon-x11.so.0 and libxcb-xkb.so.1. Take them from the build image (Ubuntu 22.04, scripts/docker/linux-build.Dockerfile) for each architecture. Make AppRun or the binary's rpath find them, for example LD_LIBRARY_PATH=$here/usr/lib, prepended so the bundled copies are used only when needed or always (choose and justify). Never bundle glibc, libasound, or any GL, EGL or Vulkan driver or loader.
2. Find any other library that a bare runner lacks. Reproduce the catalog's environment in an ubuntu:22.04 container with only the packages listed above, plus xvfb and icewm: run the AppImage with --screenshot (or plain, checking a window appears with xdotool) and fix every missing library the same way. Candidates are libxcursor, libxrandr and libxi; libX11, libxcb and libEGL may already come with the listed packages, so check.
3. scripts/appimage-smoke.sh:
   - stop installing the libraries that are now bundled, so the smoke test exercises the bundled copies;
   - allow exactly those files in its "no ELF file but OpenRB-338 and orb" check.
   Keep the no-glibc check.
4. README.md: add a "# OpenRB-338" heading so the catalog reads the name. Put it below the logo, or make the logo part of the heading, keeping the centered logo look.
5. docs/building.md "The AppImage" and the target table: say which libraries are bundled, why, and where they come from.
6. Run scripts/package-appimage.sh and scripts/appimage-smoke.sh for x86_64 (and aarch64 if this Mac can, otherwise say so). Run shellcheck on the changed scripts.
Do not push, do not tag, and do not comment on the catalog PR: the supervisor makes the release and asks the user before /retest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The x86_64 AppImage shows its window, or writes its --screenshot, in an ubuntu:22.04 container with only the catalog runner's package list (no libxkbcommon-x11-0); the command and output are in the notes
- [ ] #2 The AppImage bundles libxkbcommon.so.0, libxkbcommon-x11.so.0, libxcb-xkb.so.1 (and any other library step 2 found), and no glibc, libasound or GL/EGL/Vulkan library
- [ ] #3 scripts/appimage-smoke.sh no longer installs the bundled libraries and passes for x86_64 (aarch64 too, or the reason it was not run is in the notes)
- [ ] #4 README.md has a '# OpenRB-338' heading before 'Features', and its top still shows the centered logo
- [ ] #5 docs/building.md describes the bundled libraries; shellcheck is clean on the changed scripts
<!-- AC:END -->

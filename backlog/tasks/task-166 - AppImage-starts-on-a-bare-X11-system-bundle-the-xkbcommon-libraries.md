---
id: TASK-166
title: 'AppImage starts on a bare X11 system: bundle the xkbcommon libraries'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-31 08:05'
updated_date: '2026-10-31 08:27'
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
- [x] #1 The x86_64 AppImage shows its window, or writes its --screenshot, in an ubuntu:22.04 container with only the catalog runner's package list (no libxkbcommon-x11-0); the command and output are in the notes
- [x] #2 The AppImage bundles libxkbcommon.so.0, libxkbcommon-x11.so.0, libxcb-xkb.so.1 (and any other library step 2 found), and no glibc, libasound or GL/EGL/Vulkan library
- [x] #3 scripts/appimage-smoke.sh no longer installs the bundled libraries and passes for x86_64 (aarch64 too, or the reason it was not run is in the notes)
- [x] #4 README.md has a '# OpenRB-338' heading before 'Features', and its top still shows the centered logo
- [x] #5 docs/building.md describes the bundled libraries; shellcheck is clean on the changed scripts
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Bundle libxkbcommon.so.0, libxkbcommon-x11.so.0, libxcb-xkb.so.1 in appimage-pack.sh (Linux side, from the 22.04 image or runner via ldconfig -p), AppRun prepends usr/lib to LD_LIBRARY_PATH. 2. Dockerfile and release.yml install libxkbcommon-x11-0. 3. Reproduce the catalog runner in a container, old vs new AppImage. 4. Smoke test: stop installing the libs, assert absent, allow the five ELF files. 5. README heading, docs/building.md.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: worktree-agent-a8a1543006c473490

Binaries: a full emulated x86_64 cargo build is too slow, so the binaries for both architectures were extracted from the released v2026.1030.1842 AppImages (same sources; the pack step does not care) into dist/<target>/bin, then SKIP_BUILD=1 SOURCE_DATE_EPOCH=1793215800 scripts/package-appimage.sh for both targets. x86_64 packs in the emulated fallback (ORB_APPIMAGE_EMULATED), aarch64 natively.
Design: always bundled (LD_LIBRARY_PATH prepended): one tested version, no too-old host copy; the libs only need glibc 2.33 (objdump -T: libxkbcommon 2.33, others <=2.17) and libxcb (host). Libs come from the build image (Dockerfile now installs libxkbcommon-x11-0); release.yml appimage job installs it on the runner; the pack script fails when ldconfig -p lacks one.
Catalog reproduction (container ubuntu:22.04 linux/amd64, packages imagemagick libasound2-dev pulseaudio-utils alsa-utils alsa-oss libjack0 desktop-file-utils xmlstarlet xsel xterm xvfb icewm x11-utils x11-apps netpbm xdotool libgl1-mesa-dri libgl1-mesa-dev; Xvfb :99 + icewm, unpacked AppRun because Docker Desktop cannot exec the x86_64 runtime): ldconfig has libxkbcommon.so.0 but no libxkbcommon-x11 or libxcb-xkb. OLD (released 2026.1030.1842): --demo --screenshot panics 'Library libxkbcommon-x11.so could not be loaded' (xkbcommon-dl x11.rs:59:28), plain run exits with no window. NEW: plain run, xdotool search --name OpenRB found the window after 2 s (title OpenRB-338, root grab 1280x1024 PNG), --demo --screenshot wrote 203967 B. Step 2: libXcursor, libXrandr, libXi, libX11, libxcb, libEGL.so.1 were all present on that runner (pulled by xvfb/x11-apps/mesa-dev), so no other library is needed; none bundled.
Smoke: scripts/appimage-smoke.sh passed SMOKE-OK for x86_64 (emulated AppRun path) and aarch64 (native): five ELF files, no glibc, GLIBC max 2.35, four backend screenshots; the container fails if a keyboard lib is installed. shellcheck clean on the three scripts.
README: kept the logo, added a centered visible '# OpenRB-338' below it (a heading with only an image has no text for the catalog's parser). Not verified against the catalog's parser; the supervisor sees it on /retest.
Cleanup: removed the ubuntu image pulled by the runs; the project build images stay (rebuilt with the Dockerfile change).

Review round 1 (send back), fixed in the amended commit:
1-3. docs/building.md: libxkbcommon-x11-0 removed from the host list of 'What may ship' (kept as a .tar.gz note), usr/lib row in the AppDir table, the garbled sentence rewritten.
4. LD_LIBRARY_PATH removed from AppRun. appimage-pack.sh now runs patchelf --set-rpath: RUNPATH $ORIGIN/../lib on usr/bin/OpenRB-338 and usr/bin/orb (the AppDir copies only; the tar.gz binaries stay unchanged) and RUNPATH $ORIGIN on the three libs (RUNPATH is not transitive). patchelf is installed by the build image (Dockerfile) and the release.yml appimage job. Verified in ubuntu:22.04 amd64 with LD_LIBRARY_PATH empty, LD_DEBUG=libs xvfb-run AppRun --demo --screenshot: 'trying file=/tmp/new/usr/bin/../lib/libxkbcommon-x11.so.0' (also libxkbcommon.so.0, libxcb-xkb.so.1), 'calling init' for all three from usr/bin/../lib; screenshot written. appimage-smoke.sh now asserts the RUNPATHs (readelf -d) and the LD_DEBUG load paths, so it checks this itself.
5. appimage-pack.sh filters ldconfig -p on the cache tag (libc6,x86-64 / libc6,AArch64) from ARCH.
Commands. Package: SKIP_BUILD=1 SOURCE_DATE_EPOCH=1793215800 scripts/package-appimage.sh x86_64-unknown-linux-gnu (and aarch64-unknown-linux-gnu). Smoke: scripts/appimage-smoke.sh dist/<target>/OpenRB-338-2026.1028.1930-<arch>.AppImage -> SMOKE-OK both (x86_64 through the emulated unpacked AppRun, aarch64 through --appimage-extract-and-run). Catalog container (AC1): docker run --rm --platform linux/amd64 ubuntu:22.04@sha256:e0f80c82... ; apt-get install --no-install-recommends imagemagick libasound2-dev pulseaudio-utils alsa-utils alsa-oss libjack0 desktop-file-utils xmlstarlet xsel xterm xvfb icewm x11-utils x11-apps netpbm xdotool libgl1-mesa-dri libgl1-mesa-dev squashfs-tools binutils; unsquashfs -o <end of ELF section headers> old and new AppImage; Xvfb :99 + icewm; for each: AppRun in the background and poll xdotool search --name OpenRB (old: exits, no window; new: window after 2 s), then AppRun --demo --screenshot /tmp/s.png (old: panic at xkbcommon-dl x11.rs:59; new: 203967 B). Rerun after the fix gave the same results. shellcheck clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
AppImage bundles libxkbcommon.so.0, libxkbcommon-x11.so.0 and libxcb-xkb.so.1 in usr/lib, found through RUNPATH set by patchelf in appimage-pack.sh ($ORIGIN/../lib on the two programs, $ORIGIN on the libraries; no LD_LIBRARY_PATH, so nothing leaks into children such as zenity). Libs come from the 22.04 image/runner, filtered by architecture. Reproduced the catalog failure with the released AppImage in a catalog-like ubuntu:22.04 container (panic, no window) and the new x86_64 AppImage shows its window and writes the screenshot. appimage-smoke.sh no longer installs the libs, asserts RUNPATH and load paths, and passes for x86_64 and aarch64. README heading, docs/building.md and release.yml (libxkbcommon-x11-0, patchelf) updated. Branch worktree-agent-a8a1543006c473490, one commit (amended after review round 1). No Rust touched.
<!-- SECTION:FINAL_SUMMARY:END -->

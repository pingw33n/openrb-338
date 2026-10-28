---
id: TASK-112
title: 'Linux: AppImage of the UI (amd64 and arm64; local script and GitHub Actions)'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 10:19'
updated_date: '2026-10-28 19:42'
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
ordinal: 132000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user chose AppImage as the Linux package (2026-10-28): one file that runs on most distros without installing. There are two variants, x86_64 and aarch64. The CLI stays in task-108's .tar.gz.

What:
- A script builds OpenRB-338-<version>-x86_64.AppImage and -aarch64.AppImage from task-108's Linux builds of rb-ui (the binary named OpenRB-338 per task-104). It uses linuxdeploy or appimagetool, with:
  - an AppRun;
  - a .desktop file (Name=OpenRB-338, Categories=AudioVideo;Audio;Music);
  - an icon PNG rendered from the app_icon part (rb_ui::art, resvg) at 256 px and other sizes;
  - the license.
- Bundle only what the target system may lack. Graphics and audio libraries (libGL/EGL, X11/Wayland, libasound) normally come from the host; say which libraries are bundled and why.
- Also put the CLI (rb, later orb) inside the AppImage? Optional; say yes or no and why.
- GitHub Actions: release.yml builds both AppImages, on ubuntu (x86_64) and ubuntu-24.04-arm (aarch64), or cross. Build on the oldest Ubuntu LTS runner available so the glibc requirement is low, and record the minimum glibc. Upload both.
- Local: build from this Mac if practical (for example Docker with the Linux build); otherwise in CI only, and say so.
- No signing (the user, 2026-10-28).
- Check: run the x86_64 AppImage in a Linux container or VM (for example docker with xvfb: it starts and renders a --screenshot) and record the result. The aarch64 one at least in CI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 OpenRB-338-<version>-x86_64.AppImage and -aarch64.AppImage are built (CI for both; locally where practical) with desktop entry and icon; the minimum glibc is recorded
- [ ] #2 The x86_64 AppImage was started headless (xvfb, --screenshot) in a container or VM, with the result in the notes
- [ ] #3 docs/building.md describes it; release.yml uploads both; validated with actionlint (or act)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-110 (user decision 2026-10-28): the version of every package is the UTC build time, never the Cargo version (0.1.0). File names use scripts/version.sh = YYYY_MM_DD_HH_MM_SS from SOURCE_DATE_EPOCH, e.g. OpenRB-338-2026_10_28_19_30_00-x64-setup.exe or ...-x86_64.AppImage (the release-tag form of task-113); human text uses '2026-10-28 19:30:00 UTC'; where a format wants numbers (Windows file version, macOS plist) use YYYY.MMDD.HHMMSS style numeric parts. scripts/version.sh is the single source. The built-in kit may ship in public packages. Scheme and examples: docs/building.md, 'The version is the build time'. The macOS bundle id is net.emphased.openrb338 (the Windows and Linux ids should follow that domain).
<!-- SECTION:NOTES:END -->

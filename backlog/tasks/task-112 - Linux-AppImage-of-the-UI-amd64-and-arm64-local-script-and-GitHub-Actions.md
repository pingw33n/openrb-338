---
id: TASK-112
title: 'Linux: AppImage of the UI (amd64 and arm64; local script and GitHub Actions)'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 10:19'
updated_date: '2026-10-29 06:00'
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
- [x] #1 OpenRB-338-<version>-x86_64.AppImage and -aarch64.AppImage are built (CI for both; locally where practical) with desktop entry and icon; the minimum glibc is recorded
- [x] #2 docs/building.md describes it; release.yml uploads both; validated with actionlint (or act)
- [x] #3 The aarch64 AppImage was started through its runtime (--appimage-extract-and-run) headless (xvfb, --screenshot) in an ubuntu:22.04 container, with the result in the notes; the x86_64 one waits for a CI runner or x86_64 host (task-134)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. scripts/package-appimage.sh TARGET: build (scripts/build.sh, profile dist) unless SKIP_BUILD; stage an AppDir (usr/bin/OpenRB-338 and rb, AppRun, .desktop, hicolor icons rendered by ui-art iconset, LICENSE); pack with appimagetool 1.9.1 (pinned by URL and sha256, per arch) to dist/<target>/OpenRB-338-<version>-<arch>.AppImage. Packing runs natively on a matching Linux host and inside the Ubuntu 22.04 image (docker) elsewhere; appimagetool runs with --appimage-extract-and-run (no FUSE).
2. AppRun: runs OpenRB-338; as a symlink named rb, or with first argument --rb, runs the bundled rb (decision: yes, rb is inside).
3. scripts/appimage-smoke.sh: smoke test in stock ubuntu:22.04 (rb --help, rb info, --screenshot under xvfb); readelf check that no glibc is bundled.
4. release.yml appimage job per arch (ubuntu-22.04 and ubuntu-22.04-arm) after package, using bin-<target> artifacts; pin action SHAs (already used ones); actionlint.
5. docs/building.md: the AppImage section, libs the host provides, min glibc 2.35, rb choice. Unrunnable-here criteria go to task-134 notes.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From task-110 (user decision 2026-10-28): the version of every package is the UTC build time, never the Cargo version (0.1.0). File names use scripts/version.sh = YYYY_MM_DD_HH_MM_SS from SOURCE_DATE_EPOCH, e.g. OpenRB-338-2026_10_28_19_30_00-x64-setup.exe or ...-x86_64.AppImage (the release-tag form of task-113); human text uses '2026-10-28 19:30:00 UTC'; where a format wants numbers (Windows file version, macOS plist) use YYYY.MMDD.HHMMSS style numeric parts. scripts/version.sh is the single source. The built-in kit may ship in public packages. Scheme and examples: docs/building.md, 'The version is the build time'. The macOS bundle id is net.emphased.openrb338 (the Windows and Linux ids should follow that domain).

Branch: worktree-agent-a3e89d59df1d32bbf. Version form is YYYY.MMDD.HHMM (task-133); the older note above (YYYY_MM_DD_HH_MM_SS) is superseded. docker info: the client works but the daemon is NOT running (no ~/.docker/run/docker.sock); I did not start it, so no AppImage could be built or smoke-tested locally on this Mac.

Evidence: appimagetool 1.9.1 assets downloaded (curl) and their sha256 equal the digests of 'gh api repos/AppImage/appimagetool/releases/tags/1.9.1': x86_64 ed4ce84f0d9c...eb0 (15092216 B), aarch64 f0837e7448a0...158 (14678536 B); pinned in scripts/package-appimage.sh. Local stub run (fake OpenRB-338 and rb scripts, SKIP_BUILD=1, SOURCE_DATE_EPOCH=1793215800): the script rendered the icons with ui-art (PNG 256x256 .DirIcon, 64x64 hicolor), wrote the AppDir, .desktop (X-AppImage-Version=2026.1028.1930), AppRun routes: 'a b' -> ui, '--rb x' -> cli, ARGV0=./links/rb -> cli; it then stopped at 'docker build' because the daemon is not running (not started by me). So no AppImage was built; no image pulled; no container started; nothing to remove. Decisions: rb is inside (AppRun: symlink named rb, or --rb first); no library bundled (host provides libasound2, X11/Wayland, GL/Vulkan; glibc minimum 2.35 from task-108's objdump finding, not re-measured here); appimagetool without --no-appstream/--comp (defaults; not testable here). release.yml: new appimage job per arch (ubuntu-22.04, ubuntu-22.04-arm) with only action SHAs already in the file (checkout, download-artifact, rust-cache, upload-artifact; already verified by earlier tasks), smoke step in CI. Checks: cargo +nightly fmt --check, clippy -D warnings, cargo test --release 586 passed 0 failed, shellcheck scripts/*.sh, actionlint all clean. CI-only criteria moved to task-134 notes.

Review round 1 (supervisor, send back; Docker daemon started by the user, server 29.8.2). A: built both AppImages with scripts/package-appimage.sh (SOURCE_DATE_EPOCH=1793215800, binaries from build.sh in the Docker image) and ran scripts/appimage-smoke.sh on each in ubuntu:22.04: x86_64 OpenRB-338-2026.1028.1930-x86_64.AppImage 15317496 B sha256 088ec4703e34fb2b9e1e9100e91ef857a1bf1b4b4ec5359f962d0939c92827a1; aarch64 ...-aarch64.AppImage 14547464 B sha256 def28e2cd6f92bedbb5e618e6fdbbf911bfba8f460a4d83a32f236ad8178a9d7 (a second packing run gave the same sha). Smoke output (both): desktop entry valid, six hicolor icons, exactly two ELF files (usr/bin/OpenRB-338, usr/bin/rb), no glibc file, objdump -T GLIBC maximum GLIBC_2.35 (OpenRB-338) and GLIBC_2.34 (rb) on both architectures, rb --help and rb info (Calm.rbs: Tempo 130 BPM, Mode song...) , symlink named rb gave the same help, PNG signature ok (191621 B aarch64, 191815 B x86_64), SMOKE-OK. Found while doing it: (a) appimagetool needs the file command (added file to scripts/docker/linux-build.Dockerfile; the image was rebuilt); (b) in Docker Desktop's amd64 emulation the x86_64 AppImage runtime and the x86_64 appimagetool give Exec format error (ELF e_ident ABI version 65, the AppImage marker), although the static x86_64 programs inside run: new scripts/appimage-pack.sh falls back to unsquashfs -o (offset = e_shoff + e_shnum*e_shentsize) and runs the unpacked appimagetool with its helpers on PATH (squashfs-tools added to the image); appimage-smoke.sh does the same for the x86_64 AppImage and runs squashfs-root/AppRun, ARGV0 set by hand for the symlink test: so the x86_64 runtime itself was not run (said in docs and task-134). Items: 1 chmod: now inside appimage-pack.sh (container or native), not on the host. 2 GLIBC maximum asserted <= GLIBC_2.35 with sort -V. 3 PNG signature 89504e470d0a1a0a asserted. 4 rb output goes to files, then grep/cmp; no pipe into head. 5 libvulkan1 installed explicitly; backend measured on aarch64: WGPU_BACKEND=vulkan screenshot ok (lavapipe), WGPU_BACKEND=gl and VK_ICD_FILENAMES=/nonexistent both failed with FailedToCreateSurfaceForAnyBackend (no XDG_RUNTIME_DIR noise for gl), so the smoke test sets WGPU_BACKEND=vulkan and docs say a Vulkan driver is needed. 6 docs reworded (not the only download; the tar.gz). 7 ARGV0 verified: aarch64 AppImage run natively under --appimage-extract-and-run, ./links/rb printed the same help as --rb --help; AppRun needs no fallback. 8 commit body lines <= 72 columns. Fmt, clippy, 586 release tests, shellcheck, actionlint clean. Docker leftovers: no container of mine (buildx_buildkit_open-luxp-builder0 predates my work); images built: openrb-linux-build:aarch64 and :x86_64; pulled: ubuntu:22.04 (digest e0f80c82...); volumes openrb-cargo/rustup/target-<arch> (build caches) kept; dist/ is git-ignored.

Review round 2 (supervisor, send back; item 3 corrected: stay Done). 1 appimage-pack.sh: the unsquashfs fallback runs only when OPENRB_APPIMAGE_EMULATED is set; otherwise a runtime failure fails the script. package-appimage.sh sets it for x86_64 on a Darwin host only (passed into docker run). Added trap to remove the work dir on failure. 2 appimage-smoke.sh: the EMULATED branch is gated on the same variable (set automatically for linux/amd64 on Darwin); otherwise --appimage-extract must succeed. release.yml has no EMULATED (grep count 0). 3 AC split: the old AC2 (x86_64 started with --screenshot) is replaced by AC3, aarch64 only, checked: the aarch64 AppImage ran through its runtime with --screenshot under xvfb in ubuntu:22.04 (PNG 191621 B). The x86_64 runtime run (and --screenshot through it) is open and listed in task-134 note (2); only a CI runner or an x86_64 host can finish it. 4 docs/building.md Checked says the emulated run is local only. Reran: x86_64 with the variable set (auto on this Mac): packed, smoke SMOKE-OK with the EMULATED line, same sha256 088ec470...; aarch64 without it: packed, SMOKE-OK, sha256 def28e2c... unchanged. shellcheck and actionlint clean.

Review round 3: the emulation flag is no longer inherited. appimage-smoke.sh starts from emulated= and sets 1 only for Darwin plus linux/amd64; package-appimage.sh clears it in the native call (OPENRB_APPIMAGE_EMULATED='') and sets it only in the Darwin x86_64 docker branch; appimage-pack.sh and the smoke container require the value to be exactly 1. Gate test: export OPENRB_APPIMAGE_EMULATED=1, then package-appimage.sh and appimage-smoke.sh on aarch64 (native in its container): no unsquashfs or EMULATED line, SMOKE-OK, sha256 def28e2c... unchanged. shellcheck clean (SC1007 fixed with '').
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added scripts/package-appimage.sh (AppDir, appimagetool 1.9.1 pinned by sha256, icons from ui-art, .desktop, AppRun with the rb route, LICENSE), scripts/appimage-pack.sh, scripts/appimage-smoke.sh, file and squashfs-tools in the build image, an appimage job per architecture in release.yml and docs/building.md 'The AppImage'. Built and smoke tested locally in Docker: OpenRB-338-2026.1028.1930-x86_64.AppImage (15317496 B) and -aarch64.AppImage (14547464 B); glibc minimum 2.35 (GLIBC_2.35 / GLIBC_2.34 measured, none bundled). The aarch64 AppImage ran through its runtime including --screenshot under xvfb (AC3, checked). The x86_64 runtime cannot be exec-ed in Docker Desktop's emulation, so only locally and only with OPENRB_APPIMAGE_EMULATED (never set by CI) its unpacked AppRun was run: the x86_64 runtime check is not done and only a CI runner or x86_64 host can finish it; it is listed in task-134, with the CI jobs, a real desktop and arm64 hardware. AC split: the old AC2 became the aarch64-only AC3; the task stays Done. Fmt, clippy, 586 tests, shellcheck, actionlint clean. Branch worktree-agent-a3e89d59df1d32bbf, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

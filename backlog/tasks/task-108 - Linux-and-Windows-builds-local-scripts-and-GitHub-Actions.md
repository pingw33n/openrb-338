---
id: TASK-108
title: 'Linux, Windows and macOS builds: local scripts and GitHub Actions'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 09:34'
updated_date: '2026-10-28 15:20'
labels:
  - tools
  - cli
  - ui
milestone: m-4
dependencies:
  - TASK-067
references:
  - Cargo.toml
  - rust-toolchain.toml
  - AGENTS.md
priority: medium
type: feature
ordinal: 128000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for builds of rb (the CLI) and rb-ui for Linux and Windows. They should be made locally by scripts and automated with GitHub Actions. Today the project is built only with cargo on this Mac.

Context:
- The workspace is at the repository root. The toolchain is pinned in rust-toolchain.toml.
- rb-ui uses eframe/winit (on Linux: X11 or Wayland, dlopened) and cpal audio (on Linux: libasound).
- After task-067 the standard mod is embedded (assets/Standard.rbm), so the binaries need no Propellerhead file at run time.
- The repository is private (github.com/pingw33n/rb; to be renamed open-rb-338 later, task-068).

What:
1. Local scripts under scripts/ (shell, runnable on this Mac) that build release binaries of rb and rb-ui:
   - for x86_64 Linux (gnu) and x86_64 Windows (msvc or gnu: choose and say why), and for aarch64 Linux if cheap;
   - into dist/<target>/, packaged as .tar.gz (Linux) and .zip (Windows) with the license and a short readme.
   - Cross-compiling from macOS: pick a tool and document how to install it. Candidates: cargo-zigbuild for Linux; cargo-xwin, or mingw with cross, for Windows. Note what cannot be cross-built (for example Linux audio headers) and how the script handles it, such as a Docker fallback.
   - The script for the host build (macOS) as well, so all are uniform.
2. GitHub Actions workflows in .github/workflows/:
   - ci.yml on push and pull requests: fmt --check, clippy -D warnings and cargo test on ubuntu-latest, windows-latest and macos-latest. Tests that need original/, RB_MOD_DIR or other non-committed files skip as they do today; verify that they skip cleanly.
   - release.yml on a tag (v*) or manual dispatch: builds the packages on native runners for Linux, Windows and macOS, and uploads them as workflow artifacts (and as release assets for a tag).
   - Cache cargo. Pin action versions.
   - Workflows must not need any secret, and must never fetch or contain Propellerhead files other than the committed assets/Standard.rbm.
3. Document the scripts and the workflows in AGENTS.md (Build and test) and in a new docs/building.md.
4. Run the local scripts here and record the produced files and sizes in the notes. Push nothing: the supervisor pushes main. To check the workflows before merge, run them with act or validate them with actionlint, and record which. After the merge the supervisor reports the first real CI run.
- Out of scope: code signing, notarization, installers (.msi, .deb) and an .app bundle. Note them as follow-ups in the final summary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 AGENTS.md and docs/building.md describe the scripts and workflows; cargo test and clippy pass
- [x] #2 scripts/ builds release packages of rb and rb-ui for Linux x86_64, Windows x86_64 and macOS (aarch64 and x86_64, or universal) from this Mac; tool requirements and limits are documented; the produced packages are listed in the notes
- [x] #3 release.yml builds rb and rb-ui packages for all six targets (Linux, Windows, macOS x amd64, arm64); the local scripts build every pair practical from this Mac and document the skipped ones; tool requirements are documented; the produced packages are listed in the notes
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. scripts/package.sh TARGET: build rb and OpenRB-338 for one target and pack dist/<target>/ (tar.gz or zip with LICENSE and README.txt). Method by target: apple-darwin = native cargo (rustup target); windows-msvc = cargo-xwin off Windows (amd64 and arm64), plain cargo on Windows; linux-gnu = Docker (ubuntu 22.04, glibc 2.35) off Linux, plain cargo on Linux.
2. scripts/build-all.sh runs every target practical from the host and lists skipped ones. SOURCE_DATE_EPOCH and RB_BUILD_BRANCH pass through.
3. .github/workflows/ci.yml (nightly fmt, clippy, test on 3 OS) and release.yml (6 native builds, artifacts, release assets left to task-113).
4. docs/building.md, AGENTS.md, LICENSE (MIT, already declared in Cargo.toml).
5. Run the scripts, actionlint, fmt/clippy/tests; record sizes.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Scope widened by the user (2026-10-28): macOS is a first-class target. scripts/ also builds macOS packages for aarch64-apple-darwin and x86_64-apple-darwin (both from this Mac; a universal binary via lipo if cheap, else two packages), as .tar.gz of rb and rb-ui; release.yml builds them on macos runners too. The .dmg with an .app bundle and the Windows installer .exe are separate tasks (task-110, task-111) that build on this task's scripts and workflows.

Targets decided by the user (2026-10-28): every OS has an amd64 and an arm64 variant: x86_64/aarch64-unknown-linux-gnu, x86_64/aarch64-pc-windows-msvc (or -gnu/-gnullvm for arm64 if msvc is impractical; say which), x86_64/aarch64-apple-darwin. GitHub Actions builds all six, on native runners where they exist (ubuntu-24.04-arm, windows-11-arm, macos-14 for arm64; ubuntu-latest, windows-latest, macos-13 for amd64) or by cross-compiling. The local scripts may skip pairs that are hard to build from this Mac; list which are skipped and why. No code signing anywhere (the user, 2026-10-28).

From task-120: formatting is enforced by nightly rustfmt (rustfmt.toml has unstable options). ci.yml must add a fmt job: rustup toolchain install nightly-2026-10-18 --profile minimal --component rustfmt, then cargo +nightly-2026-10-18 fmt --all --check. Build, clippy and tests stay on the pinned stable toolchain. Also name the pinned nightly in docs/building.md when this task writes it (AGENTS.md already names it).

Branch: worktree-agent-abb567089ef0b384c

Packages built by scripts/build-all.sh on this Mac (aarch64) with SOURCE_DATE_EPOCH=1793181022, in dist/TARGET/:
- aarch64-apple-darwin  OpenRB-338-0.1.0-aarch64-apple-darwin.tar.gz  15365863 B (native cargo)
- x86_64-apple-darwin   OpenRB-338-0.1.0-x86_64-apple-darwin.tar.gz   15881082 B (rustup target, Apple linker)
- x86_64-pc-windows-msvc  OpenRB-338-0.1.0-x86_64-pc-windows-msvc.zip   16419064 B (cargo-xwin; PE32+ x86-64)
- aarch64-pc-windows-msvc OpenRB-338-0.1.0-aarch64-pc-windows-msvc.zip  15648301 B (cargo-xwin; PE32+ Aarch64)

SKIPPED locally: both Linux targets. Docker Desktop was not running (the docker socket is missing; I was told not to start it), so the Docker path (scripts/docker/linux-build.Dockerfile, build_docker in build.sh) is written but NOT run. CI builds Linux natively on ubuntu-22.04 and ubuntu-22.04-arm. build-all.sh printed: skipped, Docker is not running. So AC2's Linux x86_64 is CI-only for now. The Windows zips and macOS tars were not run on their OS (cross-built; file(1) confirms the formats).
Failed hypothesis: cargo-xwin's clang-cl 21 turns the vendored libvorbis call _mm_prefetch(float*) into an error (aotuv_lancer_vorbis_sys mdct.c:983). CFLAGS_target, CC_target and a PATH shim are all overwritten or bypassed by cargo-xwin; the CL env var (read by clang-cl itself) works. MSVC cl in CI only warns.

Tools installed (all in docs/building.md): brew install actionlint llvm shellcheck; cargo install cargo-xwin --locked; rustup target add --toolchain 1.99.1 x86_64-apple-darwin x86_64-pc-windows-msvc aarch64-pc-windows-msvc.
Validation: actionlint 1.7.12 on both workflows: no findings. shellcheck on scripts: clean. act not used (it has no macOS or Windows runners). Nightly fmt check ok; clippy --all-targets -D warnings ok; cargo test --workspace (debug) with RB_MOD_DIR, RB_CD_ISO and RB_THIRDPARTY_DIR unset: 538 passed, 0 failed, 7 ignored (the gated tests skip cleanly); cargo test --release -q with RB_MOD_DIR: 542 passed, 0 failed, 3 ignored.
Unverified until the first CI run: windows-11-arm (rustup install step, 7z for the zip, MSVC arm64 tools), ubuntu-22.04-arm, the runner labels (macos-15-intel replaces the retired macos-13), the Linux apt step, Bash on the Windows runners in package.sh. Actions are pinned by SHA (checkout v7.0.1, upload-artifact v7.0.2, rust-cache v2.9.3).

Hooks for tasks 110-113: scripts/build.sh leaves dist/TARGET/bin/ (also uploaded as artifact bin-TARGET), scripts/version.sh, scripts/release-env.sh (SOURCE_DATE_EPOCH is now; task-113 takes it from the tag); release.yml has no release job yet. The Windows exe is still console-subsystem (task-111).
Decision flagged: no LICENSE file existed (Cargo.toml says MIT); I added the MIT text with 'Copyright (c) 2026 the OpenRB-338 authors' so the packages can ship it. The user may want another holder name.
Follow-ups, out of scope: signing and notarization, .app and .dmg (110), installer (111), AppImage (112), draft release (113), README (109), a universal macOS binary.

Decision by the user (2026-10-28): the project license is GPL-3.0-only, not MIT. Applied: LICENSE is the verbatim GPLv3 text (gnu.org gpl-3.0.txt, 674 lines, 35149 bytes, the canonical size); license in the workspace Cargo.toml is GPL-3.0-only (all seven crates inherit it); package README.txt and docs/building.md name GPL-3.0-only; every package ships LICENSE. Cargo metadata shows GPL-3.0-only for all crates; cargo build --locked ok. The earlier note about the MIT holder name is void. No per-file headers added.

Linux built locally with Docker (user started it; rebased on 7982389). Packages (SOURCE_DATE_EPOCH=1793181022): OpenRB-338-0.1.0-aarch64-unknown-linux-gnu.tar.gz 17980244 B (2m29s build, native arm64 container); OpenRB-338-0.1.0-x86_64-unknown-linux-gnu.tar.gz 18957970 B (11m12s, emulated amd64). Minimum glibc: highest GLIBC symbol is 2.35 (OpenRB-338) and 2.34 (rb), both architectures, so glibc 2.35 (Ubuntu 22.04).
Smoke test in ubuntu:22.04 containers (amd64 and arm64): rb --help and rb info on tests/songs/shipped/G-Spot.rbs work; OpenRB-338 --help works; OpenRB-338 --screenshot under xvfb wrote a PNG (191 KB) on both (software Vulkan). Found: both rb and OpenRB-338 link libasound.so.2, so even rb --help fails without libasound2; OpenRB-338 also needs libxcursor1, libxrandr2, libxi6, libxkbcommon-x11-0 and a GL or Vulkan driver. Documented in docs/building.md. Fixed: tar of the Linux and macOS packages carried macOS xattr headers (tar warnings on extract): package.sh uses --no-xattrs.
Docker left behind: no containers running. Images pulled or built by me: ubuntu:22.04 (amd64 and arm64), openrb-linux-build:x86_64, openrb-linux-build:aarch64; volumes openrb-cargo-*, openrb-rustup-*, openrb-target-* (build caches; docker volume rm to drop). The buildkit container and other images were already there.
User decisions (2026-10-28): GPL-3.0-only; the cost of six release package jobs and the 3-OS CI on the private repo is approved; accepting Microsoft's SDK/CRT license via XWIN_ACCEPT_LICENSE=1 for local cargo-xwin builds is approved; six targets, no signing.
Review round 1 (all fixed): 1 host CPU check moved into the Linux branch of build.sh (no exit on unknown CPU for macOS or Windows); 2 build.sh and docs agree: a Linux target on a Linux host of the other architecture goes to Docker with qemu; 3 build-all.sh checks Docker for any Linux target not native on the host and reports a skip; 4 cargo-xwin cache path named for macOS and Linux in build.sh and docs; 5 Dockerfile base pinned by the ubuntu:22.04 index digest sha256:e0f80c82..., and the unchecksummed rustup fetch is noted there; 6 release.yml fetches rustup-init.exe from static.rust-lang.org/rustup/dist/aarch64-pc-windows-msvc; 7 ci.yml push only on main plus pull_request; 8 action SHAs verified with the GitHub API against tags (checkout v7.0.1 = commit 3d3c42e5, upload-artifact v7.0.2 = commit cf430e03, rust-cache v2.9.3 is an annotated tag 2201cc58 dereferencing to commit d1527c8f, matching the pins); runner labels ubuntu-22.04, ubuntu-22.04-arm, windows-latest, windows-11-arm, macos-15, macos-15-intel are all in the runner-images README; 9 Decision line added to the commit; 10 AC2 stays ticked: the Linux x86_64 package builds and runs here now.
Checks after the round: nightly fmt ok, clippy ok, cargo test --release --workspace with RB_MOD_DIR 551 passed, 0 failed; shellcheck and actionlint clean.

Review round 2 (docs only): assets/ui/README.md license line now says GPL-3.0-only; docs/building.md:10 reflowed to under 100 columns. Unverified until the first CI run: the Windows arm64 cross build with cargo-xwin was not built from the arm64 runner's side (the local aarch64-pc-windows-msvc package is a cross build from this Mac, never run on Windows), and the 7z fallback in package.sh (the zip tool on Windows runners) was never run.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added scripts/build.sh, package.sh, build-all.sh, version.sh, release-env.sh and a Linux build Dockerfile (base image pinned by digest); .github/workflows/ci.yml (nightly-2026-10-18 fmt, clippy and tests on Linux, Windows, macOS; pushes to main and pull requests) and release.yml (six packages on native runners as artifacts, with bin-TARGET artifacts for tasks 110-113); docs/building.md; AGENTS.md; LICENSE (verbatim GPLv3; the project is GPL-3.0-only). Built locally on this Mac, all with SOURCE_DATE_EPOCH=1793181022: aarch64 and x86_64 macOS (tar.gz, 15.4 and 15.9 MB), x86_64 and aarch64 Windows msvc (zip via cargo-xwin, 16.4 and 15.6 MB), aarch64 and x86_64 Linux (tar.gz via Docker, 18.0 and 19.0 MB; minimum glibc 2.35). The Linux packages were smoke-tested in ubuntu:22.04 containers (rb --help, rb info, OpenRB-338 --screenshot under xvfb); both programs need libasound2 at run time. Validated with actionlint and shellcheck; fmt, clippy and tests pass (551 release tests with RB_MOD_DIR). Review round 1 fixed. Branch worktree-agent-abb567089ef0b384c, one commit. The first CI run must confirm windows-11-arm and ubuntu-22.04-arm. Follow-ups: signing, tasks 110-113, universal macOS binary.
<!-- SECTION:FINAL_SUMMARY:END -->

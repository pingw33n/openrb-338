---
id: TASK-108
title: 'Linux, Windows and macOS builds: local scripts and GitHub Actions'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 09:34'
updated_date: '2026-10-28 13:45'
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
- [ ] #1 AGENTS.md and docs/building.md describe the scripts and workflows; cargo test and clippy pass
- [ ] #2 scripts/ builds release packages of rb and rb-ui for Linux x86_64, Windows x86_64 and macOS (aarch64 and x86_64, or universal) from this Mac; tool requirements and limits are documented; the produced packages are listed in the notes
- [ ] #3 release.yml builds rb and rb-ui packages for all six targets (Linux, Windows, macOS x amd64, arm64); the local scripts build every pair practical from this Mac and document the skipped ones; tool requirements are documented; the produced packages are listed in the notes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Scope widened by the user (2026-10-28): macOS is a first-class target. scripts/ also builds macOS packages for aarch64-apple-darwin and x86_64-apple-darwin (both from this Mac; a universal binary via lipo if cheap, else two packages), as .tar.gz of rb and rb-ui; release.yml builds them on macos runners too. The .dmg with an .app bundle and the Windows installer .exe are separate tasks (task-110, task-111) that build on this task's scripts and workflows.

Targets decided by the user (2026-10-28): every OS has an amd64 and an arm64 variant: x86_64/aarch64-unknown-linux-gnu, x86_64/aarch64-pc-windows-msvc (or -gnu/-gnullvm for arm64 if msvc is impractical; say which), x86_64/aarch64-apple-darwin. GitHub Actions builds all six, on native runners where they exist (ubuntu-24.04-arm, windows-11-arm, macos-14 for arm64; ubuntu-latest, windows-latest, macos-13 for amd64) or by cross-compiling. The local scripts may skip pairs that are hard to build from this Mac; list which are skipped and why. No code signing anywhere (the user, 2026-10-28).

From task-120: formatting is enforced by nightly rustfmt (rustfmt.toml has unstable options). ci.yml must add a fmt job: rustup toolchain install nightly-2026-10-18 --profile minimal --component rustfmt, then cargo +nightly-2026-10-18 fmt --all --check. Build, clippy and tests stay on the pinned stable toolchain. Also name the pinned nightly in docs/building.md when this task writes it (AGENTS.md already names it).
<!-- SECTION:NOTES:END -->

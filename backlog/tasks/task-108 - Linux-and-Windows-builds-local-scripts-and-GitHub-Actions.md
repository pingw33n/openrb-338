---
id: TASK-108
title: 'Linux and Windows builds: local scripts and GitHub Actions'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 09:34'
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
- [ ] #1 scripts/ builds release packages of rb and rb-ui for Linux x86_64 and Windows x86_64 from this Mac (and the macOS host build); tool requirements and limits are documented; the produced packages are listed in the notes
- [ ] #2 .github/workflows/ci.yml runs fmt, clippy and tests on Linux, Windows and macOS, and release.yml builds and uploads the packages; both are validated (actionlint or act) and need no secrets or non-committed files
- [ ] #3 AGENTS.md and docs/building.md describe the scripts and workflows; cargo test and clippy pass
<!-- AC:END -->

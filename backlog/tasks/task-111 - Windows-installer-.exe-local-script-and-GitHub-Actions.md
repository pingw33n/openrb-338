---
id: TASK-111
title: 'Windows: installer .exe (local script and GitHub Actions)'
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-10-28 10:12'
labels:
  - tools
  - ui
milestone: m-4
dependencies:
  - TASK-108
references:
  - crates/rb-ui/src/art.rs
priority: medium
type: feature
ordinal: 131000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: the user asked (2026-10-28) for a Windows installer .exe, built locally and by GitHub Actions.

What:
- An installer made with NSIS. makensis runs on macOS (brew install makensis) and on windows-latest, so the same script builds it here and in CI. Inno Setup is the alternative but needs Windows or Wine. If you choose otherwise, say why.
- It installs into Program Files\OpenRB-338 (per-user install to LocalAppData if no admin rights, if cheap):
  - the UI (OpenRB-338.exe, the GUI subsystem build: no console window; check that #![windows_subsystem = "windows"] is set for release);
  - the CLI (rb.exe, later orb.exe per task-068);
  - the license and readme.
- A Start menu shortcut and an optional desktop shortcut; the app icon embedded in the exe (an .ico generated from the app_icon part with resvg; a resource via winres or embed-resource in rb-ui's build script).
- An optional checkbox that adds the install folder to the user's PATH for the CLI.
- An uninstaller, registered in Apps & features.
- Name it OpenRB-338-<version>-setup.exe.
- release.yml builds it on windows-latest (or on Linux with makensis) from task-108's Windows build and uploads it.
- Signing: Authenticode needs a certificate. Leave an env-gated step that is skipped without secrets, and document what SmartScreen shows for an unsigned installer.
- Check: build it here; if a Windows machine is available (the test machine, per the agent brief; install only into C:\re\rb\<folder>, never C:\ReBirth RB-338, and uninstall afterwards), run the installer silently (/S /D=...), start the app, run rb --help, and uninstall. Record the results in the notes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A local script produces OpenRB-338-<version>-setup.exe (NSIS) that installs the UI (no console window, icon) and the CLI with Start menu entry, optional PATH and an uninstaller
- [ ] #2 release.yml builds and uploads the installer; a signing step exists but is skipped without secrets; SmartScreen behavior is documented
- [ ] #3 The installer was run (silent install, app start, rb --help, uninstall) on the test machine or the reason it was not is recorded; docs/building.md describes it
<!-- AC:END -->

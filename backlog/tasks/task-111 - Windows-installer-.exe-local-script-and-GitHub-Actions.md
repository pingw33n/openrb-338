---
id: TASK-111
title: 'Windows: installer .exe (local script and GitHub Actions)'
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-10-28 10:12'
updated_date: '2026-10-28 21:35'
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
- [x] #1 A local script produces OpenRB-338-<version>-setup.exe (NSIS) that installs the UI (no console window, icon) and the CLI with Start menu entry, optional PATH and an uninstaller
- [x] #2 The installer was run (silent install, app start, rb --help, uninstall) on the test machine or the reason it was not is recorded; docs/building.md describes it
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. GUI subsystem: windows_subsystem=windows for the UI bin only (cfg_attr on not(debug_assertions): packages use the dist profile); attach to the parent console at start (AttachConsole) so stderr and --screenshot messages still show when run from a terminal; document what is lost.
2. Icon: ui-art ico subcommand (PNG-entry .ico, 16..256) from app_icon.svg via the iconset machinery; commit assets/app_icon.ico; embed it plus VERSIONINFO as an RC resource in rb-ui build.rs, Windows targets only (cross-builds with llvm-rc).
3. NSIS script scripts/installer/openrb.nsi plus scripts/package-installer.sh TARGET (x86_64/aarch64): license page (LICENSE), required app section, Start menu shortcut, optional desktop shortcut and PATH (off), uninstaller and Add/Remove Programs entry with DisplayVersion; per-user vs per-machine decided in notes.
4. release.yml installer job after package (downloads bin-<target> artifacts by name, runs makensis on windows-latest), uploads artifacts by name.
5. Validate: makensis, 7zz l, llvm-readobj subsystem and resources, file; real install on the test machine under C:\re\rb if feasible.
6. docs/building.md (NSIS install, SmartScreen, console loss), notes, ACs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
User decisions (2026-10-28): no Authenticode signing: drop the signing step, document SmartScreen. Two installers, one per architecture: OpenRB-338-<version>-x64-setup.exe and -arm64-setup.exe, from task-108's x86_64 and aarch64 Windows builds. If the arm64 one cannot be built from this Mac, build it only in GitHub Actions (windows-11-arm or cross) and say so.

From task-110 (user decision 2026-10-28): the version of every package is the UTC build time, never the Cargo version (0.1.0). File names use scripts/version.sh = YYYY_MM_DD_HH_MM_SS from SOURCE_DATE_EPOCH, e.g. OpenRB-338-2026_10_28_19_30_00-x64-setup.exe or ...-x86_64.AppImage (the release-tag form of task-113); human text uses '2026-10-28 19:30:00 UTC'; where a format wants numbers (Windows file version, macOS plist) use YYYY.MMDD.HHMMSS style numeric parts. scripts/version.sh is the single source. The built-in kit may ship in public packages. Scheme and examples: docs/building.md, 'The version is the build time'. The macOS bundle id is net.emphased.openrb338 (the Windows and Linux ids should follow that domain).

Branch: worktree-agent-a2f8a1a369338785d (rebased on main 1ce103c). Superseded: version is YYYY.MMDD.HHMM (task-133); installer names OpenRB-338-<version>-<arch>-setup.exe.

Evidence (2026-10-28):
- Local build: SOURCE_DATE_EPOCH=1793215800 scripts/package-installer.sh x86_64-pc-windows-msvc | aarch64-pc-windows-msvc (cargo-xwin, dist profile, makensis 3.13 -V2 -WX). x64-setup 8,381,129 bytes, arm64-setup 7,478,505 bytes. Two runs give byte-identical installers (cmp, both archs; the first attempt differed because NSIS stores file mtimes: the script now stages the files with the build time).
- readobj: OpenRB-338.exe IMAGE_SUBSYSTEM_WINDOWS_GUI, rb.exe WINDOWS_CUI (x64 and arm64); coff-resources: 8 RT_ICON, GROUP_ICON, VERSIONINFO 2026.1028.1930.0. 7zz l lists the 5 files plus the uninstaller.
- Test machine (all 6 permits held, then released; C:/re/rb/inst111, deleted; the ReBirth install untouched): setup.exe /S /PATH=1 /DESKTOP=1 /D=C:\re\rb\inst111\app exit 0; ARP key DisplayVersion 2026.1028.1930; both shortcuts; PATH stays REG_EXPAND_SZ; rb.exe --help OK; OpenRB-338.exe runs (title 'OpenRB-338 - Untitled', a screenshot shows the panel), no child conhost; --bogus with stderr redirected prints the usage text, exit 2; Uninstall.exe /S _?=dir removes the keys, shortcuts and the PATH entry. Failed hypothesis: the first path.ps1 dropped empty PATH entries (the user's PATH ended with a semicolon); rewritten to keep the entries, cycle repeated OK. Afterwards I restored the user's HKCU PATH to its original (trailing semicolon, 50 chars).
- Not verified: the arm64 installer was not run (the machine is x64); AttachConsole's visible output (the console cannot be read back; the exe exits 2 without a crash); the wizard pages (only /S); the release.yml installer job (actionlint ok; needs CI; Ubuntu's apt NSIS may be older than 3.13). cargo xwin clippy --release -p rb-ui --target x86_64-pc-windows-msvc -D warnings is clean (host clippy cannot see the Windows cfg code); the clippy fix came after the machine test and was not rerun there.
- Decisions: per-user install (reason in docs/building.md), NSIS, PATH through PowerShell (NSIS strings are 1024 chars here), icon committed as assets/app_icon.ico (byte comparison across CPUs is not safe, the test checks the sizes). Signing: none (user decision), SmartScreen documented.

Review round 1 (all 12 points addressed, one amended commit):
1. build.rs no longer canonicalizes the icon path (join from CARGO_MANIFEST_DIR, forward slashes in the .rc). The native Windows build with rc.exe (ci.yml windows-latest, release package jobs) was NOT run: unverified until a CI Windows job runs; the cross build with llvm-rc was rebuilt and checked.
2. AC #3 (the release.yml installer job in CI, SmartScreen text) removed from this task: the CI run of the installer job moves to a follow-up task (the supervisor creates one for the first release run's checks: dmg, installer, Linux Docker). The job and the SmartScreen text are in the commit.
3. Commit subject shortened.
4. openrb.nsi: EnsureClosed opens OpenRB-338.exe and rb.exe for writing (a sharing violation means running) in the installer and the uninstaller; Retry/Cancel box, /S or Cancel: exit code 5, nothing changed. The uninstaller keeps the registry keys and exits 6 when a file remains. Tested on the test machine (all permits, C:/re/rb/inst111, deleted): install over a running app exit 5, uninstall of a running app exit 5 (files and Apps entry kept), in-place upgrade exit 0. Exit 6 not exercised. Documented in docs/building.md.
5. windows-latest has no NSIS (runner-images Windows2025 list: Inno Setup 6.7.1 and WiX, no NSIS), so the job stays on ubuntu; MAKENSIS_FLAGS is empty there (no -WX); the .nsi uses only 3.0x features by reading (Unicode true, MUI2, FileFunc, LogicLib ForEach, nsExec, SendMessage /TIMEOUT). Not run on 3.08.
6. Full install cycle rerun with the final x64 installer (sha256 374d8097160dc88b3e1f564c3ee4455341e9ef8db13d18e0da22340b5f4b5267): see docs/building.md; AC #2 evidence is of the final build. arm64 still not run.
7. setup.exe and Uninstall.exe use assets/app_icon.ico (MUI_ICON/MUI_UNICON); makensis accepted the PNG-entry .ico, the installer has GROUP_ICON.
8. path.ps1 keeps every entry that is not ours (empty ones too), removes our entry and the ';' after it (a last entry: only the entry), disposes the key in finally. Install then uninstall left HKCU Path byte-identical (tested, incl. a user PATH with a trailing ';').
9. SetShellVarContext current in .onInit and un.onInit.
10. A reinstall into another folder removes the PATH entry of the folder recorded in HKCU (tested: orphan entry gone, Apps entry moves; the old folder's files stay, documented).
11. docs/building.md lines of this task wrapped to 100 (the remaining long lines are older ones and a toml/command block).
12. assets/app_icon.ico vs a fresh ui-art ico on this host (aarch64 macOS): cmp identical, sha256 67f66747f099ae8c3592cc79f3c28961f6faf4eb2ff56ada0a8a6bac08c366e7.
Final: x64-setup 8,395,465 bytes sha256 374d8097160dc88b3e1f564c3ee4455341e9ef8db13d18e0da22340b5f4b5267; arm64-setup 7,498,263 bytes sha256 7c819fb65e89b75f52cb8a4bb8e2c3f64f27475fb6f6a957db1c6ce820058cc1. fmt, clippy, cargo xwin clippy (Windows), cargo test --release 586 passed 0 failed, shellcheck, actionlint all clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Outcome: NSIS installers for Windows x64 and arm64, built with scripts/package-installer.sh TARGET (cargo-xwin cross-built dist binaries) and by a new installer job in release.yml (ubuntu-latest, apt nsis, MAKENSIS_FLAGS empty; downloads bin-<target>, uploads installer-x64 / installer-arm64). OpenRB-338-2026.1028.1930-x64-setup.exe is 8,395,465 bytes and ...-arm64-setup.exe 7,498,263 bytes; two runs are byte-identical.
Changes: OpenRB-338.exe is a GUI-subsystem exe in release/dist builds (rb.exe stays a console program) with AttachConsole for terminal stderr; assets/app_icon.ico (ui-art ico) and a VERSIONINFO are embedded by rb-ui's build.rs (embed-resource, Windows targets only); the installer is per-user, with Start menu and optional desktop shortcut, optional rb on the user's PATH (path.ps1), license page, installer and uninstaller icons, running-app check (exit 5), uninstall that keeps its keys when files remain (exit 6), orphan PATH removal on reinstall elsewhere, Apps entry with DisplayVersion; silent switches /S /STARTMENU= /DESKTOP= /PATH= /D=. docs/building.md has the section The Windows installer, SmartScreen text and tool install.
Checked: makensis -WX, 7zz l, llvm-readobj, actionlint, shellcheck, nightly fmt, clippy and cargo xwin clippy for Windows, cargo test --release 586 passed; on the Windows test machine (x64) the full silent install/run/lock/upgrade/orphan/uninstall cycle with the final installer, PATH byte-identical afterwards. Not checked: arm64 installer run, the native-Windows build with rc.exe, the release.yml installer job on Ubuntu's older NSIS, AttachConsole visible output, wizard pages, exit 6. AC #3 (the CI job) was moved to a follow-up task by the supervisor. Branch worktree-agent-a2f8a1a369338785d, one commit.
<!-- SECTION:FINAL_SUMMARY:END -->

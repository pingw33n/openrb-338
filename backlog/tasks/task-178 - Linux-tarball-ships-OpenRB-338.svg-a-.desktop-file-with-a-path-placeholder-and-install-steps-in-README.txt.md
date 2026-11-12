---
id: TASK-178
title: >-
  Linux tarball ships OpenRB-338.svg, a .desktop file with a path placeholder,
  and install steps in README.txt
status: Done
assignee:
  - '@agent-worker'
created_date: '2026-11-12T13:01:00Z'
updated_date: '2026-11-12T13:15:00Z'
labels:
  - tools
  - docs
milestone: m-5
dependencies: []
references:
  - scripts/package.sh
  - scripts/package-appimage.sh
  - assets/ui/app_icon.svg
  - https://github.com/pingw33n/OpenRB-338/issues/7
priority: medium
type: enhancement
ordinal: 194000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Why: GitHub issue #7 (eugenialoli, Linux Mint): "for Linux please provide a .desktop file, and the icon in SVG/PNG. The .ico doesn't work on Linux, and it does require a .desktop file in order to get a menu entry." The reporter uses the plain tarball (OpenRB-338-<version>-x86_64-unknown-linux-gnu.tar.gz), which today holds only the two binaries, LICENSE and README.txt (scripts/package.sh:27-50): no icon file and no desktop entry, so the app cannot get a menu entry. The AppImage already has both inside (scripts/package-appimage.sh:92-123).

The user's decision (2026-11-12): the tarball gets the icon, a ready .desktop file whose paths are a placeholder, and install steps in its README.txt; no install script and no --register flag for now.

Why a ready file rather than letting users write one: the window sets the app id OpenRB-338 (orb-ui main.rs with_app_id). Wayland shells match a window to the desktop file named <app id>.desktop, X11 shells to StartupWMClass. A hand-written file with another name or without StartupWMClass launches the app but the running window gets a generic icon and a second dock entry. .desktop files do not support relative paths, so Exec= and Icon= must be absolute, hence the placeholder.

What (Linux targets of scripts/package.sh only; macOS and Windows packages unchanged):
- OpenRB-338.svg in the package root: assets/ui/app_icon.svg as committed (the source the AppImage's PNGs are rendered from).
- OpenRB-338.desktop in the package root: the same entry as the AppImage's (Type, Name, GenericName, Comment, Categories, Terminal=false, StartupWMClass=OpenRB-338, %f in Exec), with Exec="@DIR@/OpenRB-338" %f and Icon=@DIR@/OpenRB-338.svg, and without X-AppImage-Version. Keep one source for the entry's common lines so the AppImage's and the tarball's cannot drift (e.g. a template under scripts/ or assets/ both scripts fill in); the AppImage's output must stay byte-identical apart from what the template forces, check by diffing a built AppDir's .desktop before and after.
- README.txt (written by package.sh) gains, for Linux only, a short section: the menu entry install, run in the unpacked folder:
    sed "s|@DIR@|$PWD|g" OpenRB-338.desktop > ~/.local/share/applications/OpenRB-338.desktop
  that the file name must stay OpenRB-338.desktop (the window is matched to it), that moving the folder means running it again, and the removal: rm ~/.local/share/applications/OpenRB-338.desktop. Check whether the menu picks the entry up without update-desktop-database on a current desktop; mention the command only if it is needed. Quote paths so a folder with spaces works (Exec quoting rules of the Desktop Entry spec: test a path with a space).
- docs/building.md (the package contents section) and README.md's Linux install notes: one line each.
- Verification: desktop-file-validate passes on the file after the sed (with a path containing a space). If a Linux desktop is not available, at least run the sed and desktop-file-validate in the ubuntu:22.04 container the AppImage smoke test uses, and say in the notes that the menu entry itself was not seen.
- A reply text for issue #7 (this part) in the task notes; post nothing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Linux tarballs contain OpenRB-338.svg and OpenRB-338.desktop (Exec and Icon with @DIR@, StartupWMClass=OpenRB-338, file name OpenRB-338.desktop); macOS and Windows packages are unchanged; the entry's common lines have one source shared with the AppImage, whose .desktop is unchanged
- [x] #2 README.txt in the Linux tarballs gives the one-line install from the unpacked folder, the removal, and the rename and move caveats; after that sed (path with a space) desktop-file-validate passes; results in the notes
- [x] #3 docs/building.md and README.md mention the files; scripts/package.sh for a Linux target runs in CI unchanged otherwise; the task notes hold a reply text for issue #7
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. scripts/OpenRB-338.desktop.in: the entry's common lines with @EXEC@ and @ICON@ placeholders; package-appimage.sh fills them (Exec=OpenRB-338 %f, Icon=OpenRB-338) and appends X-AppImage-Version, byte-identical to today's file (diffed).
2. package.sh, Linux targets: copy assets/ui/app_icon.svg as OpenRB-338.svg, fill the template with the quoted @DIR@ Exec and Icon, add the README.txt install section.
3. docs/building.md and README.md one line each.
4. Verify without Docker (not running): sed locally, diff the AppImage entry, bash -n, run package.sh with SKIP_BUILD and a fake bin for Linux, macOS and Windows targets; desktop-file-validate only if available.
5. Notes with the reply text for issue 7.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Branch: task-178-linux-desktop-entry

Implementation:
- scripts/OpenRB-338.desktop.in holds the common lines (@EXEC@, @ICON@). package-appimage.sh fills Exec=OpenRB-338, Icon=OpenRB-338 and appends X-AppImage-Version; cmp of the generated text against the previous heredoc output: identical.
- package.sh (Linux targets) adds OpenRB-338.svg (cmp equal to assets/ui/app_icon.svg), OpenRB-338.desktop (Exec="@DIR@/OpenRB-338" %f, Icon=@DIR@/OpenRB-338.svg, StartupWMClass=OpenRB-338) and a README.txt menu section. With SKIP_BUILD=1 and fake bins, the aarch64-apple-darwin tar.gz and x86_64-pc-windows-msvc zip have no new files and an unchanged README.
- docs/building.md and README.md mention the files. User addition (2026-11-12): README.md's Linux notes also say the AppImage holds its own icon and menu entry, and an AppImage tool such as Gear Lever or AppImageLauncher adds the menu item.

Verification (Docker, ubuntu:22.04 aarch64, desktop-file-utils 0.26-1ubuntu3; the unfilled file fails validation as expected: Icon @DIR@ looks relative):
- Review round 1 (on 64f8cb6): the README.txt command now writes aside and moves into place, and names the characters a path must not contain (| & \ " $ ` %; spaces are fine):
  d=~/.local/share/applications; mkdir -p $d && sed "s|@DIR@|$PWD|g" OpenRB-338.desktop > $d/OpenRB-338.desktop.new && mv $d/OpenRB-338.desktop.new $d/OpenRB-338.desktop
- That line, taken from the README.txt of the real fake-bin tarball, run in '/tmp/dir with space/OpenRB 338': exit 0, Exec="/tmp/dir with space/OpenRB 338/OpenRB-338" %f, desktop-file-validate: VALIDATE-OK.
- Same line in '/tmp/pipe|dir' with an existing entry: sed fails ("unknown option to s"), exit 1, cmp shows the existing OpenRB-338.desktop untouched; a stray OpenRB-338.desktop.new is left (harmless: the menu ignores it).
- Not done: a real Docker build of the package, seeing the menu entry on a desktop.
- Open item: whether a menu needs update-desktop-database for a new entry was not checked. Expected not (it rebuilds only the MIME cache and the entry has no MimeType). README.txt does not mention it.

Reply text for issue #7 (post nothing):
Thanks for the report. The Linux .tar.gz now has OpenRB-338.svg (the icon) and OpenRB-338.desktop. A desktop file needs absolute paths, so the file holds @DIR@ where the folder goes. The package's README.txt has a one-line command to run in the unpacked folder that fills in the folder and installs the file as ~/.local/share/applications/OpenRB-338.desktop. Keep that file name: the desktop matches the window's icon to it. If you move the folder, run the line again; remove the entry with rm ~/.local/share/applications/OpenRB-338.desktop. The AppImage already has its own entry and icon.

Review round 2 (on e4233b1): the install line had a shell variable assignment, which fish rejects. It is now one line without variables (mkdir -p ~/.local/share/applications && sed "s|@DIR@|$PWD|g" OpenRB-338.desktop > ~/.local/share/applications/OpenRB-338.desktop.new && mv ...new ...desktop), and README.txt says it works in bash, zsh and fish and that a failed run leaves a deletable OpenRB-338.desktop.new. Re-run in ubuntu:22.04 (fish 3.3.1, desktop-file-utils 0.26), the line taken from the tarball's README.txt, path '/tmp/dir with space/OpenRB 338': under sh and under fish -c exit 0, Exec="/tmp/dir with space/OpenRB 338/OpenRB-338" %f, desktop-file-validate VALIDATE-OK both times; path with | and an existing entry: sed fails, exit 1, entry untouched. The earlier notes' mention of the old d=... line is superseded.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The Linux tarballs now ship OpenRB-338.svg and OpenRB-338.desktop (@DIR@ placeholder, StartupWMClass=OpenRB-338) and README.txt install, removal and move steps. The install line writes to a temporary name and moves it into place, and the README names the characters a folder path must not contain. The entry's common lines live in scripts/OpenRB-338.desktop.in, shared with package-appimage.sh, whose output is byte-identical to before. docs/building.md and README.md updated (README.md also points AppImage users to Gear Lever or AppImageLauncher). Branch task-178-linux-desktop-entry, one commit. Tested: package.sh with fake binaries for Linux, macOS and Windows targets; the README line on a path with a space plus desktop-file-validate in ubuntu:22.04 pass; a path with | leaves an existing entry untouched. Not done: a real package build, seeing the menu entry. Open item: whether the menu needs update-desktop-database was not checked (not mentioned in README.txt).
<!-- SECTION:FINAL_SUMMARY:END -->

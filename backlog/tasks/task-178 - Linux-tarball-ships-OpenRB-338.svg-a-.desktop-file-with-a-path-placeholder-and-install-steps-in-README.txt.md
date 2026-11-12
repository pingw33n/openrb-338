---
id: TASK-178
title: >-
  Linux tarball ships OpenRB-338.svg, a .desktop file with a path placeholder,
  and install steps in README.txt
status: To Do
assignee:
  - '@agent-worker'
created_date: '2026-11-12T13:01:00Z'
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
- [ ] #1 The Linux tarballs contain OpenRB-338.svg and OpenRB-338.desktop (Exec and Icon with @DIR@, StartupWMClass=OpenRB-338, file name OpenRB-338.desktop); macOS and Windows packages are unchanged; the entry's common lines have one source shared with the AppImage, whose .desktop is unchanged
- [ ] #2 README.txt in the Linux tarballs gives the one-line install from the unpacked folder, the removal, and the rename and move caveats; after that sed (path with a space) desktop-file-validate passes; results in the notes
- [ ] #3 docs/building.md and README.md mention the files; scripts/package.sh for a Linux target runs in CI unchanged otherwise; the task notes hold a reply text for issue #7
<!-- AC:END -->

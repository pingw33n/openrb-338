#!/usr/bin/env bash
# Runs appimagetool on an AppDir inside the build image (task-112; called by
# scripts/package-appimage.sh when it is not on a Linux host of the target's architecture).
#
#   ARCH=x86_64 scripts/appimage-pack.sh APPIMAGETOOL APPDIR OUT
#
# The tool is an AppImage and runs with --appimage-extract-and-run; if it does not run, the
# script fails. Docker Desktop on a Mac runs amd64 containers emulated, and there the kernel
# refuses to exec the x86_64 appimagetool itself ("Exec format error": its ELF header carries
# the AppImage marker, ABI version 65), while the static programs inside it run. Only when
# OPENRB_APPIMAGE_EMULATED is 1 (package-appimage.sh sets it for x86_64 on a macOS host, and
# nothing in CI does) is the tool's squashfs (which starts after the section header table of
# its ELF runtime) unpacked with unsquashfs and the appimagetool inside run directly.
set -euo pipefail

tool=${1:?usage: appimage-pack.sh APPIMAGETOOL APPDIR OUT}
appdir=${2:?}
out=${3:?}

if [ "${OPENRB_APPIMAGE_EMULATED:-}" != 1 ]; then
    # the normal path: a runtime that does not run fails the script (and the CI job)
    "$tool" --appimage-extract-and-run "$appdir" "$out"
else
    echo "OPENRB_APPIMAGE_EMULATED is set: unpacking appimagetool with unsquashfs" >&2
    offset=$(readelf -h "$tool" | awk '
        /Start of section headers/ { off = $5 }
        /Size of section headers/ { size = $5 }
        /Number of section headers/ { num = $5 }
        END { print off + size * num }')
    work=$(mktemp -d)
    trap 'rm -rf "$work"' EXIT
    unsquashfs -q -o "$offset" -d "$work/at" "$tool"
    # its helpers (mksquashfs, desktop-file-validate, zsyncmake) are in the same folder
    PATH=$work/at/usr/bin:$PATH appimagetool "$appdir" "$out"
fi
chmod +x "$out"

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
# ORB_APPIMAGE_EMULATED is 1 (package-appimage.sh sets it for x86_64 on a macOS host, and
# nothing in CI does) is the tool's squashfs (which starts after the section header table of
# its ELF runtime) unpacked with unsquashfs and the appimagetool inside run directly.
#
# Before it packs, the script copies the X11 keyboard libraries into APPDIR/usr/lib (task-166):
# winit loads libxkbcommon-x11 with dlopen and panics when the host lacks it (a bare Ubuntu with
# Xvfb has no libxkbcommon-x11-0). They come from this Linux system (the Ubuntu 22.04 build
# image, or the 22.04 runner, where release.yml installs them), so each has the glibc 2.35
# floor of the programs. patchelf sets the search path (no LD_LIBRARY_PATH, which would leak
# into children such as zenity): RUNPATH $ORIGIN/../lib on the two programs of the AppDir,
# $ORIGIN on the libraries (a RUNPATH is not inherited). No other library is bundled. Needs
# patchelf.
set -euo pipefail

tool=${1:?usage: appimage-pack.sh APPIMAGETOOL APPDIR OUT}
appdir=${2:?}
out=${3:?}

bundled_libs="libxkbcommon.so.0 libxkbcommon-x11.so.0 libxcb-xkb.so.1"
mkdir -p "$appdir/usr/lib"
for soname in $bundled_libs; do
    case ${ARCH:?ARCH is not set} in
        x86_64) tag='libc6,x86-64' ;;
        aarch64) tag='libc6,AArch64' ;;
        *) echo "unknown ARCH $ARCH" >&2; exit 2 ;;
    esac
    path=$(ldconfig -p | awk -v n="$soname" -v t="($tag)" \
        '$1 == n && index($0, t) { print $NF; exit }')
    [ -n "$path" ] && [ -f "$path" ] || {
        echo "$soname ($ARCH) not found on this system (apt-get install libxkbcommon-x11-0)" >&2
        exit 1
    }
    cp -L "$path" "$appdir/usr/lib/$soname"
    chmod 644 "$appdir/usr/lib/$soname"
    patchelf --set-rpath "\$ORIGIN" "$appdir/usr/lib/$soname"
done
for prog in OpenRB-338 orb; do
    patchelf --set-rpath "\$ORIGIN/../lib" "$appdir/usr/bin/$prog"
done
# the squashfs stores the file times (package-appimage.sh stamped the rest of the AppDir)
if [ -n "${SOURCE_DATE_EPOCH:-}" ]; then
    find "$appdir" -exec touch -h -d "@$SOURCE_DATE_EPOCH" {} +
fi

if [ "${ORB_APPIMAGE_EMULATED:-}" != 1 ]; then
    # the normal path: a runtime that does not run fails the script (and the CI job)
    "$tool" --appimage-extract-and-run "$appdir" "$out"
else
    echo "ORB_APPIMAGE_EMULATED is set: unpacking appimagetool with unsquashfs" >&2
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

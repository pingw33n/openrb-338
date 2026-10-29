#!/usr/bin/env bash
# Builds one target (scripts/build.sh) and packs dist/<target>/bin/ into
# dist/<target>/OpenRB-338-<YYYY.MMDD.HHMM>-<target>.tar.gz (Linux, macOS) or .zip (Windows;
# the version is the UTC build time, scripts/version.sh), with
# the license and a short readme (task-108).
#
#   scripts/package.sh TARGET
#
# SKIP_BUILD=1 packs the binaries already in dist/<target>/bin/. The installers and bundles
# of task-110 (dmg), task-111 (Windows installer) and task-112 (AppImage) start from the same
# dist/<target>/bin/ and the same version (`scripts/version.sh`).
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
target=${1:?usage: scripts/package.sh TARGET}
dist=${DIST:-$root/dist}
# one build time for the name, the binaries (About box) and the archive
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(date +%s)}
version=$("$root/scripts/version.sh")
name=OpenRB-338-$version-$target

[ -n "${SKIP_BUILD:-}" ] || "$root/scripts/build.sh" "$target"

stage=$dist/$target/stage/$name
rm -rf "$dist/$target/stage"
mkdir -p "$stage"
cp "$dist/$target/bin/"* "$stage/"
cp "$root/LICENSE" "$stage/LICENSE"

case $target in
    *-windows-*) run="OpenRB-338.exe (the app) and orb.exe (the command line)" ;;
    *) run="OpenRB-338 (the app) and orb (the command line)" ;;
esac
cat > "$stage/README.txt" <<EOF
OpenRB-338 $version ($target)

A reimplementation of the audio engine of Propellerhead ReBirth RB-338 2.01.

This package holds $run.
Neither needs any file of the original program: the standard drum kit is built in.

  orb render SONG.rbs -o SONG.flac    render a song to FLAC, Ogg Vorbis or WAV
  orb --help                          all commands
  OpenRB-338                          the desktop UI

The binaries are not signed (docs/building.md says what the system shows for that).
Licensed under the GNU General Public License, version 3 only (GPL-3.0-only; see LICENSE).
EOF

# owner, group and (with GNU tar) mtime and order are fixed; bsdtar keeps the file mtimes
epoch=${SOURCE_DATE_EPOCH:-$(date +%s)}
case $target in
    *-windows-*)
        archive=$dist/$target/$name.zip
        rm -f "$archive"
        if command -v zip >/dev/null; then
            (cd "$dist/$target/stage" && zip -qr -X "$archive" "$name")
        elif command -v 7z >/dev/null; then
            (cd "$dist/$target/stage" && 7z a -tzip -bso0 "$archive" "$name")
        else
            echo "no zip or 7z on PATH (brew install zip; or install 7-Zip)" >&2
            exit 1
        fi
        ;;
    *)
        archive=$dist/$target/$name.tar.gz
        if tar --version 2>/dev/null | grep -q GNU; then
            (cd "$dist/$target/stage" && tar --sort=name --owner=0 --group=0 --numeric-owner \
                --mtime="@$epoch" -cf - "$name") | gzip -n > "$archive"
        else
            (cd "$dist/$target/stage" && tar --no-xattrs --uid 0 --gid 0 --numeric-owner \
                -cf - "$name") \
                | gzip -n > "$archive"
        fi
        ;;
esac
rm -rf "$dist/$target/stage"
ls -l "$archive"

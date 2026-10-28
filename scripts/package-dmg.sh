#!/usr/bin/env bash
# Builds dist/universal-apple-darwin/OpenRB-338-<YYYY_MM_DD_HH_MM_SS>-universal-apple-darwin.dmg
# (the UTC build time, scripts/version.sh; task-110): a universal (aarch64 +
# x86_64, `lipo`) OpenRB-338.app with the CLI inside, an /Applications link for drag-install,
# the license and a readme.
#
#   scripts/package-dmg.sh
#
# Builds both macOS targets with scripts/build.sh first (on a Mac only); SKIP_BUILD=1 uses the
# binaries already in dist/<target>/bin/ (release.yml downloads them from the two macOS package
# jobs). The CLI sits in the bundle, at OpenRB-338.app/Contents/MacOS/rb: dragging the app to
# Applications installs both, the bundle's ad-hoc seal covers both (rb is signed first), and `rb`
# goes on PATH with one `ln -s` (README.txt). A second file next to the app would be left
# behind by the drag.
# Environment as scripts/build.sh and scripts/macos-app.sh.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
dist=${DIST:-$root/dist}
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(date +%s)}
version=$("$root/scripts/version.sh")
built="${version:0:4}-${version:5:2}-${version:8:2} ${version:11:2}:${version:14:2}:${version:17:2}"
arm=aarch64-apple-darwin
x86=x86_64-apple-darwin
uni=universal-apple-darwin
out=$dist/$uni

if [ -z "${SKIP_BUILD:-}" ]; then
    "$root/scripts/build.sh" "$arm"
    "$root/scripts/build.sh" "$x86"
fi

rm -rf "$out"
mkdir -p "$out/bin"
for b in rb OpenRB-338; do
    lipo -create "$dist/$arm/bin/$b" "$dist/$x86/bin/$b" -output "$out/bin/$b"
    chmod 755 "$out/bin/$b"
    archs=$(lipo -archs "$out/bin/$b" | tr ' ' '\n' | sort | tr '\n' ' ')
    [ "$archs" = "arm64 x86_64 " ] || { echo "$b is not universal: $archs" >&2; exit 1; }
done

stage=$out/stage
mkdir -p "$stage"
"$root/scripts/macos-app.sh" "$stage" "$out/bin/OpenRB-338" "$out/bin/rb"
ln -s /Applications "$stage/Applications"
cp "$root/LICENSE" "$stage/LICENSE"
cat > "$stage/README.txt" <<EOF
OpenRB-338, built $built UTC

A reimplementation of the audio engine of Propellerhead ReBirth RB-338 2.01.

Install: drag OpenRB-338.app onto Applications. It needs no file of the original program (the
standard drum kit is built in) and runs natively on Apple silicon and on Intel Macs.

The app is not signed or notarized, so macOS blocks it the first time ("cannot be opened because
the developer cannot be verified"). Pick one:
  - macOS 15 (Sequoia) and newer: open the app once (it is refused), then System Settings >
    Privacy & Security, scroll to Security, click "Open Anyway" for OpenRB-338 and confirm.
    Control-click > Open no longer bypasses the check there.
  - macOS 14 and older: right-click the app, choose Open, then Open.
  - Any version, in Terminal:
        xattr -dr com.apple.quarantine /Applications/OpenRB-338.app

The command line tool rb is inside the app. To use it from a terminal:
    sudo ln -s /Applications/OpenRB-338.app/Contents/MacOS/rb /usr/local/bin/rb
    rb render SONG.rbs -o SONG.flac     render a song to FLAC, Ogg Vorbis or WAV
    rb --help                           all commands

Licensed under the GNU General Public License, version 3 only (GPL-3.0-only; see LICENSE).
EOF

dmg=$out/OpenRB-338-$version-$uni.dmg
rm -f "$dmg"
# the same fixed times as the bundle (macos-app.sh)
stamp=$(TZ=UTC date -r "${SOURCE_DATE_EPOCH:-$(date +%s)}" +%Y%m%d%H%M.%S)
find "$stage" -exec touch -h -t "$stamp" {} +
# hdiutil sometimes reports "Resource busy" right after the files were written; retry.
for attempt in 1 2 3; do
    hdiutil create -quiet -volname "OpenRB-338" -srcfolder "$stage" -fs HFS+ \
        -format UDZO -ov "$dmg" && break
    [ "$attempt" -lt 3 ] || { echo "hdiutil create failed" >&2; exit 1; }
    sleep 5
done
rm -rf "$stage"
ls -l "$dmg"

#!/usr/bin/env bash
# Builds dist/<target>/OpenRB-338-<YYYY.MMDD.HHMM>-<target>.dmg for one macOS target (the
# version is the UTC build time, scripts/version.sh; task-110, task-136): a single-architecture
# OpenRB-338.app with the CLI inside, an /Applications link for drag-install, the license and a
# readme. One image per architecture, Apple silicon and Intel; there is no universal binary.
#
#   scripts/package-dmg.sh aarch64-apple-darwin|x86_64-apple-darwin
#
# Builds the target with scripts/build.sh first (on a Mac only); SKIP_BUILD=1 uses the binaries
# already in dist/<target>/bin/ (release.yml downloads them from the macOS package job). The
# CLI sits in the bundle, at OpenRB-338.app/Contents/MacOS/orb: dragging the app to
# Applications installs both, the bundle's ad-hoc seal covers both (orb is signed first), and `orb`
# goes on PATH with one `ln -s` (README.txt). A second file next to the app would be left
# behind by the drag.
# Environment as scripts/build.sh and scripts/macos-app.sh.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
dist=${DIST:-$root/dist}
target=${1:?usage: scripts/package-dmg.sh aarch64-apple-darwin|x86_64-apple-darwin}
case $target in
    aarch64-apple-darwin) arch=arm64 desc="Macs with Apple silicon (M1 and newer)" ;;
    x86_64-apple-darwin) arch=x86_64 desc="Intel Macs (get the Apple silicon image for an M1 or newer)" ;;
    *) echo "not a macOS target: $target" >&2; exit 1 ;;
esac
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(date +%s)}
version=$("$root/scripts/version.sh")
out=$dist/$target

if [ -z "${SKIP_BUILD:-}" ]; then
    "$root/scripts/build.sh" "$target"
fi

for b in orb OpenRB-338; do
    [ -f "$out/bin/$b" ] || { echo "missing $out/bin/$b" >&2; exit 1; }
    archs=$(lipo -archs "$out/bin/$b")
    [ "$archs" = "$arch" ] || { echo "$b is not $arch only: $archs" >&2; exit 1; }
done

stage=$out/stage
rm -rf "$stage"
mkdir -p "$stage"
"$root/scripts/macos-app.sh" "$stage" "$out/bin/OpenRB-338" "$out/bin/orb"
ln -s /Applications "$stage/Applications"
cp "$root/LICENSE" "$stage/LICENSE"
cat > "$stage/README.txt" <<EOF
OpenRB-338 $version

A reimplementation of the audio engine of Propellerhead ReBirth RB-338 2.01.

Install: drag OpenRB-338.app onto Applications. It needs no file of the original program (the
standard drum kit is built in). This image is for $desc.

The app is not signed or notarized, so macOS blocks it the first time ("cannot be opened because
the developer cannot be verified"). Pick one:
  - macOS 15 (Sequoia) and newer: open the app once (it is refused), then System Settings >
    Privacy & Security, scroll to Security, click "Open Anyway" for OpenRB-338 and confirm.
    Control-click > Open no longer bypasses the check there.
  - macOS 14 and older: right-click the app, choose Open, then Open.
  - Any version, in Terminal:
        xattr -dr com.apple.quarantine /Applications/OpenRB-338.app

The command line tool orb is inside the app. To use it from a terminal:
    sudo ln -s /Applications/OpenRB-338.app/Contents/MacOS/orb /usr/local/bin/orb
    orb render SONG.rbs -o SONG.flac    render a song to FLAC, Ogg Vorbis or WAV
    orb --help                          all commands

Licensed under the GNU General Public License, version 3 only (GPL-3.0-only; see LICENSE).
EOF

dmg=$out/OpenRB-338-$version-$target.dmg
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

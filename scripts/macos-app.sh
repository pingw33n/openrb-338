#!/usr/bin/env bash
# Makes OpenRB-338.app from the UI binary (and optionally the CLI binary) (task-110).
#
#   scripts/macos-app.sh OUT_DIR UI_BINARY [CLI_BINARY]
#
# OUT_DIR/OpenRB-338.app gets:
#   Contents/MacOS/OpenRB-338      the UI (CFBundleExecutable)
#   Contents/MacOS/orb             the CLI, if given
#   Contents/Resources/OpenRB-338.icns   the app_icon part, rendered by `ui-art iconset`
#   Contents/Info.plist
# and is signed ad hoc (`codesign -s -`; orb first, then the bundle, whose seal covers both):
# Apple silicon refuses an unsigned bundle, and there is no Developer ID signing or
# notarization (docs/building.md). The bundle is what gives the Dock
# and Activity Monitor the name and the icon: an unbundled binary has no CFBundleIconFile.
#
# The version is the build time (SOURCE_DATE_EPOCH, default now; the number the About box
# shows): CFBundleShortVersionString and CFBundleVersion are YYYY.MMDD.HHMM (e.g. 2026.1028.1930;
# at most three integers; scripts/version.sh), and OpenRBBuildTime is the full
# `2026-10-28 19:30:00 UTC`. Both bundle keys are the time and not a commit count: it orders
# builds across checkouts and branches, and the file name of the .dmg carries the same version.
# The binary got its own SOURCE_DATE_EPOCH when it was built: a bundle made with another one
# (scripts/macos-dev-app.sh after a plain cargo build) can differ from its About box.
#
# Environment: SOURCE_DATE_EPOCH, ICNS (an existing .icns: skips rendering one), BUNDLE_ID
# (default net.emphased.openrb338).
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:?usage: scripts/macos-app.sh OUT_DIR UI_BINARY [CLI_BINARY]}
ui=${2:?usage: scripts/macos-app.sh OUT_DIR UI_BINARY [CLI_BINARY]}
cli=${3:-}
export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(date +%s)}
version=$("$root/scripts/version.sh")   # 2026.1028.1930
build=$version
built="$("$root/scripts/version.sh" --time) UTC"   # 2026-10-28 19:30:00 UTC
bundle_id=${BUNDLE_ID:-net.emphased.openrb338}

[ "$(uname -s)" = Darwin ] || { echo "macOS only (iconutil, codesign)" >&2; exit 1; }

app=$out/OpenRB-338.app
rm -rf "$app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp -p "$ui" "$app/Contents/MacOS/OpenRB-338"
chmod 755 "$app/Contents/MacOS/OpenRB-338"
if [ -n "$cli" ]; then
    cp -p "$cli" "$app/Contents/MacOS/orb"
    chmod 755 "$app/Contents/MacOS/orb"
fi

# The icon: the committed app_icon part at the iconset sizes (16 to 1024), then iconutil.
if [ -n "${ICNS:-}" ]; then
    cp "$ICNS" "$app/Contents/Resources/OpenRB-338.icns"
else
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    cargo run --manifest-path "$root/Cargo.toml" --release --locked -q -p orb-ui-art --bin ui-art \
        -- iconset "$tmp/OpenRB-338.iconset" >/dev/null
    iconutil -c icns "$tmp/OpenRB-338.iconset" -o "$app/Contents/Resources/OpenRB-338.icns"
fi

# The oldest macOS the binaries run on: the highest minos of the Mach-O headers.
minos=$(otool -l "$app/Contents/MacOS/OpenRB-338" | awk '
    $1 == "minos" { print $2 }
    $1 == "cmd" && $2 == "LC_VERSION_MIN_MACOSX" { want = 1 }
    want && $1 == "version" { print $2; want = 0 }' | sort -V | tail -1)
[ -n "$minos" ] || { echo "no minos in the Mach-O headers of $ui" >&2; exit 1; }

cat > "$app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>OpenRB-338</string>
    <key>CFBundleIconFile</key>
    <string>OpenRB-338</string>
    <key>CFBundleIdentifier</key>
    <string>$bundle_id</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>OpenRB-338</string>
    <key>CFBundleDisplayName</key>
    <string>OpenRB-338</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$version</string>
    <key>CFBundleVersion</key>
    <string>$build</string>
    <key>OpenRBBuildTime</key>
    <string>$built</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.music</string>
    <key>LSMinimumSystemVersion</key>
    <string>$minos</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright (C) 2026 the OpenRB-338 authors. GPL-3.0-only.</string>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
</dict>
</plist>
EOF
plutil -lint "$app/Contents/Info.plist" >/dev/null

# Ad hoc: the CLI first (explicitly: the linker only signed it on its own), then the bundle,
# whose seal covers both executables. No --deep.
if [ -n "$cli" ]; then
    codesign --force --sign - "$app/Contents/MacOS/orb"
fi
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"

# Fixed times (SOURCE_DATE_EPOCH, default now) so two builds of one input have equal files.
stamp=$(TZ=UTC date -r "${SOURCE_DATE_EPOCH:-$(date +%s)}" +%Y%m%d%H%M.%S)
find "$app" -exec touch -h -t "$stamp" {} +
echo "made $app (version $version, build $build, macOS $minos or newer)"

#!/usr/bin/env bash
# Builds dist/<target>/OpenRB-338-<YYYY.MMDD.HHMM>-<x86_64|aarch64>.AppImage, the Linux package of
# OpenRB-338 and rb (task-112; the version is the UTC build time, scripts/version.sh).
#
#   scripts/package-appimage.sh x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu
#
# Builds the target with scripts/build.sh first (Docker off a matching Linux host); SKIP_BUILD=1
# uses the binaries already in dist/<target>/bin/ (release.yml downloads them from the package
# jobs). The AppDir is laid out on any host; the AppImage is then packed by appimagetool
# (pinned below: version and sha256), natively on a Linux host of the target's architecture, and
# in the Ubuntu 22.04 build image (Docker) anywhere else. appimagetool runs with
# --appimage-extract-and-run, so no FUSE is needed. The icons come from `ui-art iconset`, which
# needs cargo on the host. docs/building.md ("The AppImage") describes the result.
# Environment as scripts/build.sh; APPIMAGETOOL names an appimagetool to use instead of the
# download.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
target=${1:?usage: scripts/package-appimage.sh x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu}
dist=${DIST:-$root/dist}
case $target in
    x86_64-unknown-linux-gnu)
        arch=x86_64
        platform=linux/amd64
        tool_sha=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
        ;;
    aarch64-unknown-linux-gnu)
        arch=aarch64
        platform=linux/arm64
        tool_sha=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
        ;;
    *) echo "not a Linux target: $target" >&2; exit 2 ;;
esac
tool_version=1.9.1
tool_url=https://github.com/AppImage/appimagetool/releases/download/$tool_version/appimagetool-$arch.AppImage

export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(date +%s)}
version=$("$root/scripts/version.sh")

[ -n "${SKIP_BUILD:-}" ] || "$root/scripts/build.sh" "$target"
bin=$dist/$target/bin
for f in OpenRB-338 rb; do
    [ -f "$bin/$f" ] || { echo "missing $bin/$f (run scripts/build.sh $target)" >&2; exit 1; }
done

sha256() { if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi; }

# appimagetool: downloaded once into dist/tools/, and checked against the pinned sha256 every run
tool=${APPIMAGETOOL:-$dist/tools/appimagetool-$tool_version-$arch.AppImage}
if [ -z "${APPIMAGETOOL:-}" ]; then
    if [ ! -f "$tool" ]; then
        mkdir -p "$dist/tools"
        curl -sSfL -o "$tool.part" "$tool_url"
        mv "$tool.part" "$tool"
    fi
    got=$(sha256 "$tool" | cut -d' ' -f1)
    [ "$got" = "$tool_sha" ] || {
        echo "appimagetool $arch: sha256 $got, expected $tool_sha (delete $tool)" >&2
        exit 1
    }
    chmod +x "$tool"
fi

# The AppDir. Only the two programs and their metadata go in: no library is bundled (the host
# provides libasound, X11 or Wayland and the GL or Vulkan driver; docs/building.md).
appdir=$dist/$target/OpenRB-338.AppDir
rm -rf "$appdir"
mkdir -p "$appdir/usr/bin" "$appdir/usr/share/applications" "$appdir/usr/share/doc/OpenRB-338"
cp "$bin/OpenRB-338" "$bin/rb" "$appdir/usr/bin/"
chmod 755 "$appdir/usr/bin/OpenRB-338" "$appdir/usr/bin/rb"
cp "$root/LICENSE" "$appdir/LICENSE"
cp "$root/LICENSE" "$appdir/usr/share/doc/OpenRB-338/LICENSE"

# AppRun: OpenRB-338 by default; the bundled CLI as `rb`, when the AppImage is run through a
# symlink named rb (the runtime puts the name in ARGV0) or with --rb as the first argument.
cat > "$appdir/AppRun" <<'EOF'
#!/bin/sh
here=$(dirname "$(readlink -f "$0")")
self=${ARGV0:-$0}
if [ "${self##*/}" = rb ]; then
    exec "$here/usr/bin/rb" "$@"
fi
if [ "${1:-}" = --rb ]; then
    shift
    exec "$here/usr/bin/rb" "$@"
fi
exec "$here/usr/bin/OpenRB-338" "$@"
EOF
chmod 755 "$appdir/AppRun"

# The desktop entry: the file name and StartupWMClass are the app id the window sets
# (with_app_id("OpenRB-338") in rb-ui), so Wayland and X11 shells match the window to it.
cat > "$appdir/usr/share/applications/OpenRB-338.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=OpenRB-338
GenericName=Bass line and drum machine
Comment=A reimplementation of the audio engine of Propellerhead ReBirth RB-338
Exec=OpenRB-338 %f
Icon=OpenRB-338
Terminal=false
Categories=AudioVideo;Audio;Music;
StartupWMClass=OpenRB-338
X-AppImage-Version=$version
EOF
cp "$appdir/usr/share/applications/OpenRB-338.desktop" "$appdir/OpenRB-338.desktop"

# The icon: the committed app_icon part (assets/ui/app_icon.svg) rendered with resvg by ui-art,
# at 16 to 512 px (the 64 px one is the 32 px size at 2x).
icons=$(mktemp -d)
trap 'rm -rf "$icons"' EXIT
cargo run --manifest-path "$root/Cargo.toml" --release --locked -q -p rb-ui-art --bin ui-art \
    -- iconset "$icons" >/dev/null
for pair in 16:icon_16x16 32:icon_32x32 64:icon_32x32@2x 128:icon_128x128 256:icon_256x256 \
    512:icon_512x512; do
    size=${pair%%:*}
    dir=$appdir/usr/share/icons/hicolor/${size}x${size}/apps
    mkdir -p "$dir"
    cp "$icons/${pair#*:}.png" "$dir/OpenRB-338.png"
done
cp "$icons/icon_256x256.png" "$appdir/OpenRB-338.png"
cp "$icons/icon_256x256.png" "$appdir/.DirIcon"

# the squashfs stores the file times: give every file the build time
stamp=$(TZ=UTC date -r "$SOURCE_DATE_EPOCH" +%Y%m%d%H%M.%S 2>/dev/null \
    || TZ=UTC date -d "@$SOURCE_DATE_EPOCH" +%Y%m%d%H%M.%S)
find "$appdir" -exec touch -h -t "$stamp" {} +

out=$dist/$target/OpenRB-338-$version-$arch.AppImage
rm -f "$out"

host_arch=
if [ "$(uname -s)" = Linux ]; then
    case $(uname -m) in
        x86_64 | amd64) host_arch=x86_64 ;;
        arm64 | aarch64) host_arch=aarch64 ;;
    esac
fi
if [ "$host_arch" = "$arch" ]; then
    # never inherited: the fallback is for Docker on macOS only
    OPENRB_APPIMAGE_EMULATED='' ARCH=$arch "$root/scripts/appimage-pack.sh" "$tool" "$appdir" "$out"
else
    # packing in the Ubuntu 22.04 image of build.sh (the other architecture runs emulated)
    image=openrb-linux-build:$arch
    docker build --platform "$platform" -t "$image" \
        -f "$root/scripts/docker/linux-build.Dockerfile" "$root/scripts/docker"
    # Docker Desktop on a Mac cannot exec the x86_64 appimagetool (see appimage-pack.sh)
    emulated=
    if [ "$(uname -s)" = Darwin ] && [ "$arch" = x86_64 ]; then emulated=1; fi
    docker run --rm --platform "$platform" -e "ARCH=$arch" -e SOURCE_DATE_EPOCH \
        -e "OPENRB_APPIMAGE_EMULATED=$emulated" \
        -v "$root:$root" -v "$dist:$dist" -w "$root" "$image" \
        "$root/scripts/appimage-pack.sh" "$tool" "$appdir" "$out"
fi
rm -rf "$appdir"
ls -l "$out"

#!/usr/bin/env bash
# Smoke test of an AppImage in a stock ubuntu:22.04 container (task-112). Needs a running
# Docker; the container is removed afterwards. The image runs for the AppImage's architecture
# (emulated off a matching host).
#
#   scripts/appimage-smoke.sh dist/<target>/OpenRB-338-<version>-<arch>.AppImage
#
# It installs the runtime libraries the host must provide (docs/building.md), then checks:
#   - the AppImage extracts, with the desktop entry, the icons and LICENSE, and bundles no
#     glibc (no libc.so.6, ld-linux, libm, libpthread) and no ELF file but OpenRB-338 and rb;
#   - the most recent GLIBC_ version the two programs need;
#   - `--rb --help` and `--rb info` on a committed song (the CLI inside), also through a
#     symlink named rb;
#   - OpenRB-338 --screenshot under xvfb writes a PNG with Vulkan, with OpenGL, automatically
#     (it must choose Vulkan) and with the Vulkan ICDs hidden (the fallback: OpenGL).
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
appimage=${1:?usage: scripts/appimage-smoke.sh OpenRB-338-<version>-<arch>.AppImage}
appimage=$(cd "$(dirname "$appimage")" && pwd)/$(basename "$appimage")
case $appimage in
    *-x86_64.AppImage) platform=linux/amd64 ;;
    *-aarch64.AppImage) platform=linux/arm64 ;;
    *) echo "not an OpenRB-338 AppImage name: $appimage" >&2; exit 2 ;;
esac

# the same pinned base image as scripts/docker/linux-build.Dockerfile
base=ubuntu:22.04@sha256:e0f80c829509ef4df9e76bd8944eb7b8ec25cbb379b112a7f9448b2c77a77c9b
song=$root/tests/songs/shipped/Calm.rbs

# Docker Desktop on a Mac cannot exec the x86_64 runtime (docs/building.md): only there the
# smoke test runs the unpacked AppRun instead. release.yml never sets this.
emulated= # never inherited from the environment
if [ "$(uname -s)" = Darwin ] && [ "$platform" = linux/amd64 ]; then emulated=1; fi

docker run --rm --platform "$platform" -e "OPENRB_APPIMAGE_EMULATED=$emulated" \
    -v "$appimage:/t/app.AppImage:ro" \
    -v "$song:/t/song.rbs:ro" "$base" bash -euo pipefail -c '
export DEBIAN_FRONTEND=noninteractive
apt-get update -qq
apt-get install -y -qq --no-install-recommends libasound2 libxcursor1 libxrandr2 libxi6 \
    libxkbcommon-x11-0 libvulkan1 mesa-vulkan-drivers libegl1 xvfb xauth binutils desktop-file-utils \
    squashfs-tools >/dev/null
cp /t/app.AppImage /tmp/app.AppImage
chmod +x /tmp/app.AppImage
cd /tmp
# run_app ARGS: the AppImage as a user runs it. Docker Desktop on a Mac refuses to exec an
# x86_64 AppImage (its runtime) under amd64 emulation: then the squashfs is unpacked with
# unsquashfs (it starts after the ELF runtime) and its AppRun is run, with ARGV0 set by hand.
if [ "$OPENRB_APPIMAGE_EMULATED" != 1 ]; then
    # the normal path (Linux, CI): the AppImage must run, or the script fails
    ./app.AppImage --appimage-extract >/dev/null
    run_app() { ./app.AppImage --appimage-extract-and-run "$@"; }
    run_as_rb() { ./links/rb --appimage-extract-and-run "$@"; }
else
    echo "EMULATED (local x86_64 on macOS only): running squashfs-root/AppRun, not the runtime"
    off=$(readelf -h app.AppImage | awk "/Start of section headers/ {o=\$5} \
        /Size of section headers/ {s=\$5} /Number of section headers/ {n=\$5} END {print o+s*n}")
    unsquashfs -q -o "$off" -d squashfs-root app.AppImage
    run_app() { squashfs-root/AppRun "$@"; }
    run_as_rb() { ARGV0=./links/rb squashfs-root/AppRun "$@"; }
fi
echo "== AppDir"
ls -A squashfs-root
desktop-file-validate squashfs-root/OpenRB-338.desktop && echo "desktop entry valid"
find squashfs-root/usr/share/icons -name "*.png" | sort
test -s squashfs-root/LICENSE
echo "== ELF files (only OpenRB-338 and rb are expected)"
find squashfs-root -type f -exec sh -c "head -c4 \"\$1\" | grep -q ELF && echo \"\$1\"" _ {} \; \
    | sort | tee /tmp/elf.txt
test "$(wc -l < /tmp/elf.txt)" = 2
if find squashfs-root | grep -E "/(libc\.so|ld-linux|libm\.so|libpthread|libdl\.so)"; then
    echo "a glibc file is bundled" >&2
    exit 1
fi
echo "no glibc bundled"
echo "== most recent GLIBC_ version per program"
for f in $(cat /tmp/elf.txt); do
    max=$(objdump -T "$f" | grep -o "GLIBC_[0-9.]*" | sort -uV | tail -1)
    echo "$f $max"
    # the highest must not be above GLIBC_2.35 (sort -V puts the highest last)
    top=$(printf "%s\n" "$max" GLIBC_2.35 | sort -V | tail -1)
    test "$top" = GLIBC_2.35
done
echo "== rb inside (output goes to files first: no pipe into head)"
run_app --rb --help > /tmp/help.txt 2>&1
head -5 /tmp/help.txt
grep -qi usage /tmp/help.txt
run_app --rb info /t/song.rbs > /tmp/info.txt 2>&1
cat /tmp/info.txt
grep -qi tempo /tmp/info.txt
mkdir links
ln -s ../app.AppImage links/rb
run_as_rb --help > /tmp/help2.txt 2>&1
head -3 /tmp/help2.txt
cmp /tmp/help.txt /tmp/help2.txt
echo "the symlink named rb ran the CLI"
echo "== OpenRB-338 --screenshot under xvfb"
ls /usr/share/vulkan/icd.d
# shot LABEL BACKEND VAR=VALUE...: a screenshot with that environment, checked for the PNG
# signature and for the backend wgpu chose (OPENRB_GPU_INFO=1 makes OpenRB-338 print it).
shot() {
    local label=$1 want=$2; shift 2
    rm -f /tmp/shot.png
    env OPENRB_GPU_INFO=1 "$@" xvfb-run -a bash -c \
        "$(declare -f run_app); run_app --demo --screenshot /tmp/shot.png" > /tmp/ui.txt 2>&1 || {
        echo "$label: failed" >&2
        cat /tmp/ui.txt
        exit 1
    }
    test -s /tmp/shot.png
    test "$(od -An -tx1 -N8 /tmp/shot.png | tr -d " \n")" = 89504e470d0a1a0a
    grep "^gpu: " /tmp/ui.txt
    grep -q "^gpu: $want " /tmp/ui.txt
    echo "$label: PNG signature ok, $(stat -c %s /tmp/shot.png) B, backend $want"
}
# wgpu draws through Vulkan (Mesa lavapipe, software, in the container) or OpenGL (Mesa
# llvmpipe through EGL: libegl1). Each is forced; then nothing is forced (wgpu sorts the
# adapters by device type, lavapipe and llvmpipe are both software, so the backend order
# decides); then the Vulkan ICDs are hidden: the window must open with OpenGL (task-135).
shot "WGPU_BACKEND=vulkan" Vulkan WGPU_BACKEND=vulkan
shot "WGPU_BACKEND=gl" Gl WGPU_BACKEND=gl
shot "backend automatic, Vulkan present" Vulkan X=1
shot "no Vulkan ICD, backend automatic" Gl VK_ICD_FILENAMES=/nonexistent
echo SMOKE-OK
'

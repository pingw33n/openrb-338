#!/usr/bin/env bash
# Builds the dist-profile binaries of orb (the CLI) and OpenRB-338 (orb-ui) for one target triple and
# copies them to dist/<target>/bin/ (task-108). scripts/package.sh packs them.
#
#   scripts/build.sh TARGET
#
# TARGET is one of the six: x86_64/aarch64-unknown-linux-gnu, x86_64/aarch64-pc-windows-msvc,
# x86_64/aarch64-apple-darwin. How a target is built depends on the machine (docs/building.md):
#   native      the host builds it with cargo (a CI runner, or a Mac for macOS targets)
#   cargo-xwin  Windows targets off Windows (needs `cargo install cargo-xwin`)
#   docker      Linux targets on a macOS or Windows host, or on a Linux host of the other
#               architecture (needs a running Docker)
# Environment passed through to the build (the version string, task-102): SOURCE_DATE_EPOCH,
# ORB_BUILD_BRANCH, GITHUB_REF_NAME. DIST sets the output folder (default: dist/).
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
target=${1:?usage: scripts/build.sh TARGET}
dist=${DIST:-$root/dist}
# an absolute path: a Docker build mounts it into the container at the same path (task-142)
mkdir -p "$dist"
dist=$(cd "$dist" && pwd)
bin_dir=$dist/$target/bin

case $(uname -s) in
    Darwin) host_os=macos ;;
    Linux) host_os=linux ;;
    MINGW* | MSYS* | CYGWIN*) host_os=windows ;;
    *) echo "unsupported host: $(uname -s)" >&2; exit 2 ;;
esac

arch=${target%%-*}
case $target in
    *-apple-darwin) os=macos; exe= ;;
    *-pc-windows-msvc) os=windows; exe=.exe ;;
    *-unknown-linux-gnu) os=linux; exe= ;;
    *) echo "unknown target: $target" >&2; exit 2 ;;
esac

# The dist profile of Cargo.toml: fat LTO, one codegen unit, stripped (task-132). Its binaries
# are in target/<target>/dist/.
profile=dist
cargo_args=(--profile "$profile" --locked --target "$target" -p orb-cli -p orb-ui)
target_dir=${CARGO_TARGET_DIR:-$root/target}

build_docker() {
    local platform=linux/amd64
    [ "$arch" = aarch64 ] && platform=linux/arm64
    local image=openrb-linux-build:$arch
    docker build --platform "$platform" -t "$image" \
        -f "$root/scripts/docker/linux-build.Dockerfile" "$root/scripts/docker"
    local git_common mounts=()
    git_common=$(cd "$root" && git rev-parse --path-format=absolute --git-common-dir)
    # a worktree's .git file points into the common git dir, so mount it at its own path
    case $git_common in "$root"/*) ;; *) mounts+=(-v "$git_common:$git_common:ro") ;; esac
    local env=(-e SOURCE_DATE_EPOCH -e ORB_BUILD_BRANCH -e GITHUB_REF_NAME -e "DIST=$dist"
        -e GIT_CONFIG_COUNT=1 -e GIT_CONFIG_KEY_0=safe.directory -e "GIT_CONFIG_VALUE_0=*"
        -e CARGO_TARGET_DIR=/target)
    # a DIST outside the checkout is not in the container: mount it at the same path, or the
    # binaries die with the container
    case $dist in "$root" | "$root"/*) ;; *) mounts+=(-v "$dist:$dist") ;; esac
    # stale binaries of an earlier run must not pass the check below
    rm -rf "$bin_dir"
    # ${mounts[@]+...}: bash 3.2 (macOS) counts an empty array as unbound under set -u
    docker run --rm --platform "$platform" "${env[@]}" \
        -v "$root:$root" -w "$root" ${mounts[@]+"${mounts[@]}"} \
        -v "openrb-cargo-$arch:/cargo/registry" \
        -v "openrb-rustup-$arch:/rustup" \
        -v "openrb-target-$arch:/target" \
        "$image" scripts/build.sh "$target"
    # the success line is only true when the files are on the host
    local b
    for b in orb OpenRB-338; do
        [ -f "$bin_dir/$b" ] || {
            echo "docker build finished but $bin_dir/$b is missing" >&2
            exit 1
        }
    done
    echo "built $target -> $bin_dir"
}

# Only a Linux target cares about the host CPU: it builds natively on a Linux host of the
# same architecture, and in Docker anywhere else (a Linux host of the other architecture
# needs qemu emulation in Docker).
if [ "$os" = linux ]; then
    host_arch=
    if [ "$host_os" = linux ]; then
        case $(uname -m) in
            x86_64 | amd64) host_arch=x86_64 ;;
            arm64 | aarch64) host_arch=aarch64 ;;
        esac
    fi
    if [ "$host_arch" != "$arch" ]; then
        build_docker
        exit 0
    fi
fi

case $os in
    windows)
        if [ "$host_os" = windows ]; then
            rustup target add "$target"
            cargo build "${cargo_args[@]}"
        else
            command -v cargo-xwin >/dev/null || {
                echo "cargo-xwin is missing: cargo install cargo-xwin --locked" >&2
                exit 1
            }
            rustup target add "$target"
            # The vendored libvorbis (aotuv_lancer_vorbis_sys) calls _mm_prefetch(float *, ..).
            # MSVC's cl accepts that with a warning; clang-cl 21+ makes it an error. cargo-xwin
            # overwrites CFLAGS_<target> and CC_<target>, but clang-cl reads CL itself.
            export CL=-Wno-incompatible-pointer-types
            # the first run downloads the Microsoft CRT and SDK into the cargo-xwin cache
            # (~/Library/Caches/cargo-xwin on macOS, ~/.cache/cargo-xwin on Linux)
            XWIN_ACCEPT_LICENSE=1 cargo xwin build "${cargo_args[@]}"
        fi
        ;;
    macos)
        [ "$host_os" = macos ] || { echo "macOS targets build only on macOS" >&2; exit 1; }
        rustup target add "$target"
        # The macOS app (task-110; both targets) deliberately needs macOS 11.0 (Big Sur), the Rust
        # default of aarch64; the x86_64 default would be 10.12. LSMinimumSystemVersion is
        # read back from the binary.
        export MACOSX_DEPLOYMENT_TARGET=11.0
        cargo build "${cargo_args[@]}"
        ;;
    linux)
        cargo build "${cargo_args[@]}"
        ;;
esac

mkdir -p "$bin_dir"
for b in orb OpenRB-338; do
    cp "$target_dir/$target/$profile/$b$exe" "$bin_dir/$b$exe"
done
echo "built $target -> $bin_dir"

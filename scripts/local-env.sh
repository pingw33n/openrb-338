#!/bin/sh
# Builds and tests the workspace on a machine without the ALSA development package.
#
# `orb-cli` depends on `orb-player`, which depends on `cpal`, whose `alsa-sys` needs pkg-config and
# `libasound.so` at build time. A distribution that ships only the runtime library (`libasound.so.2`,
# as a minimal Linux install does) fails to link `orb`. This script stands in for the missing
# pieces, without root and without installing anything:
#
#   - a `pkg-config` shim on PATH that answers alsa-sys' query (`--libs` -> -lasound);
#   - a directory with `libasound.so` -> the runtime library, passed with `-L`.
#
# With the real development package (`pkg-config` and `libasound2-dev`) nothing of this is needed:
# the shim only fills what is absent, and the real pkg-config wins when it is installed.
#
# Usage:  sh scripts/local-env.sh cargo test --release -p orb-mcp
#         sh scripts/local-env.sh cargo build --release -p orb-cli
#
# The same file documents the environment for the checks in docs/building.md.

set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
SHIM="$ROOT/target/local-env/pkg-config"
LIBDIR="$ROOT/target/local-env/lib"

if ! command -v pkg-config >/dev/null 2>&1; then
    mkdir -p "$(dirname "$SHIM")"
    cat >"$SHIM" <<'EOF'
#!/bin/sh
# Enough of pkg-config for alsa-sys: the library it looks for, and an empty include path.
for arg in "$@"; do
    case "$arg" in
        --libs) echo "-lasound" ;;
        --cflags) echo "" ;;
        --modversion) echo "1.2.0" ;;
    esac
done
exit 0
EOF
    chmod +x "$SHIM"
    PATH="$(dirname "$SHIM"):$PATH"
    export PATH
    echo "local-env: pkg-config not installed, using the shim at $SHIM" >&2
fi

# The runtime library stands in for the missing development one: `alsa-sys` asks pkg-config for
# the flags, but the linker still needs a `libasound.so` to resolve against. The shim answers the
# query, so this step cannot be keyed on pkg-config's answer: it runs whenever the development
# library is absent.
DEV_LIB=""
for dir in /usr/lib/x86_64-linux-gnu /usr/lib/aarch64-linux-gnu /usr/lib /usr/lib64; do
    if [ -e "$dir/libasound.so" ]; then
        DEV_LIB="$dir/libasound.so"
        break
    fi
done

if [ -z "$DEV_LIB" ]; then
    mkdir -p "$LIBDIR"
    for lib in /usr/lib/x86_64-linux-gnu/libasound.so.2 /usr/lib/aarch64-linux-gnu/libasound.so.2 \
               /usr/lib/libasound.so.2 /usr/lib64/libasound.so.2; do
        if [ -e "$lib" ]; then
            ln -sf "$lib" "$LIBDIR/libasound.so"
            RUSTFLAGS="${RUSTFLAGS:-} -L $LIBDIR"
            export RUSTFLAGS
            echo "local-env: linking against $lib through $LIBDIR" >&2
            break
        fi
    done
fi

exec "$@"

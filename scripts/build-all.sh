#!/usr/bin/env bash
# Packages every target practical from this machine and says which it skipped (task-108).
#
#   scripts/build-all.sh [TARGET...]
#
# Without arguments: all six targets. A target that cannot be built here is skipped with the
# reason, and the script exits 1 if a target that should have worked failed. See
# docs/building.md for what each host can build.
set -uo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
all=(
    aarch64-apple-darwin x86_64-apple-darwin
    x86_64-pc-windows-msvc aarch64-pc-windows-msvc
    x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu
)
targets=("$@")
[ ${#targets[@]} -gt 0 ] || targets=("${all[@]}")

host=$(uname -s)
done_=()
skipped=()
failed=()
for t in "${targets[@]}"; do
    case $t in
        *-apple-darwin)
            if [ "$host" != Darwin ]; then skipped+=("$t: macOS targets need a Mac"); continue; fi ;;
        *-pc-windows-msvc)
            if [ "$host" != Darwin ] && [ "$host" != Linux ] && [ -z "${MSYSTEM:-}" ]; then
                skipped+=("$t: unsupported host"); continue
            fi
            if [ -n "${MSYSTEM:-}" ]; then :; elif ! command -v cargo-xwin >/dev/null; then
                skipped+=("$t: cargo-xwin is not installed"); continue
            fi ;;
        *-unknown-linux-gnu)
            # native only on a Linux host of the target's architecture; else Docker
            native=
            if [ "$host" = Linux ]; then
                case $(uname -m) in
                    x86_64 | amd64) [ "${t%%-*}" = x86_64 ] && native=1 ;;
                    arm64 | aarch64) [ "${t%%-*}" = aarch64 ] && native=1 ;;
                esac
            fi
            if [ -z "$native" ] && ! docker info >/dev/null 2>&1; then
                skipped+=("$t: Docker is not running (needed off a matching Linux host)")
                continue
            fi ;;
    esac
    echo "=== $t"
    if "$root/scripts/package.sh" "$t"; then done_+=("$t"); else failed+=("$t"); fi
done

echo
echo "built:   ${done_[*]:-none}"
for s in "${skipped[@]:-}"; do [ -z "$s" ] || echo "skipped: $s"; done
echo "failed:  ${failed[*]:-none}"
ls -l "$root"/dist/*/OpenRB-338-*.tar.gz "$root"/dist/*/OpenRB-338-*.zip 2>/dev/null
[ ${#failed[@]} -eq 0 ]

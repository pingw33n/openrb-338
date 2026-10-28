#!/usr/bin/env bash
# Prints the version of a package (task-133): the build time in UTC, to the minute, as
# YYYY.MMDD.HHMM, from SOURCE_DATE_EPOCH (default: now). It is the number the About box shows
# (`2026.1028.1930 (7bd8876)`, task-102), the file names carry, the macOS bundle has as
# CFBundleShortVersionString and CFBundleVersion, and the release tag is `v` + it (task-113).
# The `version` of Cargo.toml (cargo needs one) is shown nowhere.
#
#   SOURCE_DATE_EPOCH=1793215800 scripts/version.sh              # 2026.1028.1930
#   SOURCE_DATE_EPOCH=1793215800 scripts/version.sh --time       # 2026-10-28 19:30:00
#   scripts/version.sh --epoch v2026.1028.1930                   # 1793215800
#
# --time: the full time (seconds included; they are shown nowhere but OpenRBBuildTime).
# --epoch TAG: the SOURCE_DATE_EPOCH of a release tag, which must match
# ^v[0-9]{4}\.[0-9]{4}\.[0-9]{4}$ (`v2026.1028.1930`; seconds 0); it fails for any other text
# (the version without the v too) and for a date that does not exist.
set -euo pipefail

# fmt EPOCH FORMAT: strftime in UTC (BSD date -r, GNU date -d)
fmt() {
    date -u -r "$1" "+$2" 2>/dev/null || date -u -d "@$1" "+$2"
}

case ${1:-} in
    --epoch)
        tag=${2:?usage: scripts/version.sh --epoch TAG}
        [[ $tag =~ ^v([0-9]{4})\.([0-9]{2})([0-9]{2})\.([0-9]{2})([0-9]{2})$ ]] || {
            echo "not a version or release tag (v2026.1028.1930): $tag" >&2
            exit 1
        }
        y=${BASH_REMATCH[1]} mo=${BASH_REMATCH[2]} d=${BASH_REMATCH[3]}
        h=${BASH_REMATCH[4]} mi=${BASH_REMATCH[5]}
        # the round trip rejects a month 13 or an hour 25, which date might wrap around
        if epoch=$(date -u -j -f '%Y%m%d%H%M%S' "$y$mo$d${h}${mi}00" +%s 2>/dev/null); then :; else
            epoch=$(date -u -d "$y-$mo-$d $h:$mi:00" +%s 2>/dev/null) || {
                echo "not a valid date: $tag" >&2
                exit 1
            }
        fi
        [ "$(fmt "$epoch" %Y.%m%d.%H%M)" = "$y.$mo$d.$h$mi" ] || {
            echo "not a valid date: $tag" >&2
            exit 1
        }
        echo "$epoch"
        ;;
    --time) fmt "${SOURCE_DATE_EPOCH:-$(date +%s)}" '%Y-%m-%d %H:%M:%S' ;;
    "") fmt "${SOURCE_DATE_EPOCH:-$(date +%s)}" '%Y.%m%d.%H%M' ;;
    *)
        echo "usage: scripts/version.sh [--time | --epoch TAG]" >&2
        exit 1
        ;;
esac

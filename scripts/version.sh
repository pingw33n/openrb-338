#!/usr/bin/env bash
# Prints the version of a package, the build time in UTC as YYYY_MM_DD_HH_MM_SS (the form of the
# release tag, task-113), from SOURCE_DATE_EPOCH (default: now). It is the time the About box
# shows (`2026-10-28 19:30:00`, task-102). The package names and the macOS bundle take it; the
# `version` of Cargo.toml (cargo needs one) is shown nowhere.
#
#   SOURCE_DATE_EPOCH=1793215800 scripts/version.sh    # 2026_10_28_19_30_00
set -euo pipefail
epoch=${SOURCE_DATE_EPOCH:-$(date +%s)}
if date -u -r "$epoch" +%Y_%m_%d_%H_%M_%S 2>/dev/null; then :; else
    date -u -d "@$epoch" +%Y_%m_%d_%H_%M_%S
fi

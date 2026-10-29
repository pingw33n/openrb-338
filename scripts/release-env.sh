#!/usr/bin/env bash
# Prints the build environment that all packages of one release share, as KEY=VALUE lines
# (task-108; release.yml appends them to $GITHUB_OUTPUT). The version string of the binaries
# (task-102) is made from them, so every package of a release carries the same build time and
# the same main/not-main form of the hash:
#
#   SOURCE_DATE_EPOCH  the build time (Unix seconds). With a TAG argument, v<YYYY.MMDD.HHMM>
#                      (task-113), the time of the tag (`scripts/version.sh --epoch TAG`, seconds
#                      0); a text that is not such a tag fails. Without one, SOURCE_DATE_EPOCH
#                      if set, else now.
#   RB_BUILD_BRANCH    "main" when HEAD is on origin/main (a tag's detached HEAD has no branch)
#
#   scripts/release-env.sh [TAG]
#
# Run in a checkout with full history (fetch-depth: 0).
set -euo pipefail

if [ $# -ge 1 ]; then
    SOURCE_DATE_EPOCH=$("$(dirname "$0")/version.sh" --epoch "$1")
fi
echo "SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH:-$(date +%s)}"
if git merge-base --is-ancestor HEAD origin/main 2>/dev/null; then
    echo "RB_BUILD_BRANCH=main"
else
    echo "RB_BUILD_BRANCH=other"
fi

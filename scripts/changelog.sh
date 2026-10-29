#!/usr/bin/env bash
# Prints the changelog of a release as Markdown (task-113): the commits between the previous
# release tag and this one, grouped by the area prefix of their subject, without the backlog:
# commits, each with the task ids of its `Task:` lines. It is the body of the draft release
# (release.yml) and runs locally for a preview:
#
#   scripts/changelog.sh v2026.1028.1930                  # since the latest earlier release tag
#   scripts/changelog.sh v2026.1020.1000 v2026.1028.1930  # since a given tag
#   scripts/changelog.sh HEAD                             # a preview of the next release
#   scripts/changelog.sh --since abc1234 v2026.1028.1930  # since a commit (the first release)
#
# NEW is a tag or any revision. Without PREV the previous release tag is the greatest tag
# matching ^v[0-9]{4}\.[0-9]{4}\.[0-9]{4}$ that sorts before NEW (the form is zero-padded, so
# the text order is the version order). With no such tag (the first release), and without
# --since, the whole history up to NEW is listed and the text says so. The tags are read from
# the repository the script runs in.
#
# Areas, in this order: engine, cli, tools, tests, docs; a subject with another prefix (or
# none) goes to "other". Commits of the area backlog are left out, merge commits are skipped.
set -euo pipefail

since=
if [ "${1:-}" = "--since" ]; then
    since=${2:?usage: scripts/changelog.sh [--since REV] [PREV] NEW}
    shift 2
fi
case $# in
    1) prev='' new=$1 ;;
    2) prev=$1 new=$2 ;;
    *)
        echo "usage: scripts/changelog.sh [--since REV] [PREV] NEW" >&2
        exit 1
        ;;
esac
git rev-parse --verify --quiet "$new^{commit}" >/dev/null || {
    echo "not a revision: $new" >&2
    exit 1
}

pattern='^v[0-9]{4}\.[0-9]{4}\.[0-9]{4}$'
if [ -z "$prev" ] && [ -z "$since" ]; then
    # the latest release tag before NEW, in version order
    prev=$(git tag --list 'v*' | { grep -E "$pattern" || true; } | LC_ALL=C sort |
        LC_ALL=C awk -v new="$new" '$0 < new { p = $0 } END { print p }')
fi
if [ -n "$prev" ]; then
    git rev-parse --verify --quiet "$prev^{commit}" >/dev/null || {
        echo "not a revision: $prev" >&2
        exit 1
    }
    range="$prev..$new"
    from="since $prev"
elif [ -n "$since" ]; then
    git rev-parse --verify --quiet "$since^{commit}" >/dev/null || {
        echo "not a revision: $since" >&2
        exit 1
    }
    range="$since..$new"
    from="since $since (not included)"
else
    range=$new
    from="in the first release (all of the history)"
fi

# One record per commit: subject, US, body (RS ends a record). awk groups and prints.
git log --no-merges --format='%s%x1f%b%x1e' "$range" | LC_ALL=C awk -v from="$from" -v new="$new" '
BEGIN {
    RS = "\036"; FS = "\037"
    n = split("engine cli tools tests docs other", order, " ")
    total = 0; listed = 0
}
{
    subj = $1; body = $2
    sub(/^\n/, "", subj)
    if (subj == "") next
    total++
    area = "other"; text = subj
    if (match(subj, /^[a-z]+: /)) {
        a = substr(subj, 1, RLENGTH - 2)
        if (a == "backlog") next
        if (a == "engine" || a == "cli" || a == "tools" || a == "tests" || a == "docs") {
            area = a; text = substr(subj, RLENGTH + 1)
        }
    }
    # the task ids of the Task: lines of the body
    nb = split(body, lines, "\n"); ids = ""; delete seen
    for (i = 1; i <= nb; i++) {
        if (lines[i] !~ /^Task:/) continue
        l = lines[i]
        while (match(l, /task-[0-9]+/)) {
            id = substr(l, RSTART, RLENGTH)
            if (!(id in seen)) { seen[id] = 1; ids = ids (ids == "" ? "" : ", ") id }
            l = substr(l, RSTART + RLENGTH)
        }
    }
    count[area]++
    items[area, count[area]] = "- " text (ids == "" ? "" : " (" ids ")")
    listed++
}
END {
    printf "Changes %s up to %s: %d commits, %d listed (backlog commits are left out).\n",
        from, new, total, listed
    for (k = 1; k <= n; k++) {
        a = order[k]
        if (!count[a]) continue
        printf "\n### %s\n\n", a
        for (j = 1; j <= count[a]; j++) print items[a, j]
    }
}'

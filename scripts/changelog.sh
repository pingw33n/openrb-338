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
# matching ^v[0-9]{4}\.[0-9]{4}\.[0-9]{4}$ in NEW's history, other than NEW itself (the form is
# zero-padded, so the text order is the version order). With no such tag (the first release), and without
# --since, the whole history up to NEW is listed and the text says so. The tags are read from
# the repository the script runs in.
#
# Areas, in this order: engine, player, cli, ui, tools, tests, docs; a subject with none of them
# (or no prefix) goes to "other". A prefix may name several areas, comma-separated
# (`player,cli,ui: ...`, AGENTS.md): the commit is listed once, under its first known area,
# with the others after its task ids ("also cli, ui"). Commits whose areas are all backlog are
# left out, merge commits are skipped.
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
    # the latest release tag in NEW's history other than NEW itself, in version order (NEW may
    # be HEAD or a commit, which a text comparison with the tags would not order)
    prev=$(git tag --merged "$new" --list 'v*' | { grep -E "$pattern" || true; } |
        { grep -vxF "$new" || true; } | LC_ALL=C sort | tail -n 1)
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
    n = split("engine player cli ui tools tests docs other", order, " ")
    for (k = 1; k < n; k++) known[order[k]] = 1
    total = 0; listed = 0
}
{
    subj = $1; body = $2
    sub(/^\n/, "", subj)
    if (subj == "") next
    total++
    area = "other"; text = subj; also = ""
    if (match(subj, /^[a-z]+(,[a-z]+)*: /)) {
        na = split(substr(subj, 1, RLENGTH - 2), as, ",")
        backlog_only = 1; first = ""; delete got
        for (i = 1; i <= na; i++) {
            if (as[i] != "backlog") backlog_only = 0
            if (!(as[i] in known) || (as[i] in got)) continue
            got[as[i]] = 1
            if (first == "") first = as[i]
            else also = also (also == "" ? "" : ", ") as[i]
        }
        if (backlog_only) next
        if (first != "") { area = first; text = substr(subj, RLENGTH + 1) }
        else also = ""
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
    note = ids
    if (also != "") note = note (note == "" ? "" : "; ") "also " also
    items[area, count[area]] = "- " text (note == "" ? "" : " (" note ")")
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

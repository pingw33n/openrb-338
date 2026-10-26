#!/usr/bin/python3
"""task-010: hash and compare the exports of a campaign.

    analyze.py OUTROOT

OUTROOT holds one directory per rb_export.py run, named <protocol>_r<rep>[_<anything>], each with
<song>.wav or <song>.<k>.wav (k-th export in one process). Songs whose name starts with zz_other are
skipped (the song exported first in protocol P4). For every (protocol, song) it prints the SHA-256 of
each export and tools/rbcmp.py's verdict for every pair."""
import glob
import hashlib
import itertools
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
RBCMP = os.path.join(ROOT, "tools", "rbcmp.py")


def sha(p):
    with open(p, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def cmp(a, b):
    out = subprocess.run([sys.executable, RBCMP, "--json", a, b], capture_output=True, text=True).stdout
    return json.loads(out)


def short(r):
    """One line: first differing frame, max |d| per channel, differing samples, rms."""
    if r["equal"]:
        return "equal"
    fd = r.get("first_diff") or {}
    return "first diff frame %s, max |d| %s, %d samples differ, rms %.3g" % (
        fd.get("frame"), r["max_abs_diff"], r["differing_samples"], r["rms_diff"])


def main(root):
    entries = {}
    for p in sorted(glob.glob(os.path.join(root, "*", "*.wav"))):
        run = os.path.basename(os.path.dirname(p))
        stem = os.path.basename(p)[:-4]
        m = re.match(r"(p\w+?)_r(\d+)(?:_.*)?$", run)
        proto, rep = (m.group(1), m.group(2)) if m else (run, "0")
        m = re.match(r"(.*)\.(\d+)$", stem)
        song, k = (m.group(1), m.group(2)) if m else (stem, "1")
        if song.startswith("zz_other"):
            continue
        entries.setdefault((proto, song), []).append((rep, k, p))
    for (proto, song), lst in sorted(entries.items()):
        print("== %s %s" % (proto, song))
        hashes = {p: sha(p) for _, _, p in lst}
        for rep, k, p in lst:
            print("  r%s e%s %s" % (rep, k, hashes[p][:16]))
        for (r1, k1, a), (r2, k2, b) in itertools.combinations(lst, 2):
            verdict = "identical" if hashes[a] == hashes[b] else short(cmp(a, b))
            print("  r%se%s vs r%se%s: %s" % (r1, k1, r2, k2, verdict))


if __name__ == "__main__":
    main(sys.argv[1])

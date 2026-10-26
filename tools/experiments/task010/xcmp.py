#!/usr/bin/python3
"""task-010: compare export pairs across protocols.

    xcmp.py OUTROOT A:B...      (A and B relative to OUTROOT, e.g. p1_r1/808_snare.wav)"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from analyze import cmp, sha, short  # noqa: E402

root = sys.argv[1]
for pair in sys.argv[2:]:
    a, b = [os.path.join(root, x) for x in pair.split(":")]
    print("%-70s %s" % (pair, "identical" if sha(a) == sha(b) else short(cmp(a, b))))

#!/usr/bin/python3
"""task-010: fit the global 808 snare counter at the start of 808_snare exports.

    snare.py chain E1.wav E2.wav ...   exports in one process, in order; E1 is fresh (counter 0)
    snare.py ref REF.wav X.wav ...     REF a fresh export (counter 0), each X fitted against it

Model (ReBirth.dll sub_1002A1E4): per hit c = (c + 1) & 31 (word_1014CBD4, 0x1002A25D..0x1002A27D),
noise slot = word_100340DC[word_1003409C[c]] (word tables, 0x1002A28C / 0x1002A29D); T1 below is
word_1003409C, whose 32 values select one of the 4 noise samples word_100340DC[0..3] = 6..9.
808_snare has 8 hits per export, one per beat (22050 frames at 120 BPM). Hit h of export i sounds
like hit h of export j (first 8000 frames within 1 LSB) iff T1[(s_i + h + 1) & 31] ==
T1[(s_j + h + 1) & 31]. Hits are compared only at the same position h, because hits inside one
export differ even with the same noise (their offsets inside the 64-frame block differ)."""
import itertools
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wav import left  # noqa: E402

T1 = [0, 1, 2, 3, 1, 0, 3, 1, 0, 3, 2, 1, 0, 2, 3, 1, 0, 2, 3, 2, 0, 1, 2, 0, 3, 1, 2, 0, 1, 3, 2, 1]
W, L, HITS = 22050, 8000, 8


def equal_hits(a, b):
    return [max(abs(a[t] - b[t]) for t in range(h * W, h * W + L)) <= 1 for h in range(HITS)]


def pred(si, sj):
    return [T1[(si + h + 1) & 31] == T1[(sj + h + 1) & 31] for h in range(HITS)]


def fmt(m):
    return "".join("=" if v else "x" for v in m)


def main():
    mode, files = sys.argv[1], sys.argv[2:]
    e = [left(f) for f in files]
    if mode == "ref":
        for f, x in zip(files[1:], e[1:]):
            m = equal_hits(e[0], x)
            print("%-45s equal hits %s  start counter candidates %s"
                  % (f, fmt(m), [s for s in range(32) if pred(0, s) == m]))
        return
    n = len(e)
    meas = {(i, j): equal_hits(e[i], e[j]) for i in range(n) for j in range(i + 1, n)}
    for (i, j), m in meas.items():
        print("e%d vs e%d equal hits %s" % (i + 1, j + 1, fmt(m)))
    cands = [[0]] + [[s for s in range(32) if pred(0, s) == meas[(0, j)]] for j in range(1, n)]
    print("candidates per export (from e1):", cands)
    for combo in itertools.product(*cands):
        if all(pred(combo[i], combo[j]) == m for (i, j), m in meas.items()):
            print("consistent starts:", combo, "steps:", [(combo[k + 1] - combo[k]) % 32 for k in range(n - 1)])


if __name__ == "__main__":
    main()

#!/usr/bin/python3
"""task-010: 303 oscillator phase offset between exports, as a fit to whole idle buffers.

    phase.py REF.wav OTHER.wav...

Window: frames 1000..4000 of the left channel (the first note of 303_accent0). The note period P is
measured on REF (self-shift 600..760 samples, parabolic refinement). For each OTHER the time shift s
(|s| <= P/2, parabolic refinement) that best aligns it with REF gives the phase difference
d = (-s / P) mod 1 in note periods. The idle 303 advances 1024 * 1.8538e-4 / 2 periods per 1024-frame
buffer at pitch index 0 (docs/export_state.md 7.2), so k, the nearest whole number of buffers with
k * INC = d (mod 1), is printed with the misfit in periods. A misfit near 0 for every pair means the
exports differ by whole idle buffers; it is a fit, not a proof."""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wav import left  # noqa: E402

ST, LN = 1000, 3000
INC = 1024 * 1.8538e-4 / 2


def sse(a, b, s):
    return math.fsum((a[t] - b[t + s]) ** 2 for t in range(ST, ST + LN))


def best(a, b, lo, hi):
    vals = {s: sse(a, b, s) for s in range(lo, hi)}
    s = min(vals, key=vals.get)
    y0, y1, y2 = vals.get(s - 1, vals[s]), vals[s], vals.get(s + 1, vals[s])
    den = y0 - 2 * y1 + y2
    return s + (0.5 * (y0 - y2) / den if den else 0), math.sqrt(vals[s] / LN)


def main():
    ref = left(sys.argv[1])
    p, _ = best(ref, ref, 600, 760)
    print("period %.2f samples; idle advance %.5f periods/buffer" % (p, INC))
    for f in sys.argv[2:]:
        b = left(f)
        s, r = best(ref, b, -int(p / 2) - 2, int(p / 2) + 3)
        d = (-s / p) % 1.0
        e, k = min((abs(((k * INC - d) + 0.5) % 1.0 - 0.5), k) for k in range(-30, 31))
        print("%-45s shift %+8.2f rms %6.1f  dphi %.4f periods  nearest k %+d (misfit %.4f periods)"
              % (f, s, r, d, k, e))


if __name__ == "__main__":
    main()

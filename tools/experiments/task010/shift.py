#!/usr/bin/python3
"""task-010: is the difference of two exports a time shift of the waveform inside a window?

    shift.py A.wav B.wav START LEN MAXSHIFT

For s in [-MAXSHIFT, MAXSHIFT]: rms(a[t] - b[t+s]) over t in [START, START+LEN), left channel.
Prints the signal rms of A, the rms difference at s = 0, and the best s with its rms."""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from wav import left  # noqa: E402


def main():
    a, b = left(sys.argv[1]), left(sys.argv[2])
    st, ln, mx = int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5])

    def r(s):
        return math.sqrt(math.fsum((a[t] - b[t + s]) ** 2 for t in range(st, st + ln)) / ln)

    sig = math.sqrt(math.fsum(a[t] ** 2 for t in range(st, st + ln)) / ln)
    best = min((r(s), s) for s in range(-mx, mx + 1))
    print("signal rms %.1f, rms diff at 0: %.1f, best shift %d: rms %.1f" % (sig, r(0), best[1], best[0]))


if __name__ == "__main__":
    main()

#!/usr/bin/python3
"""Compare two 16-bit PCM renders (WAV, or FLAC through the `flac` command).

    rbcmp.py A B [--tolerance LSB] [--bpm BPM] [--align N] [--json]

Reports: equality, lengths, the first differing frame (index, time, bar/beat
at --bpm, 4/4, 16 sixteenths per bar), per-channel max |difference|, number
of differing samples, RMS difference, the histogram of |d| (1, 2..3, 4..15,
16+), and with --align N the constant frame offset k in [-N, N] that best
matches A[i] to B[i+k] (k > 0: B is A delayed by k frames).  Differences are
B - A over the common length.

Exit status: 0 when the files are equal or every |d| <= --tolerance (and the
lengths and formats agree), 1 otherwise, 2 on a read error.  Stdlib only.
"""
import argparse
import array
import json
import math
import struct
import subprocess
import sys

BUCKETS = ["1", "2..3", "4..15", "16+"]


def _bucket(d):
    return 0 if d == 1 else 1 if d < 4 else 2 if d < 16 else 3


def parse_wav(data):
    if data[:4] != b"RIFF" or data[8:12] != b"WAVE":
        raise ValueError("not a RIFF/WAVE file")
    pos, fmt, pcm = 12, None, None
    while pos + 8 <= len(data):
        cid, size = data[pos:pos + 4], struct.unpack_from("<I", data, pos + 4)[0]
        body = data[pos + 8:pos + 8 + size]
        if cid == b"fmt ":
            if len(body) < 16:
                raise ValueError("truncated fmt chunk")
            tag, ch, rate, _, _, bits = struct.unpack_from("<HHIIHH", body)
            if ch == 0:
                raise ValueError("zero channels")
            fmt = (tag, ch, rate, bits)
        elif cid == b"data":
            # streamed WAVs may carry a bogus size
            pcm = data[pos + 8:] if size == 0xFFFFFFFF or size > len(data) - pos - 8 else body
            break
        pos += 8 + size + (size & 1)
    if fmt is None or pcm is None:
        raise ValueError("missing fmt or data chunk")
    tag, ch, rate, bits = fmt
    if tag not in (1, 0xFFFE) or bits != 16:
        raise ValueError("only 16-bit PCM is supported (tag %d, %d bits)" % (tag, bits))
    pcm = pcm[:len(pcm) // (2 * ch) * 2 * ch]
    s = array.array("h")
    s.frombytes(pcm)
    if sys.byteorder == "big":
        s.byteswap()
    return {"channels": ch, "rate": rate, "samples": s}


def load(path):
    with open(path, "rb") as f:
        data = f.read()
    if data[:4] == b"fLaC":
        try:
            data = subprocess.run(["flac", "-d", "-c", "-s", path], check=True,
                                  stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout
        except FileNotFoundError:
            raise ValueError("%s is FLAC but the `flac` command is not installed" % path)
        except subprocess.CalledProcessError as e:
            raise ValueError("flac failed on %s: %s" % (path, e.stderr.decode(errors="replace")))
    return parse_wav(data)


def position(frame, rate, bpm):
    per16 = rate * 60.0 / bpm / 4.0
    n = int(frame / per16)
    return {"frame": frame, "time": frame / rate, "bar": n // 16 + 1,
            "beat": n % 16 // 4 + 1, "sixteenth": n % 4 + 1}


def diff_stats(a, b, ch, n_frames, rate, bpm):
    """Compare a and b (interleaved) over n_frames frames."""
    n = n_frames * ch
    first = None
    maxd = [0] * ch
    count = 0
    sq = 0
    hist = [0] * 4
    if a[:n] != b[:n]:
        for i in range(n):
            d = b[i] - a[i]
            if d:
                if d < 0:
                    d = -d
                if first is None:
                    first = i // ch
                c = i % ch
                if d > maxd[c]:
                    maxd[c] = d
                count += 1
                sq += d * d
                hist[_bucket(d)] += 1
    return {
        "frames_compared": n_frames,
        "first_diff": None if first is None else position(first, rate, bpm),
        "max_abs_diff": maxd,
        "differing_samples": count,
        "rms_diff": math.sqrt(sq / n) if n else 0.0,
        "histogram": dict(zip(BUCKETS, hist)),
    }


def best_offset(a, b, ch, n_align, window=32768):
    """k in [-n_align, n_align] minimizing mismatches of a[i] vs b[i+k]."""
    na, nb = len(a) // ch, len(b) // ch
    start = 0
    for i in range(len(a)):
        if a[i]:
            start = i // ch
            break
    start = max(start, n_align)
    end = min(na, start + window, nb - n_align)
    if end <= start:
        return None
    ref = a[start * ch:end * ch]
    best = None
    # try 0 first so ties prefer no shift
    for k in sorted(range(-n_align, n_align + 1), key=abs):
        cand = b[(start + k) * ch:(end + k) * ch]
        if cand == ref:
            return {"offset": k, "window_mismatches": 0, "window_frames": end - start}
        bad = 0
        limit = best[0] if best else len(ref) + 1
        for j in range(0, len(ref), ch):  # count frames, not samples
            if ref[j:j + ch] != cand[j:j + ch]:
                bad += 1
                if bad >= limit:
                    break
        if best is None or bad < best[0]:
            best = (bad, k)
    return {"offset": best[1], "window_mismatches": best[0], "window_frames": end - start}


def compare(pa, pb, bpm=120.0, align=0, tolerance=0):
    res = {"a": {"channels": pa["channels"], "rate": pa["rate"],
                 "frames": len(pa["samples"]) // pa["channels"]},
           "b": {"channels": pb["channels"], "rate": pb["rate"],
                 "frames": len(pb["samples"]) // pb["channels"]}}
    if pa["channels"] != pb["channels"] or pa["rate"] != pb["rate"]:
        res.update(equal=False, within_tolerance=False, error="format mismatch")
        return res
    ch, rate = pa["channels"], pa["rate"]
    a, b = pa["samples"], pb["samples"]
    n = min(res["a"]["frames"], res["b"]["frames"])
    res["length_equal"] = res["a"]["frames"] == res["b"]["frames"]
    res.update(diff_stats(a, b, ch, n, rate, bpm))
    res["equal"] = res["length_equal"] and res["differing_samples"] == 0
    res["within_tolerance"] = res["length_equal"] and max(res["max_abs_diff"]) <= tolerance
    if align:
        al = best_offset(a, b, ch, align)
        res["align"] = al
        if al is not None and al["offset"]:
            k = al["offset"]
            sa, sb = (a[-k * ch:], b) if k < 0 else (a, b[k * ch:])
            m = min(len(sa), len(sb)) // ch
            res["align"]["stats"] = diff_stats(sa, sb, ch, m, rate, bpm)
    return res


def _fmt_pos(p):
    return "frame %d (%.6f s, bar %d beat %d sixteenth %d)" % (
        p["frame"], p["time"], p["bar"], p["beat"], p["sixteenth"])


def report(r):
    out = []
    out.append("A: %(frames)d frames, %(channels)d ch, %(rate)d Hz" % r["a"])
    out.append("B: %(frames)d frames, %(channels)d ch, %(rate)d Hz" % r["b"])
    if "error" in r:
        out.append(r["error"])
        return "\n".join(out)
    if not r["length_equal"]:
        out.append("LENGTHS DIFFER by %d frames" % (r["b"]["frames"] - r["a"]["frames"]))
    out.append("EQUAL" if r["equal"] else "DIFFERENT")
    if r["differing_samples"]:
        out.append("first difference: " + _fmt_pos(r["first_diff"]))
        out.append("max |d| per channel: %s" % r["max_abs_diff"])
        out.append("differing samples: %d, RMS %.4f" % (r["differing_samples"], r["rms_diff"]))
        out.append("histogram: " + ", ".join("%s: %d" % kv for kv in r["histogram"].items()))
    if "align" in r:
        al = r["align"]
        if al is None:
            out.append("align: too short")
        else:
            out.append("best offset: %d frames (%d/%d window mismatches)" % (
                al["offset"], al["window_mismatches"], al["window_frames"]))
            if "stats" in al:
                s = al["stats"]
                out.append("  at that offset: %d differing samples, max %s, RMS %.4f" % (
                    s["differing_samples"], s["max_abs_diff"], s["rms_diff"]))
    return "\n".join(out)


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("a")
    ap.add_argument("b")
    ap.add_argument("--tolerance", type=int, default=0, help="max |d| in LSB accepted (default 0)")
    ap.add_argument("--bpm", type=float, default=120.0, help="tempo for bar/beat (default 120)")
    ap.add_argument("--align", type=int, default=0, metavar="N", help="search offset within +-N frames")
    ap.add_argument("--json", action="store_true")
    args = ap.parse_args(argv)
    try:
        r = compare(load(args.a), load(args.b), args.bpm, args.align, args.tolerance)
    except (OSError, ValueError) as e:
        print("rbcmp: %s" % e, file=sys.stderr)
        return 2
    print(json.dumps(r, indent=2) if args.json else report(r))
    return 0 if r["equal"] or r["within_tolerance"] else 1


if __name__ == "__main__":
    sys.exit(main())

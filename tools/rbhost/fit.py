#!/usr/bin/python3
"""Fit the idle-block count N of an app export with rbhost (docs/export_state.md sections 11-13).

    tools/rbhost/fit.py [options] SONG.rbs... --app-dir DIR

DIR holds what tools/rb_export.py wrote for each song: <stem>.wav and <stem>.json. For each song the
tool renders the song with rbhost at candidate N (multiples of 16) until a render has the same
SHA-256 as the app's WAV (the hash is compared on the machine: no WAV is downloaded), and prints the
fitted N. Compared with `run.py --idle-sweep A:B:16 --match` it is faster in four ways:

- seeded: the candidates start at N_t + OFFSET and go outward, N_t = round((export started - song
  opened) * 44100/64) from the sidecar's timeline (docs/export_state.md section 13: N - N_t was
  47.6 +- 14.6 over 142 songs). After a song is fitted the offset is the median of the offsets found
  so far (--offset is the start value);
- parallel: JOBS rbhost processes at a time (--jobs, default 2), each holding one permit of the
  semaphore `redc-win-run`; fewer if fewer are free, at least one;
- batched: rbhost.exe and all the songs' message files are uploaded once; one Rebirth.exe check;
- no downloads: hashes are compared on the machine.

The reported N is the FIRST match, in the order tried: it is proven unique only by --all, which
renders every candidate of the window (--max-candidates, default 200) and lists all the matching N.
Options: --range A:B:S replaces the seeded order by
range(A, B, S) (as run.py --idle-sweep); --max-candidates caps the seeded order. rbhost's own options
(--dll, --post-stop-blocks, --drum-counters, --startup-sound, --first-chunk-drain, --no-untitled,
--mod/--rb20ful/--mod-id) are passed through as in run.py. A song with no match exits 1. An app
export that is all zeros (a silent song, or the PCF wave-127 songs of docs/export_state.md section
13) matches at every N: one render at the seed is checked, it is reported as "all N" and its offset
is left out of the learned median.
Result: OUTDIR/fit.json (one entry per song, with the phase timings) and a table on stdout.
"""
import argparse
import json
import os
import statistics
import sys
import threading
import time

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import run  # noqa: E402
rb_export = run.rb_export
rbs, rbsmsg, rb1x = run.rbs, run.rbsmsg, run.rb1x
Failure = rb_export.Failure
log = rb_export.log

BLOCK = 64
RATE = 44100


class SomePermits(rb_export.Permits):
    """Up to `want` permits of redc-win-run (as many as are free, at least one): rbhost is headless."""

    def __init__(self, owner, want):
        super().__init__(owner)
        self.want = want

    def acquire(self, timeout=3600, lease=1800):
        rc, out = self._locky("list")
        sem = json.loads(out)["semaphores"][rb_export.SEMAPHORE]
        k = max(1, min(self.want, sem["available"]))
        while k >= 1:
            wait = timeout if k == 1 else 0
            log("acquiring %d permit(s) of %s as %s" % (k, rb_export.SEMAPHORE, self.owner))
            rc, out = self._locky("acquire", rb_export.SEMAPHORE, str(k), "--owner", self.owner,
                                  "--acquire-timeout", str(wait), "--lease-timeout", str(lease))
            if rc == 0:
                self.held, self.lease, self.n, self.last = True, lease, k, time.time()
                return
            k -= 1
        raise Failure("could not acquire a permit: " + out.strip())


def timeline_n(sidecar):
    """N_t of an app export's sidecar, or None without the two timeline steps."""
    t = {s["step"]: s["t"] for s in sidecar["timing"]["timeline"]}
    if "song opened" not in t or "export started" not in t:
        return None
    return round((t["export started"] - t["song opened"]) * RATE / BLOCK)


def candidate_order(center, limit):
    """Multiples of 16 around `center`, nearest first, never below 0, at most `limit` of them."""
    c = int(round(center / 16.0)) * 16
    out = [c] if c >= 0 else []
    d = 16
    while len(out) < limit and d <= 16 * limit:
        out += [n for n in (c + d, c - d) if n >= 0]
        d += 16
    return out[:limit]


def next_offset(offsets, default):
    """The N - N_t to seed the next song with: the median of the fits so far, else `default`."""
    return statistics.median(offsets) if offsets else default


def is_silent_wav(path):
    """True when the WAV file at `path` has only zero bytes after its 44-byte header."""
    if not os.path.exists(path):
        return False
    with open(path, "rb") as f:
        f.seek(rb_export.WAV_HEADER)
        return all(not any(chunk) for chunk in iter(lambda: f.read(1 << 20), b""))


def retry(f, *a):
    """A read-only or idempotent rcs call (proc, output, hash), repeated after a network error (status
    0: e.g. a failed name lookup; seen once in 100+ requests); an error answer of the server is not
    retried. Starting a process (exec) is not idempotent and is never retried."""
    for attempt in range(4):
        try:
            return f(*a)
        except rb_export.rcs_client.RcsError as e:
            if e.status != 0 or attempt == 3:
                raise
            time.sleep(0.5 * (attempt + 1))


class Fitter:
    def __init__(self, args, permits, exe_sha):
        self.args, self.permits, self.exe_sha = args, permits, exe_sha
        self.new_client = rb_export.rcs_client.Client.from_config

    def render_hash(self, client, wid, rbhm, idle, extra, frames):
        """Renders at `idle` blocks on the machine; returns (sha256 of the WAV, seconds)."""
        a = self.args
        wav = "%s_w%d.wav" % (a.tag, wid)
        rargs = run.rbhost_options(a, idle) + extra
        if self.permits.n > 1:   # processes side by side need their own DLL ports
            rargs.append("--private-port")
        rargs += [run.REMOTE_WIN + "\\" + rbhm, run.REMOTE_WIN + "\\" + wav]
        t0 = time.time()
        rec = client.exec(run.REMOTE_WIN + "\\rbhost.exe", rargs, cwd=run.REMOTE_WIN)
        pid = rec["id"]
        while True:
            r = retry(client.proc, pid)
            if not r["running"]:
                break
            if time.time() - t0 > a.timeout:
                client.kill(pid)
                raise Failure("rbhost timed out")
            time.sleep(0.03)
        if r["exit_code"] != 0:
            out, _ = retry(client.output, pid)
            raise Failure("rbhost exited with %s: %s" % (r["exit_code"], out.decode("cp437", "replace")[-300:]))
        h = retry(client.hash, "%s/%s" % (run.REMOTE_DIR, wav))["hash"]
        return h, time.time() - t0

    def fit_song(self, rbhm, order, want_sha, extra, frames):
        """Tries the candidates of `order` with the workers; returns (sorted matching N, [(N, seconds)])."""
        a = self.args
        lock = threading.Lock()
        state = {"next": 0, "matches": [], "tried": [], "error": None}

        def worker(wid):
            client = self.new_client()
            while True:
                with lock:
                    if state["error"] or (state["matches"] and not a.all) or state["next"] >= len(order):
                        return
                    n = order[state["next"]]
                    state["next"] += 1
                try:
                    h, secs = self.render_hash(client, wid, rbhm, n, extra, frames)
                except Exception as e:
                    with lock:
                        state["error"] = e
                    return
                with lock:
                    state["tried"].append((n, secs))
                    if h == want_sha:
                        state["matches"].append(n)

        nthreads = self.permits.n
        threads = [threading.Thread(target=worker, args=(i,)) for i in range(nthreads)]
        for t in threads:
            t.start()
        while any(t.is_alive() for t in threads):
            if time.time() - self.permits.last > 600:
                self.permits.refresh()
            time.sleep(0.2)
        for t in threads:
            t.join()
        if state["error"]:
            raise state["error"]
        return sorted(state["matches"]), state["tried"]


def fit_songs(args, fitter, jobs, extra, results):
    """Fits each job (stem, remote message, frames, app sidecar, app export is silent) in turn,
    appending a dict per song to `results` (which is returned)."""
    offsets = []
    for stem, rbhm, frames, sc, silent in jobs:
        want = sc["wav_sha256"]
        nt = timeline_n(sc)
        off = next_offset(offsets, args.offset)
        if args.range:
            a, b, s = (int(x) for x in args.range.split(":"))
            order = list(range(a, b, s))
        elif nt is None:
            raise Failure("%s: the sidecar has no 'song opened'/'export started' steps; use --range" % stem)
        else:
            order = candidate_order(nt + off, args.max_candidates)
        if silent:   # matches at every N: one render at the seed shows that rbhost is silent too
            order = order[:1]
        t0 = time.time()
        matches, tried = fitter.fit_song(rbhm, order, want, extra, frames)
        secs = time.time() - t0
        ent = {"song": stem, "app_sha256": want, "N_t": nt, "offset_used": off,
               "candidates_tried": len(tried), "matches": [] if silent else matches, "all_n": bool(silent and matches),
               "seconds": round(secs, 2),
               "seconds_per_render": round(sum(x[1] for x in tried) / max(1, len(tried)), 3),
               "workers": fitter.permits.n}
        if matches and not silent and nt is not None and len(matches) == 1:
            offsets.append(matches[0] - nt)
            ent["N_minus_N_t"] = matches[0] - nt
        results.append(ent)
        log("%s: N %s (N_t %s), %d render(s) in %.1f s" % (
            stem, "all N" if ent["all_n"] else (matches or "NONE"), nt, len(tried), secs))
    return results


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("songs", nargs="+", metavar="SONG.rbs")
    ap.add_argument("--app-dir", required=True, help="where rb_export.py wrote <stem>.wav and <stem>.json")
    ap.add_argument("-o", "--outdir", default=".")
    ap.add_argument("--owner", help="locky owner (default: the git branch)")
    ap.add_argument("--tag", default="fit", help="prefix of the remote file names (default fit)")
    ap.add_argument("--remote-dir", metavar="NAME", help="work in C:\\re\\rb\\NAME (default rbhost)")
    ap.add_argument("--jobs", type=int, default=2, help="parallel rbhost processes, one permit each (default 2)")
    ap.add_argument("--offset", type=float, default=48.0, help="start value of N - N_t (default 48)")
    ap.add_argument("--max-candidates", type=int, default=200)
    ap.add_argument("--range", metavar="A:B:S", help="candidates range(A, B, S) instead of the seeded order")
    ap.add_argument("--all", action="store_true", help="render every candidate; list all matching N")
    ap.add_argument("--timeout", type=float, default=600.0)
    ap.add_argument("--dll")
    ap.add_argument("--post-stop-blocks", type=int)
    ap.add_argument("--drum-counters", metavar="A,B,C,D")
    ap.add_argument("--idle-fpu-cw", metavar="HEX")
    ap.add_argument("--startup-sound", action="store_true")
    ap.add_argument("--first-chunk-drain", action="store_true")
    ap.add_argument("--no-untitled", action="store_true")
    ap.add_argument("--mod", metavar="MOD.rbm")
    ap.add_argument("--rb20ful", metavar="RB20FUL.DAT")
    ap.add_argument("--mod-id", type=int, default=412)
    ap.add_argument("-v", action="store_true")
    args = ap.parse_args(argv)
    if args.remote_dir:
        run.set_remote_dir(args.remote_dir)
    outdir = os.path.abspath(args.outdir)
    os.makedirs(outdir, exist_ok=True)

    t_all = time.time()
    exe = os.path.join(HERE, "build", "rbhost.exe")
    run.build(exe)
    exe_sha = rb_export.sha256_file(exe)
    client = rb_export.rcs_client.Client.from_config()
    permits = SomePermits(args.owner or rb_export.default_owner(), args.jobs)
    results, rc = [], 1
    try:
        permits.acquire()
        t_up = time.time()
        client.upload_if_changed(exe, run.REMOTE_DIR + "/rbhost.exe")
        jobs = []
        for i, song_path in enumerate(args.songs):
            song = rb1x.read_any(song_path)   # an RBS or a 1.x song
            if args.mod:   # --mod applies to every song of the batch: each must name that mod
                run.check_mod_name(args.mod, song.glob.mod_name)
            stem = os.path.splitext(os.path.basename(song_path))[0]
            rbhm = os.path.join(outdir, stem + ".rbhm")
            with open(rbhm, "wb") as f:
                f.write(rbsmsg.message_file(song, args.mod_id if args.mod else None))
            name = "%s_%d.rbhm" % (args.tag, i)
            client.upload(rbhm, "%s/%s" % (run.REMOTE_DIR, name))
            side = os.path.join(args.app_dir, stem + ".json")
            with open(side) as f:
                sc = json.load(f)
            jobs.append((stem, name, rb_export.export_frames(song), sc,
                         is_silent_wav(os.path.join(args.app_dir, stem + ".wav"))))
        extra = []
        if args.mod:
            extra = ["--mod-msg", run.upload_mod(argparse.Namespace(
                mod=args.mod, rb20ful=args.rb20ful, tag=args.tag, mod_id=args.mod_id), client, outdir,
                rb1x.read_any(args.songs[0]).glob.mod_name)]   # every song was checked above
        app_pids = rb_export.Machine(client, permits).rebirth_pids()
        if app_pids:
            raise Failure("Rebirth.exe is running on the machine (PIDs %s); rbhost runs without it" % app_pids)
        t_upload = time.time() - t_up
        log("uploaded rbhost and %d song(s) in %.2f s; %d worker(s)" % (len(jobs), t_upload, permits.n))
        results = fit_songs(args, Fitter(args, permits, exe_sha), jobs, extra, results)
        rc = 1 if any(not (e["matches"] or e.get("all_n")) for e in results) else 0
    except Exception as e:   # also an unexpected one: fit.json below keeps what was fitted
        log("FAILED: %s: %s" % (type(e).__name__, e))
    finally:
        permits.release()
    total = time.time() - t_all
    with open(os.path.join(outdir, "fit.json"), "w") as f:
        json.dump({"rbhost_sha256": exe_sha, "total_seconds": round(total, 2), "songs": results}, f, indent=1)
        f.write("\n")
    for e in results:
        print("%-28s N=%-14s N_t=%-6s N-N_t=%-5s renders=%-3d %.1f s" % (
            e["song"], "all N" if e.get("all_n") else
            ",".join(map(str, e["matches"][:3])) + ("..." if len(e["matches"]) > 3 else ""),
            e["N_t"], e.get("N_minus_N_t", "-"), e["candidates_tried"], e["seconds"]))
    log("total %.1f s" % total)
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

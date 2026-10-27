#!/usr/bin/python3
"""Render songs with the original ReBirth.dll through rbhost on the test machine.

    tools/rbhost/run.py [options] SONG.rbs... -o OUTDIR

For each song: convert it to the engine's load messages (rbsmsg.py), and on the machine run
rbhost.exe (built here with build.sh, uploaded to C:\\re\\rb\\rbhost) to render the export loop;
download OUTDIR/<stem>.wav (or <stem>.<run>.wav with --runs), the dumps, and a JSON sidecar with the
options, rbhost's log, the hashes and the RIFF check of tools/rb_export.py.

rbhost is headless: it takes neither the keyboard nor the screen, so the runner holds ONE permit of
the locky semaphore `redc-win-run` (owner: --owner, default the git branch) while it uses the
machine, and stops only the processes it started. Before every run it lists Rebirth.exe processes
(tasklist, as tools/rb_export.py does) and refuses to run if one exists; the sidecar records the
(empty) list as "rebirth_exe_pids_before".

rbhost options passed through: --idle-blocks N, --post-stop-blocks M, --drum-counters A,B,C,D,
--startup-sound, --first-chunk-drain, --dll PATH (remote), --dump-tables, --dump-state,
--dump-float (download into OUTDIR/<stem>.tables/, OUTDIR/<stem>.state/ and
OUTDIR/<stem>.float/), -v.

Mods: --mod MOD.rbm --rb20ful RB20FUL.DAT [--mod-id N] uploads the mod to C:\\re\\rb\\rbhost\\<tag>.rbm
with its message 55 (tools/rbm.py, key table from the local RB20FUL.DAT) and has rbhost load it
instead of the built-in mod; --dump-samples downloads the DLL's loaded samples of the mod into
OUTDIR/<stem>.samples/ (docs/rbm.md). The mod, the DAT and the dumps are Propellerhead material:
keep OUTDIR out of the repository.
"""
import argparse
import json
import os
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, os.path.join(ROOT, "tools"))
sys.path.insert(0, HERE)
import rb_export  # noqa: E402
import rbm  # noqa: E402
import rbs  # noqa: E402
import rbsmsg  # noqa: E402

REMOTE_DIR = "rb/rbhost"              # under the rcs root C:\re
REMOTE_WIN = "C:\\re\\rb\\rbhost"
log = rb_export.log
Failure = rb_export.Failure


def set_remote_dir(name):
    """--remote-dir NAME: work in C:\\re\\rb\\NAME instead of C:\\re\\rb\\rbhost (rbhost.exe, the
    messages, the mods and the renders all go there; delete the folder when done)."""
    global REMOTE_DIR, REMOTE_WIN
    if not name or any(c in name for c in "/\\:") or name in (".", ".."):
        raise SystemExit("--remote-dir: a plain folder name under C:\\re\\rb, got %r" % name)
    REMOTE_DIR, REMOTE_WIN = "rb/" + name, "C:\\re\\rb\\" + name


class OnePermit(rb_export.Permits):
    """One permit of redc-win-run: rbhost does not use the desktop."""

    def acquire(self, timeout=3600, lease=1800):
        log("acquiring 1 permit of %s as %s" % (rb_export.SEMAPHORE, self.owner))
        rc, out = self._locky("acquire", rb_export.SEMAPHORE, "1", "--owner", self.owner,
                              "--acquire-timeout", str(timeout), "--lease-timeout", str(lease))
        if rc != 0:
            raise Failure("could not acquire a permit: " + out.strip())
        self.held, self.lease, self.n, self.last = True, lease, 1, time.time()


def build(exe):
    subprocess.run(["sh", os.path.join(HERE, "build.sh"), exe], check=True, stdout=subprocess.DEVNULL)


def run_remote(client, permits, args, timeout):
    rec = client.exec(REMOTE_WIN + "\\rbhost.exe", args, cwd=REMOTE_WIN)
    pid = rec["id"]
    t0 = time.time()
    while True:
        r = client.proc(pid)
        if not r["running"]:
            break
        if time.time() - t0 > timeout:
            client.kill(pid)
            raise Failure("rbhost timed out")
        if time.time() - permits.last > 600:
            permits.refresh()
        time.sleep(0.2)
    out, _ = client.output(pid)
    return r["exit_code"], out.decode("cp437", "replace"), time.time() - t0


def download_dir(client, remote, local):
    os.makedirs(local, exist_ok=True)
    for e in client.list(remote):
        name = e["name"] if isinstance(e, dict) else e
        client.download(remote + "/" + name, os.path.join(local, name))


def check_mod_name(path, song_mod):
    """Read the mod at PATH; refuse it when the name in its INFO chunk is not exactly SONG_MOD, the mod
    the song names (None: no check). Returns the parsed mod."""
    mod = rbm.read(path)
    name = mod.info.name.decode("latin-1") if mod.info else None
    if song_mod is not None and name != song_mod:
        raise Failure("the song names the mod %r, %s is %r" % (song_mod, path, name))
    return mod


def upload_mod(args, client, outdir, song_mod=None):
    """Upload the mod to C:\\re\\rb\\rbhost\\<tag>.rbm and its message 55 (tools/rbm.py, with the key
    table from --rb20ful) next to it; return the message's remote path for rbhost --mod-msg."""
    if not args.rb20ful:
        raise Failure("--mod needs --rb20ful RB20FUL.DAT")
    mod = check_mod_name(args.mod, song_mod)
    msg = os.path.join(outdir, args.tag + ".rbmmsg")
    with open(msg, "wb") as f:
        f.write(rbm.mod_message(mod, rbm.key_table(mod, args.rb20ful), REMOTE_WIN + "\\%s.rbm" % args.tag,
                                args.mod_id))
    client.upload_if_changed(args.mod, "%s/%s.rbm" % (REMOTE_DIR, args.tag))
    client.upload(msg, "%s/%s.rbmmsg" % (REMOTE_DIR, args.tag))
    return REMOTE_WIN + "\\%s.rbmmsg" % args.tag


def rbhost_options(args, idle):
    """rbhost's command line options (without the dump options and the two file names) for one render."""
    rargs = []
    if args.dll:
        rargs += ["--dll", args.dll]
    if idle is not None:
        rargs += ["--idle-blocks", str(idle)]
    for opt in ("post_stop_blocks", "drum_counters", "idle_fpu_cw"):
        v = getattr(args, opt)
        if v is not None:
            rargs += ["--" + opt.replace("_", "-"), str(v)]
    for opt in ("startup_sound", "first_chunk_drain", "no_untitled"):
        if getattr(args, opt):
            rargs.append("--" + opt.replace("_", "-"))
    if args.v:
        rargs.append("-v")
    return rargs


def render(args, client, permits, outdir, exe_sha):
    results = []
    for song_path in args.songs:
        song = rbs.read_song(song_path)
        stem = os.path.splitext(os.path.basename(song_path))[0]
        rbhm = os.path.join(outdir, stem + ".rbhm")
        with open(rbhm, "wb") as f:
            f.write(rbsmsg.message_file(song, args.mod_id if args.mod else None))
        remote_rbhm = "%s/%s.rbhm" % (REMOTE_DIR, args.tag)
        client.upload(rbhm, remote_rbhm)
        frames = rb_export.export_frames(song)
        if args.idle_sweep:
            a, b, step = (int(x) for x in args.idle_sweep.split(":"))
            jobs = [("%s.idle%d" % (stem, n), n) for n in range(a, b, step)]
        else:
            jobs = [(stem if args.runs == 1 else "%s.%d" % (stem, run + 1), args.idle_blocks)
                    for run in range(args.runs)]
        ref = None
        if args.match:
            with open(args.match, "rb") as f:
                ref = f.read()[rb_export.WAV_HEADER:]
            ref_sha = rb_export.sha256_file(args.match)
        mod_msg = upload_mod(args, client, outdir, song.glob.mod_name) if args.mod else None   # once per song, not per render
        app_pids = None
        for name, idle in jobs:
            rargs = rbhost_options(args, idle)
            if mod_msg:
                rargs += ["--mod-msg", mod_msg]
            dumps = {}
            for opt in ("tables", "state", "float", "samples"):
                if getattr(args, "dump_" + opt):
                    d = "%s_%s" % (args.tag, opt)
                    try:
                        client.delete("%s/%s" % (REMOTE_DIR, d), recursive=True)
                    except Exception:
                        pass
                    rargs += ["--dump-" + opt, REMOTE_WIN + "\\" + d]
                    dumps[opt] = d
            remote_wav = "%s/%s.wav" % (REMOTE_DIR, args.tag)
            rargs += [REMOTE_WIN + "\\%s.rbhm" % args.tag, REMOTE_WIN + "\\%s.wav" % args.tag]
            if app_pids is None:   # rbhost never starts the app: one check per song suffices
                app_pids = rb_export.Machine(client, permits).rebirth_pids()
                if app_pids:
                    raise Failure("Rebirth.exe is running on the machine (PIDs %s); rbhost runs without it"
                                  % app_pids)
            log("%s: no Rebirth.exe running; rbhost %s" % (name, " ".join(rargs)))
            code, out, secs = run_remote(client, permits, rargs, args.timeout)
            sys.stderr.write(out)
            if code != 0:
                raise Failure("rbhost exited with %s" % code)
            wav = os.path.join(outdir, name + ".wav")
            if ref is not None and args.match_hash:
                # equality only: compare the hashes on the machine, download the WAV only on a match
                if client.hash(remote_wav)["hash"] != ref_sha:
                    log("%s: %d frames, sha256 differs from %s (not downloaded)" % (name, frames, args.match))
                    results.append({"name": name, "match": {"identical": False}})
                    continue
            client.download(remote_wav, wav)
            check = rb_export.check_wav(wav, frames)
            for opt, d in dumps.items():
                download_dir(client, "%s/%s" % (REMOTE_DIR, d), os.path.join(outdir, "%s.%s" % (name, opt)))
            side = {"tool": "rbhost", "song": os.path.abspath(song_path),
                    "song_sha256": rb_export.sha256_file(song_path), "rbhost_sha256": exe_sha,
                    "args": rargs, "rebirth_exe_pids_before": app_pids, "log": out, "seconds": round(secs, 3), "frames": frames,
                    "wav_sha256": rb_export.sha256_file(wav), "riff": check}
            if ref is not None:
                with open(wav, "rb") as f:
                    data = f.read()[rb_export.WAV_HEADER:]
                diff = next((i for i in range(0, min(len(data), len(ref)), 2) if data[i:i + 2] != ref[i:i + 2]),
                            None)
                side["match"] = {"ref": os.path.abspath(args.match), "identical": data == ref,
                                 "first_diff_frame": None if diff is None else diff // 4}
            rb_export.sidecar_write(os.path.join(outdir, name + ".json"), side)
            log("%s: %d frames, sha256 %s%s" % (name, frames, side["wav_sha256"],
                                               "" if ref is None else ", match %s" % side["match"]))
            results.append(side)
            if ref is not None and side["match"]["identical"] and args.idle_sweep:
                log("%s is identical to %s" % (name, args.match))
                break
    return results


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("songs", nargs="+", metavar="SONG.rbs")
    ap.add_argument("-o", "--outdir", default=".")
    ap.add_argument("--owner", help="locky owner (default: the git branch)")
    ap.add_argument("--tag", default="x", help="prefix of the remote file names (default x)")
    ap.add_argument("--remote-dir", metavar="NAME", help="work in C:\\re\\rb\\NAME (default rbhost)")
    ap.add_argument("--runs", type=int, default=1, help="render each song this many times")
    ap.add_argument("--timeout", type=float, default=600.0)
    ap.add_argument("--dll", help="remote ReBirth.dll (default: rbhost's, C:\\ReBirth RB-338\\ReBirth.dll)")
    ap.add_argument("--idle-blocks", type=int)
    ap.add_argument("--idle-sweep", metavar="START:STOP:STEP",
                    help="render once per --idle-blocks value in range(START, STOP, STEP); stops at a --match")
    ap.add_argument("--match", metavar="REF.wav", help="compare each render's samples with REF.wav")
    ap.add_argument("--match-hash", action="store_true",
                    help="with --match: compare the whole-file hashes on the machine; download only a match")
    ap.add_argument("--post-stop-blocks", type=int)
    ap.add_argument("--drum-counters", metavar="A,B,C,D")
    ap.add_argument("--idle-fpu-cw", metavar="HEX", help="experiment: x87 control word for the idle blocks")
    ap.add_argument("--startup-sound", action="store_true")
    ap.add_argument("--first-chunk-drain", action="store_true")
    ap.add_argument("--no-untitled", action="store_true", help="no untitled song before the song")
    ap.add_argument("--dump-tables", action="store_true")
    ap.add_argument("--dump-state", action="store_true")
    ap.add_argument("--dump-float", action="store_true", help="the export's float output, before int16")
    ap.add_argument("--mod", metavar="MOD.rbm", help="load this mod (message 55) instead of the built-in one")
    ap.add_argument("--rb20ful", metavar="RB20FUL.DAT", help="the CD's key file, for --mod (local path)")
    ap.add_argument("--mod-id", type=int, default=412, help="the mod id of --mod (default 412)")
    ap.add_argument("--dump-samples", action="store_true", help="the mod's loaded samples (the DLL's table)")
    ap.add_argument("-v", action="store_true")
    args = ap.parse_args(argv)
    if args.remote_dir:
        set_remote_dir(args.remote_dir)
    outdir = os.path.abspath(args.outdir)
    os.makedirs(outdir, exist_ok=True)
    exe = os.path.join(HERE, "build", "rbhost.exe")
    build(exe)
    exe_sha = rb_export.sha256_file(exe)
    client = rb_export.rcs_client.Client.from_config()
    permits = OnePermit(args.owner or rb_export.default_owner())
    rc = 1
    try:
        permits.acquire()
        client.upload_if_changed(exe, REMOTE_DIR + "/rbhost.exe")
        render(args, client, permits, outdir, exe_sha)
        rc = 0
    except Failure as e:
        log("FAILED: %s" % e)
    finally:
        permits.release()
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

#!/usr/bin/python3
"""rbhost against orb render, by the sha256 of the whole WAV, without downloading the renders.

    tools/rbhost/hashcmp.py [--remote-dir NAME] [--idle-blocks N] [--orb PATH] [--json OUT.json]
                            SONG.rbs...

For each song (RBS or ReBirth 1.x; duplicates by content are skipped): the load messages
(rbsmsg.message_file, tools/rb1x.py for a 1.x song) go to the machine, rbhost renders the export
there, the machine hashes the WAV (rcs hash), and `orb render --idle-blocks N` renders the same song
here (into a temporary file, hashed and deleted). Prints one line per song and the count of equal
ones; --json writes [{"song", "song_sha256", "rbhost_sha256", "orb_sha256", "equal"}]. Built-in mod
only (no --mod; orb embeds the built-in kit, task-067). Holds one permit of redc-win-run like
run.py, refuses to run while a Rebirth.exe runs, and deletes its remote x.rbhm and x.wav and
releases the permit also when it fails.
task-060 ran it over the 129 distinct 1.x songs of the user's collection.
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import run  # noqa: E402
rb_export, rbsmsg, rb1x = run.rb_export, run.rbsmsg, run.rb1x

REMOTE_FILES = ("x.wav", "x.rbhm")


def unique_songs(paths):
    """(path, sha256) of each path whose content was not seen before."""
    seen, out = set(), []
    for p in paths:
        h = rb_export.sha256_file(p)
        if h not in seen:
            seen.add(h)
            out.append((p, h))
    return out


def compare(songs, client, permits, idle, render, exe=None, log=print):
    """Render every (path, sha256) of SONGS with rbhost on the machine and with RENDER(path, wav)
    here (returns whether it wrote WAV), compare the hashes. EXE, when given, is uploaded first.
    Returns the result list; raises run.Failure on an rbhost failure, after the cleanup."""
    results = []
    tmp = tempfile.mkdtemp()
    permits.acquire()
    try:
        if exe:
            client.upload_if_changed(exe, run.REMOTE_DIR + "/rbhost.exe")
        if rb_export.Machine(client, permits).rebirth_pids():
            raise run.Failure("Rebirth.exe is running on the machine; rbhost runs without it")
        for p, song_sha in songs:
            rbhm = os.path.join(tmp, "x.rbhm")
            with open(rbhm, "wb") as f:
                f.write(rbsmsg.message_file(rb1x.read_any(p)))
            client.upload(rbhm, run.REMOTE_DIR + "/x.rbhm")
            rargs = ["--idle-blocks", str(idle), run.REMOTE_WIN + "\\x.rbhm", run.REMOTE_WIN + "\\x.wav"]
            code, out, _ = run.run_remote(client, permits, rargs, 900)
            if code != 0:
                raise run.Failure("%s: rbhost exited with %s\n%s" % (p, code, out))
            remote = client.hash(run.REMOTE_DIR + "/x.wav")["hash"]
            wav = os.path.join(tmp, "orb.wav")
            local = rb_export.sha256_file(wav) if render(p, wav) else None
            if os.path.exists(wav):
                os.remove(wav)
            e = {"song": os.path.basename(p), "song_sha256": song_sha, "rbhost_sha256": remote,
                 "orb_sha256": local, "equal": local == remote}
            results.append(e)
            log("%s %s rbhost %s orb %s" % ("EQUAL" if e["equal"] else "DIFF ", e["song"], remote, local))
    finally:
        for name in REMOTE_FILES:
            try:
                client.delete(run.REMOTE_DIR + "/" + name)
            except Exception:   # not there (nothing rendered yet) or the machine is gone
                pass
        permits.release()
        for name in os.listdir(tmp):
            os.remove(os.path.join(tmp, name))
        os.rmdir(tmp)
    return results


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("songs", nargs="+", metavar="SONG.rbs")
    ap.add_argument("--remote-dir", metavar="NAME", help="work in C:\\re\\rb\\NAME (default rbhost)")
    ap.add_argument("--idle-blocks", type=int, default=0, help="for both renders (default 0)")
    ap.add_argument("--orb", default=os.path.join(ROOT, "target", "release", "orb"))
    ap.add_argument("--owner", help="locky owner (default: the git branch)")
    ap.add_argument("--json", metavar="OUT.json")
    args = ap.parse_args(argv)
    if args.remote_dir:
        run.set_remote_dir(args.remote_dir)
    exe = os.path.join(HERE, "build", "rbhost.exe")
    run.build(exe)

    def render(p, wav):
        r = subprocess.run([args.orb, "render", "--idle-blocks", str(args.idle_blocks),
                            "-o", wav, p], capture_output=True)
        return r.returncode == 0

    client = rb_export.rcs_client.Client.from_config()
    permits = run.OnePermit(args.owner or rb_export.default_owner())
    results = compare(unique_songs(args.songs), client, permits, args.idle_blocks, render, exe,
                      log=lambda s: print(s, flush=True))
    if args.json:
        with open(args.json, "w") as f:
            json.dump(results, f, indent=1)
    eq = sum(e["equal"] for e in results)
    print("%d of %d equal" % (eq, len(results)))
    return 0 if eq == len(results) else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

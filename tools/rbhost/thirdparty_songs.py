#!/usr/bin/python3
"""Writes tests/refs/thirdparty_songs.json: rbhost's render hash of every distinct song of a
third-party song folder (task-129), which the `songs` test of crates/orb-cli/tests/thirdparty.rs
requires `orb render` to equal.

    tools/rbhost/thirdparty_songs.py --songs SONGS --mods MODS --rb20ful RB20FUL.DAT \
        [--remote-dir NAME] [--owner OWNER] [-o tests/refs/thirdparty_songs.json]
        [--batch N] [--fresh] [--only PATH...]

SONGS is the folder of songs (ORB_THIRDPARTY_SONGS_DIR: every file under it, recursively, whose
name contains ".rbs", case-insensitive), MODS the folder of the RECRYPTED mods
(ORB_THIRDPARTY_MODS_DIR) and RB20FUL.DAT the CD's key file (`bsdtar -xf CD.iso RB20FUL.DAT`): it
builds each mod's message 55 here, as tools/rbhost/run.py --mod --rb20ful does.

Each distinct song (by SHA-256) is rendered once with rbhost on the test machine, in
C:\\re\\rb\\NAME (--remote-dir, default t129), which is deleted at the end:
- an RBS song (4.0, 4.1 or 4.2): rbsmsg.py's load messages, `--idle-blocks 9088`, and the mod the
  original resolves: the GLOB's mod name looked up by INFO name among the mods of MODS in the
  original's folder order (NTFS: the names upcased per character, then UTF-16 units; the first
  file listed wins, docs/rbm.md 11.3), loaded with its message 55; "Standard ReBirth", or a name
  no mod has, is the built-in kit (what `orb render --standard-kit` plays);
- a ReBirth 1.x song (a MIDI file): tools/rb1x.py's messages, `--idle-blocks 0`, the built-in kit.

The WAV is hashed on the machine (rcs hash) and never downloaded. A song rbhost cannot render, or
that the converters refuse, goes into "skipped" with the reason. The manifest holds hashes and
names only:

    {"songs": [{"file" (the first path), "files" (every path of that content, relative to SONGS,
                "/"-separated), "sha256", "form" ("rbs" or "1x"), "version", "mod_name" (the
                song's), "mod" (the file of MODS, or null for the built-in kit), "idle_blocks",
                "render_sha256"}],
     "skipped": [{"file", "files", "sha256", "reason", and the settings when it got that far}]}

The run is batched and resumable: each machine session renders --batch songs (default 100;
upload rbhost.exe and the session's mods, render, delete the remote folder, release the permit)
and the manifest (-o) is rewritten after every song. A song the existing manifest already records
(rendered or skipped) is not rendered again, so an interrupted run goes on where it stopped; the
paths of the songs are refreshed from the folder. `--fresh` ignores the existing manifest.
`--only PATH...` renders those songs again and keeps the other entries. `--exclude PATH REASON`
lists a file that is not a song file (task-129: a zip holding a song, which the user excluded)
as skipped, "excluded": true, and later runs keep it.
Each session holds one permit of redc-win-run (rbhost takes neither the keyboard nor the
screen), refuses to run while a Rebirth.exe runs, and stops only the processes it started.
"""
import argparse
import hashlib
import json
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
sys.path.insert(0, HERE)
import run  # noqa: E402
rb_export, rbsmsg, rb1x, rbm = run.rb_export, run.rbsmsg, run.rb1x, run.rbm

STANDARD = rbsmsg.STANDARD_MOD
RBS_IDLE = 9088
RB1X_IDLE = 0
MOD_ID = 412
DEFAULT_OUT = os.path.join(ROOT, "tests", "refs", "thirdparty_songs.json")


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def is_song_name(name):
    return ".rbs" in name.lower()


def song_files(folder):
    """The relative paths ("/"-separated, sorted) of the song files under FOLDER, recursively."""
    out = []
    for dirpath, _dirs, names in os.walk(folder):
        for name in names:
            p = os.path.join(dirpath, name)
            if is_song_name(name) and os.path.isfile(p):
                out.append(os.path.relpath(p, folder).replace(os.sep, "/"))
    return sorted(out)


def distinct_songs(folder):
    """[(sha256, [relative paths])] of the song files under FOLDER, each content once, ordered by
    its first path."""
    by_hash = {}
    for rel in song_files(folder):
        with open(os.path.join(folder, rel), "rb") as f:
            by_hash.setdefault(sha256(f.read()), []).append(rel)
    return sorted(by_hash.items(), key=lambda kv: kv[1][0])


def ntfs_key(name):
    """The folder order of the original's FindFirstFile on NTFS (what rbhost, which runs the
    original, sees; orb itself sorts file names by bytes since task-151): each character upcased
    when its uppercase is one character, then the UTF-16 units."""
    up = "".join(c.upper() if len(c.upper()) == 1 else c for c in name)
    return (up.encode("utf-16-be"), name)


def mod_names(folder):
    """[(file, INFO name or None)] of the .rbm files of FOLDER in the original's folder order."""
    out = []
    for f in sorted((f for f in os.listdir(folder) if f.lower().endswith(".rbm")), key=ntfs_key):
        try:
            m = rbm.read(os.path.join(folder, f))
            name = m.info.name.decode("latin-1") if m.info else None
        except Exception:   # a mod that does not parse has no menu entry
            name = None
        out.append((f, name))
    return out


def resolve(mods, mod_name):
    """The file of the first mod listed whose INFO name is MOD_NAME, or None (the built-in kit)."""
    if mod_name == STANDARD:
        return None
    return next((f for f, n in mods if n == mod_name), None)


class Refused(Exception):
    """A file the original does not open as a song."""


def refusal(data):
    """Why the original's RBS reader refuses DATA (not a 1.x MIDI file), or None. sub_434D05
    first calls sub_49F3B4 (0x434D56), which reads the chunk at offset 0 (sub_49F5E2) and is
    true only for a FORM, CAT or LIST (kind 1..3) whose whole length, the 12-byte header plus
    the size less 4 (sub_49F19B), equals the file's; otherwise the load throws (0x434D92)."""
    if len(data) < 8:
        return "shorter than a chunk header"
    tag = data[:4]
    if tag not in (b"FORM", b"CAT ", b"LIST"):
        return "the first chunk %r is not FORM, CAT or LIST" % tag
    total = 8 + int.from_bytes(data[4:8], "big")
    if total != len(data):
        return "the first chunk is %d bytes long, the file %d" % (total, len(data))
    return None


def plan_song(path, mods):
    """(entry without the render hash, the song) or raises rb1x.Error / ValueError."""
    with open(path, "rb") as f:
        data = f.read()
    why = None if rb1x.is_1x(data) else refusal(data)
    if why:
        raise Refused(why)
    song = rb1x.read_any(path)
    if isinstance(song, rb1x.Song1x):
        return {"form": "1x", "version": "%d.%d" % song.version, "mod_name": STANDARD,
                "mod": None, "idle_blocks": RB1X_IDLE}, song
    v = rbsmsg.version_tuple(song)
    name = song.glob.mod_name
    return {"form": "rbs", "version": "%d.%d" % v, "mod_name": name, "mod": resolve(mods, name),
            "idle_blocks": RBS_IDLE}, song


def messages(entry, song):
    """The song's message file: its songs are created with the loaded mod's id, or the built-in
    kit's (411) when the song's mod is Standard or not found."""
    return rbsmsg.message_file(song, MOD_ID if entry["mod"] else rbsmsg.STANDARD_MOD_ID)


class Renderer:
    """rbhost on the machine: uploads rbhost.exe, each mod (and its message 55) once, and each
    song's messages; returns the render's SHA-256 taken on the machine."""

    def __init__(self, client, permits, mods_dir, rb20ful, tmp):
        self.client, self.permits, self.mods_dir, self.rb20ful = client, permits, mods_dir, rb20ful
        self.tmp, self.mod_msgs = tmp, {}

    def mod_msg(self, mod_file):
        if mod_file not in self.mod_msgs:
            i = len(self.mod_msgs)
            local = os.path.join(self.mods_dir, mod_file)
            mod = rbm.read(local)
            msg = os.path.join(self.tmp, "m%d.rbmmsg" % i)
            with open(msg, "wb") as f:
                f.write(rbm.mod_message(mod, rbm.key_table(mod, self.rb20ful),
                                        run.REMOTE_WIN + "\\m%d.rbm" % i, MOD_ID))
            self.client.upload(local, "%s/m%d.rbm" % (run.REMOTE_DIR, i))
            self.client.upload(msg, "%s/m%d.rbmmsg" % (run.REMOTE_DIR, i))
            self.mod_msgs[mod_file] = run.REMOTE_WIN + "\\m%d.rbmmsg" % i
        return self.mod_msgs[mod_file]

    def render(self, entry, song):
        rbhm = os.path.join(self.tmp, "x.rbhm")
        with open(rbhm, "wb") as f:
            f.write(messages(entry, song))
        self.client.upload(rbhm, run.REMOTE_DIR + "/x.rbhm")
        rargs = ["--idle-blocks", str(entry["idle_blocks"])]
        if entry["mod"]:
            rargs += ["--mod-msg", self.mod_msg(entry["mod"])]
        rargs += [run.REMOTE_WIN + "\\x.rbhm", run.REMOTE_WIN + "\\x.wav"]
        try:
            self.client.delete(run.REMOTE_DIR + "/x.wav")
        except Exception:   # not there yet
            pass
        code, out, _ = run.run_remote(self.client, self.permits, rargs, 900)
        if code != 0:
            return None, "rbhost exited with %s: %s" % (code, out.strip()[-300:])
        return self.client.hash(run.REMOTE_DIR + "/x.wav")["hash"], None


def build_manifest(songs_dir, mods, renderer, only=None, previous=None, save=None, limit=None,
                   log=print):
    """The manifest dict. A song PREVIOUS (the manifest of an earlier run) already records, as
    rendered or skipped, keeps its entry and is not rendered again (an interrupted run resumes),
    unless one of its paths is in ONLY, the paths to render again; with ONLY given, a song
    neither in it nor in PREVIOUS is left out. LIMIT: render at most that many songs (one batch);
    the songs past it are left out for the next run. SAVE(manifest) runs after each song."""
    prev = {}
    for e in (previous or {}).get("songs", []) + (previous or {}).get("skipped", []):
        prev[e["sha256"]] = e
    songs, skipped = [], []
    manifest = {"songs": songs, "skipped": skipped}
    done = 0
    for h, files in distinct_songs(songs_dir):
        e = prev.get(h)
        wanted = bool(set(files) & set(only)) if only is not None else e is None
        if not wanted or (limit is not None and done >= limit):
            if e is not None:   # keep what an earlier run recorded
                (skipped if "reason" in e else songs).append(dict(e, file=files[0], files=files))
            continue
        done += 1
        base = {"file": files[0], "files": files, "sha256": h}
        try:
            entry, song = plan_song(os.path.join(songs_dir, files[0]), mods)
            messages(entry, song)
        except Refused as e:
            skipped.append(dict(base, reason="the original refuses it (sub_49F3B4): %s" % e))
            log("SKIP  %s: refused: %s" % (files[0], e))
        except (rb1x.Error, rbm.FormatError, ValueError, KeyError, IndexError) as e:
            skipped.append(dict(base, reason="conversion: %s" % e))
            log("SKIP  %s: %s" % (files[0], e))
        else:
            digest, why = renderer.render(entry, song)
            if digest is None:
                skipped.append(dict(base, **entry, reason=why))
                log("SKIP  %s: %s" % (files[0], why))
            else:
                songs.append(dict(base, **entry, render_sha256=digest))
                log("OK    %s (%s %s, %s, idle %d): %s" % (
                    files[0], entry["form"], entry["version"], entry["mod"] or "built-in kit",
                    entry["idle_blocks"], digest))
        if save:
            save(manifest)
    seen = {e["sha256"] for e in songs + skipped}
    for e in (previous or {}).get("skipped", []):
        if e.get("excluded") and e["sha256"] not in seen:   # not a song file: kept as listed
            skipped.append(e)
    songs.sort(key=lambda e: e["file"])
    skipped.sort(key=lambda e: e["file"])
    return manifest


def exclude(manifest, songs_dir, rel, reason):
    """Lists the file REL (under SONGS_DIR, e.g. a zip holding a song) as skipped with REASON and
    "excluded": true, so later runs keep it; replaces an earlier entry of the same content."""
    with open(os.path.join(songs_dir, rel), "rb") as f:
        h = sha256(f.read())
    for k in ("songs", "skipped"):
        manifest[k] = [e for e in manifest[k] if e["sha256"] != h]
    manifest["skipped"].append({"file": rel, "files": [rel], "sha256": h, "reason": reason,
                                "excluded": True})
    manifest["skipped"].sort(key=lambda e: e["file"])
    return manifest


def merged(previous, manifest):
    """MANIFEST (a run in progress) over PREVIOUS: the entries of an earlier run that the run has
    not reached yet are kept, so a save in the middle of a run loses none."""
    by = {}
    for m in (previous or {"songs": [], "skipped": []}, manifest):
        for k in ("songs", "skipped"):
            for e in m[k]:
                by[e["sha256"]] = (k, e)
    out = {"songs": [], "skipped": []}
    for k, e in sorted(by.values(), key=lambda ke: ke[1]["file"]):
        out[k].append(e)
    return out


def write_manifest(path, manifest):
    """Writes the manifest through a temporary file, so an interrupted run leaves a whole one."""
    tmp = path + ".tmp"
    with open(tmp, "w") as f:
        json.dump(manifest, f, indent=1, ensure_ascii=False)
        f.write("\n")
    os.replace(tmp, path)


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--songs", required=True, help="the song folder (ORB_THIRDPARTY_SONGS_DIR)")
    ap.add_argument("--mods", required=True, help="the recrypted mods (ORB_THIRDPARTY_MODS_DIR)")
    ap.add_argument("--rb20ful", required=True, help="the CD's RB20FUL.DAT (local path)")
    ap.add_argument("--remote-dir", default="t129", metavar="NAME",
                    help="work in C:\\re\\rb\\NAME, deleted at the end (default t129)")
    ap.add_argument("--owner", help="locky owner (default: the git branch)")
    ap.add_argument("-o", "--out", default=DEFAULT_OUT)
    ap.add_argument("--only", nargs="+", metavar="PATH",
                    help="render these songs (paths relative to --songs) again, keep the others")
    ap.add_argument("--fresh", action="store_true",
                    help="ignore the existing manifest (-o) and render every song")
    ap.add_argument("--batch", type=int, default=100, metavar="N",
                    help="songs per machine session (default 100)")
    ap.add_argument("--exclude", nargs=2, action="append", default=[], metavar=("PATH", "REASON"),
                    help="list this file (relative to --songs) as skipped with REASON, e.g. a "
                         "zipped song the user excluded; kept by later runs")
    args = ap.parse_args(argv)
    run.set_remote_dir(args.remote_dir)
    mods = mod_names(args.mods)
    previous = None
    if not args.fresh and os.path.exists(args.out):
        with open(args.out) as f:
            previous = json.load(f)
    if args.exclude:
        previous = previous or {"songs": [], "skipped": []}
        for rel, reason in args.exclude:
            exclude(previous, args.songs, rel, reason)
        write_manifest(args.out, previous)
    exe = os.path.join(HERE, "build", "rbhost.exe")
    run.build(exe)
    client = rb_export.rcs_client.Client.from_config()
    permits = run.OnePermit(args.owner or rb_export.default_owner())
    log = lambda s: print(s, flush=True)  # noqa: E731
    only = args.only
    while True:
        pending = [h for h, files in distinct_songs(args.songs)
                   if (set(files) & set(only) if only is not None else
                       h not in {e["sha256"] for k in ("songs", "skipped")
                                 for e in (previous or {}).get(k, [])})]
        if not pending:
            break
        log("batch: %d songs left, rendering up to %d" % (len(pending), args.batch))
        tmp = tempfile.mkdtemp()
        permits.acquire()
        try:
            if rb_export.Machine(client, permits).rebirth_pids():
                raise SystemExit("Rebirth.exe is running on the machine; rbhost runs without it")
            client.upload(exe, run.REMOTE_DIR + "/rbhost.exe")
            renderer = Renderer(client, permits, args.mods, args.rb20ful, tmp)
            prev = previous
            previous = build_manifest(args.songs, mods, renderer, only, prev,
                                      save=lambda m: write_manifest(args.out, merged(prev, m)),
                                      limit=None if only is not None else args.batch, log=log)
            write_manifest(args.out, previous)
        finally:
            try:
                client.delete(run.REMOTE_DIR, recursive=True)
            except Exception as e:   # the machine is gone: the folder must be deleted by hand
                print("could not delete %s: %s" % (run.REMOTE_WIN, e), file=sys.stderr)
            permits.release()
            for name in os.listdir(tmp):
                os.remove(os.path.join(tmp, name))
            os.rmdir(tmp)
        if only is not None:   # one pass renders them all again (--batch does not split --only)
            break
    m = previous or {"songs": [], "skipped": []}
    print("%d songs rendered, %d skipped; %s" % (len(m["songs"]), len(m["skipped"]), args.out))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

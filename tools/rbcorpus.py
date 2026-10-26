#!/usr/bin/python3
"""The reference corpus under tests/refs: capture, re-capture and verification.

    tools/rbcorpus.py capture diag|shipped [NAME...]     render with rbhost, write FLACs + manifest
    tools/rbcorpus.py capture-app --app-dir DIR --fit-dir DIR NAME...
                                                         import app exports (FLAC) + fitted N
    tools/rbcorpus.py recapture [SET [NAME...]] [--work DIR]
                                                         re-render from the manifest with rbhost and
                                                         compare with it (sha256, and rbcmp.py against
                                                         the committed FLAC where there is one)
    tools/rbcorpus.py check-flac [SET [NAME...]]         every FLAC decodes to the manifest's WAV sha256
    tools/rbcorpus.py manifest                           rewrite MANIFEST.md from manifest.json

Sets (tests/refs/manifest.json is the source, MANIFEST.md is generated from it):
  diag     tests/songs/diag/songs/*.rbs, rbhost at one fixed --idle-blocks; FLAC in tests/refs/diag/
  shipped  tests/songs/shipped/*.rbs that use the built-in mod "Standard ReBirth" (not the two
           pattern-mode Default Songs; the songs with other mods need the RBM work, task-031);
           sha256 + frames only
  app      the original app's exports (docs/export_state.md 11.5), FLAC in tests/refs/app/, with the
           fitted idle count N of each: rbhost at that N must reproduce them byte for byte

The FLAC of a render is `flac -8 --best` of the WAV; `flac -d` gives back the identical WAV file
(the sha256 of the manifest is of that WAV, 44-byte header included).
Renders go through tools/rbhost/run.py (one permit of redc-win-run per invocation).
"""
import argparse
import datetime
import glob
import hashlib
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
REFS = os.path.join(ROOT, "tests", "refs")
MANIFEST = os.path.join(REFS, "manifest.json")
SONGS = {"diag": os.path.join(ROOT, "tests", "songs", "diag", "songs"),
         "shipped": os.path.join(ROOT, "tests", "songs", "shipped"),
         "app": os.path.join(ROOT, "tests", "songs", "diag", "songs")}
IDLE_BLOCKS = 9088
SKIP_SHIPPED = ["ReBirth 1.0 Default Song", "Silent Default Song"]   # pattern mode, docs/test_machine.md 8
RUN = os.path.join(ROOT, "tools", "rbhost", "run.py")
RBCMP = os.path.join(ROOT, "tools", "rbcmp.py")


def sha(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def load():
    if os.path.exists(MANIFEST):
        with open(MANIFEST) as f:
            return json.load(f)
    return {"protocol": {
        "tool": "tools/rbhost/run.py",
        "idle_blocks": IDLE_BLOCKS,
        "note": "diag and shipped are rbhost renders at one fixed idle count (any fixed N is valid: "
                "the engine takes N as a parameter); app entries are the original app's exports "
                "(docs/export_state.md 11.5) with the N that rbhost needs to reproduce each",
    }, "diag": {}, "shipped": {}, "app": {}}


def save(m):
    os.makedirs(REFS, exist_ok=True)
    with open(MANIFEST, "w") as f:
        json.dump(m, f, indent=1, sort_keys=False)
        f.write("\n")
    write_md(m)


def git_commit(path="tools/rbhost"):
    r = subprocess.run(["git", "-C", ROOT, "log", "-1", "--format=%h", "--", path],
                       capture_output=True, text=True)
    return r.stdout.strip()


def song_path(set_, name):
    return os.path.join(SONGS[set_], name + ".rbs")


def names_of(set_, names):
    if names:
        return names
    allp = sorted(glob.glob(os.path.join(SONGS[set_], "*.rbs")))
    out = [os.path.splitext(os.path.basename(p))[0] for p in allp]
    if set_ == "shipped":
        out = [n for n in out if n not in SKIP_SHIPPED and mod_of(n) == "Standard ReBirth"]
    return out


def mod_of(name):
    sys.path.insert(0, os.path.join(ROOT, "tools"))
    import rbs
    return rbs.read_song(song_path("shipped", name)).glob.mod_name


def render(songs, idle, outdir):
    """rbhost renders SONGS at IDLE under one permit; returns {stem: sidecar}."""
    os.makedirs(outdir, exist_ok=True)
    cmd = [sys.executable, RUN, "--idle-blocks", str(idle), "-o", outdir] + songs
    subprocess.run(cmd, check=True)
    res = {}
    for s in songs:
        stem = os.path.splitext(os.path.basename(s))[0]
        with open(os.path.join(outdir, stem + ".json")) as f:
            res[stem] = json.load(f)
    return res


def flac_encode(wav, flac):
    os.makedirs(os.path.dirname(flac), exist_ok=True)
    subprocess.run(["flac", "-s", "-f", "-8", "--best", "-o", flac, wav], check=True)


def flac_decode(flac, wav):
    subprocess.run(["flac", "-s", "-d", "-f", "-o", wav, flac], check=True)


def meta_of(sc):
    return {"rbhost_sha256": sc["rbhost_sha256"]}


def cmd_capture(a):
    m = load()
    set_ = a.set
    names = names_of(set_, a.names)
    work = a.work
    sc = render([song_path(set_, n) for n in names], IDLE_BLOCKS, work)
    today = datetime.date.today().isoformat()
    for n in names:
        s = sc[n]
        e = {"song": os.path.relpath(song_path(set_, n), ROOT),
             "song_sha256": s["song_sha256"],
             "tool": "rbhost", "tool_commit": git_commit(), "rbhost_sha256": s["rbhost_sha256"],
             "idle_blocks": IDLE_BLOCKS, "captured": today,
             "wav_sha256": s["wav_sha256"], "frames": s["frames"]}
        if set_ == "diag":
            flac = os.path.join(REFS, "diag", n + ".flac")
            flac_encode(os.path.join(work, n + ".wav"), flac)
            e["flac"] = os.path.relpath(flac, REFS)
            e["flac_sha256"] = sha(flac)
        m[set_][n] = e
    save(m)


def cmd_capture_app(a):
    m = load()
    today = datetime.date.today().isoformat()
    for n in a.names:
        with open(os.path.join(a.fit_dir, n + ".json")) as f:
            fit = json.load(f)
        ns = [int(k) for k, v in fit["n"].items() if v["equal"]]
        if len(ns) != 1:
            sys.exit("%s: %d matching N, need exactly one" % (n, len(ns)))
        with open(os.path.join(a.app_dir, n + ".json")) as f:
            side = json.load(f)
        wav = os.path.join(a.app_dir, n + ".wav")
        if sha(wav) != side["wav_sha256"]:
            sys.exit(n + ": app WAV does not match its sidecar")
        flac = os.path.join(REFS, "app", n + ".flac")
        flac_encode(wav, flac)
        m["app"][n] = {"song": os.path.relpath(song_path("app", n), ROOT),
                       "song_sha256": side["song"]["sha256"],
                       "tool": "tools/rb_export.py (the original app, docs/export_state.md 11.5)",
                       "tool_commit": git_commit("tools/rb_export.py"),
                       "idle_blocks": ns[0],
                       "captured": datetime.date.fromtimestamp(side["machine"]["time"]).isoformat(),
                       "wav_sha256": side["wav_sha256"], "frames": side["wav_check"]["frames"],
                       "flac": os.path.relpath(flac, REFS), "flac_sha256": sha(flac)}
        if sha(song_path("app", n)) != side["song"]["sha256"]:
            sys.exit(n + ": the song in tests/ differs from the one exported")
    save(m)


def sets_of(m, a):
    return [a.set] if a.set else ["diag", "shipped", "app"]


def cmd_recapture(a):
    m = load()
    bad = 0
    for set_ in sets_of(m, a):
        ents = m[set_]
        names = [n for n in (a.names or sorted(ents)) if n in ents]
        by_n = {}
        for n in names:
            by_n.setdefault(ents[n]["idle_blocks"], []).append(n)
        for idle, ns in sorted(by_n.items()):
            out = os.path.join(a.work, set_, str(idle))
            songs = [os.path.join(ROOT, ents[n]["song"]) for n in ns]
            for n, p in zip(ns, songs):
                if sha(p) != ents[n]["song_sha256"]:
                    print("%s/%s: song file changed since capture" % (set_, n))
                    bad += 1
            sc = render(songs, idle, out)
            for n in ns:
                e = ents[n]
                ok = sc[n]["wav_sha256"] == e["wav_sha256"] and sc[n]["frames"] == e["frames"]
                line = "%s/%s N=%d: %s" % (set_, n, idle, "equal" if ok else "DIFFERENT")
                if "flac" in e:
                    dec = os.path.join(out, n + ".ref.wav")
                    flac_decode(os.path.join(REFS, e["flac"]), dec)
                    r = subprocess.run([sys.executable, RBCMP, dec, os.path.join(out, n + ".wav")],
                                       capture_output=True, text=True)
                    line += ", rbcmp exit %d" % r.returncode
                    ok = ok and r.returncode == 0
                    if r.returncode:
                        line += "\n" + r.stdout
                print(line)
                bad += not ok
    sys.exit(1 if bad else 0)


def cmd_check_flac(a):
    m = load()
    work = a.work
    os.makedirs(work, exist_ok=True)
    bad = n_ok = 0
    for set_ in sets_of(m, a):
        for n, e in sorted(m[set_].items()):
            if "flac" not in e or (a.names and n not in a.names):
                continue
            p = os.path.join(REFS, e["flac"])
            dec = os.path.join(work, n + ".dec.wav")
            flac_decode(p, dec)
            ok = sha(p) == e["flac_sha256"] and sha(dec) == e["wav_sha256"]
            n_ok += ok
            if not ok:
                bad += 1
                print("%s/%s: FLAC does not decode to the WAV of the manifest" % (set_, n))
    print("%d FLACs decode to their WAV sha256, %d do not" % (n_ok, bad))
    sys.exit(1 if bad else 0)


def write_md(m):
    p = m["protocol"]
    L = ["# Reference corpus manifest", "",
         "Generated by `tools/rbcorpus.py manifest` from `manifest.json` (do not edit by hand).", "",
         "- diag and shipped: rbhost (`tools/rbhost/run.py`) renders at one fixed `--idle-blocks %d`"
         " (any fixed N is valid, the engine takes N as a parameter)." % p["idle_blocks"],
         "- app: the original app's exports (`tools/rb_export.py`, `docs/export_state.md` 11.5), each with"
         " the N at which rbhost reproduces it byte for byte (`docs/export_state.md` 13).",
         "- A FLAC decodes (`flac -d`) to the exact WAV whose sha256 is listed (header included, 16-bit"
         " stereo 44100 Hz). Shipped songs have no audio committed: the sha256 and frame count are"
         " re-captured by `tools/rbcorpus.py recapture shipped`.",
         "- Not in the corpus: the two pattern-mode Default Songs (cannot be exported, `docs/test_machine.md`"
         " section 8) and the shipped songs that use another mod than \"Standard ReBirth\" (Alloy Cowboy"
         " Resurrection, Clodhopper, Fonda Workout, Groove It Up a 2nd Time!, Metallicon City, Mutilated"
         " Species II, RSI, SuperNova): they need the RBM work (task-031).",
         "- Re-capture: `tools/rbcorpus.py recapture [diag|shipped|app] [NAME...]`;"
         " FLAC check: `tools/rbcorpus.py check-flac`.", ""]
    for set_, title in (("diag", "Diagnostic songs (rbhost)"), ("shipped", "Shipped songs (rbhost, hash only)"),
                        ("app", "App exports (cross-check of rbhost)")):
        ents = m[set_]
        if not ents:
            continue
        e0 = next(iter(ents.values()))
        L += ["## %s" % title, "",
              "%d references, captured %s, tool %s%s." % (
                  len(ents), e0["captured"], e0["tool"],
                  (", rbhost.exe sha256 %s, tools/rbhost commit %s" % (e0["rbhost_sha256"][:12], e0["tool_commit"])
                   if "rbhost_sha256" in e0 else ", tools/rb_export.py commit %s" % e0["tool_commit"])), "",
              "| Name | Song sha256 | N | Frames | WAV sha256 | Reference |", "|---|---|---|---|---|---|"]
        for n, e in sorted(ents.items()):
            L.append("| %s | %s | %d | %d | %s | %s |" % (
                n, e["song_sha256"][:12], e["idle_blocks"], e["frames"], e["wav_sha256"],
                e.get("flac", "hash only")))
        L.append("")
    with open(os.path.join(REFS, "MANIFEST.md"), "w") as f:
        f.write("\n".join(L))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("capture")
    c.add_argument("set", choices=["diag", "shipped"])
    c.add_argument("names", nargs="*")
    c.add_argument("--work", required=True)
    c.set_defaults(f=cmd_capture)
    c = sub.add_parser("capture-app")
    c.add_argument("--app-dir", required=True)
    c.add_argument("--fit-dir", required=True)
    c.add_argument("names", nargs="+")
    c.set_defaults(f=cmd_capture_app)
    c = sub.add_parser("recapture")
    c.add_argument("set", nargs="?", choices=["diag", "shipped", "app"])
    c.add_argument("names", nargs="*")
    c.add_argument("--work", required=True)
    c.set_defaults(f=cmd_recapture)
    c = sub.add_parser("check-flac")
    c.add_argument("set", nargs="?", choices=["diag", "shipped", "app"])
    c.add_argument("names", nargs="*")
    c.add_argument("--work", required=True)
    c.set_defaults(f=cmd_check_flac)
    c = sub.add_parser("manifest")
    c.set_defaults(f=lambda a: write_md(load()))
    a = ap.parse_args()
    a.f(a)


if __name__ == "__main__":
    main()

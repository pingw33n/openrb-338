#!/usr/bin/python3
"""Writes, or checks, tests/refs/thirdparty_mods.json, the hash manifest the third-party mods test
checks the user's recrypted Mods folder against (task-121).

    thirdparty_mods_manifest.py --orig MODS.ORIG --mods MODS --dat RB20FUL.DAT \
        [-o tests/refs/thirdparty_mods.json] [--orb PATH]
    thirdparty_mods_manifest.py --check --orig MODS.ORIG --dat RB20FUL.DAT \
        [--manifest tests/refs/thirdparty_mods.json] [--orb PATH]

MODS.ORIG is the folder of the ORIGINAL third-party mods and MODS the folder of the same files
recrypted (`orb mod recrypt`, same file names); RB20FUL.DAT is the CD's key file.

Writing: the originals are recrypted in bulk with the CURRENT orb (`orb mod recrypt`, into a
temporary folder), and every result must equal the matching file of MODS byte for byte, else
nothing is written. Then each original is read with the independent decoder (tools/rbm.py:
container, key pages, descrambling, the AIFF of each of the 46 slots) and the manifest keeps only
hashes and names, never file content:

    {"mods": [{"file", "sha256" (of the RECRYPTED file), "name" (the INFO menu name), "error",
               "files": [[name, sha256 of the file]], "slots": {slot name: sha256 of the channel-0
               PCM as int16 BE}, "slot_errors": {slot name: message}}, ...]}

So the manifest pins what the current recryptor makes of the originals. `--check` is the opt-in
re-verification of the recryptor on the real collection: it recrypts MODS.ORIG with the current
orb and compares every result with the committed manifest (the file set and each SHA-256).

`--orb` is the orb binary (default: target/release/orb, built first with cargo). The cargo test
`mods` of crates/orb-cli/tests/thirdparty.rs needs only ORB_THIRDPARTY_MODS_DIR (the recrypted
folder) and
this file. Regenerate it when the collection changes (the test fails on a mod it does not list):
the originals and the key file are the only inputs that are not in the repository, and the paths
are arguments here, not environment variables of any test. Stdlib only.
"""
import argparse
import hashlib
import json
import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rbm  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def file_sha256(path):
    with open(path, "rb") as f:
        return hashlib.sha256(f.read()).hexdigest()


def find_orb(orb):
    """The orb binary: `orb` if given, else target/release/orb after a release build."""
    if orb:
        return orb
    subprocess.run(["cargo", "build", "--release", "-q", "-p", "orb-cli"], cwd=ROOT, check=True)
    return os.path.join(ROOT, "target", "release", "orb" + (".exe" if os.name == "nt" else ""))


def recrypt_originals(orb, orig_dir, dat, out_dir):
    """`orb mod recrypt` over the folder of originals into out_dir; returns {file: sha256}."""
    r = subprocess.run([orb, "mod", "recrypt", orig_dir, "--rb20ful", dat, "-o", out_dir],
                       stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    if r.returncode != 0:
        raise SystemExit("orb mod recrypt failed:\n" + r.stderr)
    return {f: file_sha256(os.path.join(out_dir, f)) for f in sorted(os.listdir(out_dir))
            if f.lower().endswith(".rbm")}


def mod_entry(orig_path, mods_dir, dat):
    """The manifest entry of one original mod and its recrypted file."""
    c = rbm.check_mod(orig_path, dat)
    file = os.path.basename(orig_path)
    recrypted = os.path.join(mods_dir, file)
    if not os.path.isfile(recrypted):
        raise SystemExit("%s: no recrypted file %s" % (file, recrypted))
    slots = {s: v["sha256"] for s, v in c["slots"].items() if "error" not in v}
    return {
        "file": file,
        "sha256": file_sha256(recrypted),
        "name": c["name"],
        "error": c["error"],
        "files": [[f["name"], f["sha256"]] for f in c["files"]],
        "slots": slots,
        "slot_errors": {s: v["error"] for s, v in c["slots"].items() if "error" in v},
    }


def build_manifest(orig_dir, mods_dir, dat, orb=None):
    """The manifest; with `orb`, first requires orb's recrypt of the originals to equal MODS."""
    names = sorted(f for f in os.listdir(orig_dir) if f.lower().endswith(".rbm"))
    if not names:
        raise SystemExit("no .rbm in %s" % orig_dir)
    if orb:
        with tempfile.TemporaryDirectory() as tmp:
            made = recrypt_originals(orb, orig_dir, dat, os.path.join(tmp, "out"))
        bad = [n for n in names
               if made.get(n) != file_sha256(os.path.join(mods_dir, n))]
        if bad:
            raise SystemExit("orb mod recrypt of the originals differs from %s for %d mods: %s"
                             % (mods_dir, len(bad), ", ".join(bad)))
    return {"mods": [mod_entry(os.path.join(orig_dir, n), mods_dir, dat) for n in names]}


def check_manifest(orig_dir, dat, manifest, orb):
    """Problems (a list of strings) between orb's recrypt of the originals and the manifest."""
    with tempfile.TemporaryDirectory() as tmp:
        made = recrypt_originals(orb, orig_dir, dat, os.path.join(tmp, "out"))
    want = {m["file"]: m["sha256"] for m in manifest["mods"]}
    problems = ["%s: not in the manifest" % n for n in sorted(set(made) - set(want))]
    problems += ["%s: not recrypted from the originals" % n for n in sorted(set(want) - set(made))]
    problems += ["%s: orb's recrypt differs from the manifest" % n
                 for n in sorted(set(made) & set(want)) if made[n] != want[n]]
    return problems


def dumps(manifest):
    """One mod per line, so that a changed mod is one line of a diff."""
    lines = [json.dumps(m, separators=(",", ":"), sort_keys=True) for m in manifest["mods"]]
    return '{"mods":[\n' + ",\n".join(lines) + "\n]}\n"


def main(argv=None):
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--orig", required=True, help="the folder of the original .rbm files")
    ap.add_argument("--mods", help="the folder of the same files, recrypted (not with --check)")
    ap.add_argument("--dat", required=True, help="the CD's RB20FUL.DAT")
    ap.add_argument("--check", action="store_true",
                    help="recrypt --orig with the current orb and compare with the manifest")
    ap.add_argument("--manifest", default=os.path.join(ROOT, "tests/refs/thirdparty_mods.json"),
                    help="the manifest --check reads")
    ap.add_argument("-o", "--out", default="tests/refs/thirdparty_mods.json")
    ap.add_argument("--orb", help="the orb binary (default: build target/release/orb)")
    a = ap.parse_args(argv)
    orb = find_orb(a.orb)
    if a.check:
        with open(a.manifest) as f:
            problems = check_manifest(a.orig, a.dat, json.load(f), orb)
        for p in problems:
            print(p)
        print("%s: %s" % (a.manifest, "%d differences" % len(problems) if problems else "equal"))
        return 1 if problems else 0
    if not a.mods:
        ap.error("--mods is required without --check")
    manifest = build_manifest(a.orig, a.mods, a.dat, orb)
    with open(a.out, "w", newline="\n") as f:
        f.write(dumps(manifest))
    print("%d mods -> %s" % (len(manifest["mods"]), a.out))
    return 0


if __name__ == "__main__":
    sys.exit(main())

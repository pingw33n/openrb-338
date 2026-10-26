#!/usr/bin/python3
"""task-010: variants of diagnostic songs with both TB-303s silenced.

    mkvar.py OUTDIR NAME...     (NAME of tests/songs/diag/specs/NAME.json)

Writes OUTDIR/NAME_no303.json and .rbs (tools/rbs.py build): both 303 devices disabled, mixer
channels 0 and 1 disabled with level 0, and their tb303_1_level / tb303_2_level track events at 0."""
import json
import os
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))


def main(outdir, names):
    for name in names:
        with open(os.path.join(ROOT, "tests", "songs", "diag", "specs", name + ".json")) as f:
            spec = json.load(f)
        for i in ("0", "1"):
            spec["tb303"][i]["params"]["enabled"] = 0
        for c in spec["mixer"]["channels"][:2]:
            c["enabled"] = 0
            c["level"] = 0
        n = 0
        for trk in spec.get("tracks", {}).values():
            for ev in trk:
                if ev.get("ctrl") in ("tb303_1_level", "tb303_2_level"):
                    ev["value"] = 0
                    n += 1
        out = os.path.join(outdir, name + "_no303.json")
        with open(out, "w") as f:
            json.dump(spec, f, indent=1)
            f.write("\n")
        subprocess.check_call([sys.executable, os.path.join(ROOT, "tools", "rbs.py"), "build", out,
                               out[:-5] + ".rbs"])
        print(name, "level events zeroed:", n)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2:])

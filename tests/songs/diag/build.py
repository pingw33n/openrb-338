#!/usr/bin/env python3
"""Build the diagnostic songs: specs/NAME.json -> songs/NAME.rbs with tools/rbs.py build.

    build.py [NAME...]     all songs, or only the named ones (run gen_specs.py first to refresh specs/)
"""
import glob
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
RBS = os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(HERE))), "tools", "rbs.py")


def main(names):
    os.makedirs(os.path.join(HERE, "songs"), exist_ok=True)
    specs = sorted(glob.glob(os.path.join(HERE, "specs", "*.json")))
    if names:
        specs = [p for p in specs if os.path.basename(p)[:-5] in names]
    for p in specs:
        out = os.path.join(HERE, "songs", os.path.basename(p)[:-5] + ".rbs")
        subprocess.check_call([sys.executable, RBS, "build", p, out])
    print("built %d songs" % len(specs))


if __name__ == "__main__":
    main(sys.argv[1:])

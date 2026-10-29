#!/usr/bin/env python3
"""Write songs with the delay disabled (task-023).

    tools/nodelay.py OUTDIR SONG.rbs...

For each song, writes OUTDIR/<name>.rbs, the same song with exactly two values changed:
- the DELY chunk's byte 0 ("enabled", docs/RBS42.txt; msg 41 `5/86` byte 0) set to 0;
- every event of the delay's TRAK (track 5) whose controller is 0 ("enabled") set to value 0
  (the songs of tests/songs/diag have one, at position 0; in song mode the chase at the loop start
  sets the live value from it, docs/song_load.md section 6).
Everything else is written back as read (tools/rbs.py keeps unnamed bytes).

These variants isolate the TB-303s from the enabled delay, whose output is in every diag
reference (docs/initial_investigation.md section 8.4). crates/orb-engine/tests/fixtures/
tb303_nodelay.json holds rbhost's renders of them:

    tools/nodelay.py OUT tests/songs/diag/songs/NAME.rbs ...
    tools/rbhost/run.py --idle-blocks 9088 -o REF OUT/*.rbs
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rbs  # noqa: E402


def without_delay(song):
    d = rbs.to_json(song)
    d["delay"]["values"]["enabled"] = 0
    for e in d["tracks"]["delay"]:
        if e["ctrl"] == "enabled":
            e["value"] = 0
    out = rbs.from_json(d)
    rbs.restore_raw(out, song)
    rbs.validate(out)
    return out


def main(argv):
    if len(argv) < 2:
        sys.stderr.write(__doc__)
        return 2
    outdir = argv[0]
    os.makedirs(outdir, exist_ok=True)
    for p in argv[1:]:
        song = without_delay(rbs.read_song(p))
        with open(os.path.join(outdir, os.path.basename(p)), "wb") as f:
            f.write(song.build())
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

#!/usr/bin/python3
"""Generate crates/orb-engine/src/tables/data.rs, the engine's static tables, from the DLL.

    gen_static_tables.py --dll PATH/ReBirth.dll [-o crates/orb-engine/src/tables/data.rs]

--dll defaults to the environment variable ORB_REBIRTH_DLL. The DLL is the one of your ReBirth 2.01
installation (SHA-256 4c3a6441...8b12), or the one on the CD (ORB_CD_ISO, orb-testkit).

Reads each table from ReBirth.dll by virtual address through the PE section map (image base
0x10000000), writes the Rust constants and formats the file with rustfmt. The unit test
`tables::tests::static_tables_match_dll` checks the result against the DLL's bytes. Stdlib only.
"""
import argparse
import os
import struct
import subprocess
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

# name, virtual address, Rust element type, struct format, shape (None: a scalar), doc
ITEMS = [
    ("LEVEL_STEP", 0x1014F8E4, "f32", "f", None,
     "The step of the level curve `T` (f32 1/24, `sub_100046B0` at 0x100046C3)."),
    ("TB303_TUNE", 0x10033F90, "i16", "h", [32],
     "TB-303 tune offset in semitones, indexed by `tune >> 2` (note-on 0x10023CD4)."),
    ("DIST_GRID", 0x10151AB8, "f32", "f", [5, 5],
     "Distortion: the 5x5 grid of the output gain `r`, interpolated bilinearly at (shape, amount)\n"
     "/// (update 0x10010894)."),
    ("PCF_DECAY", 0x100332D0, "f32", "f", [16],
     "PCF envelope decay `dec` per 4 samples."),
    ("PCF_LENGTHS", 0x10033310, "u16", "H", [55],
     "PCF pattern lengths in steps."),
    ("PCF_RESOLUTIONS", 0x10033380, "u16", "H", [55],
     "PCF pattern resolutions: 8, 16 or 32 steps per bar."),
    ("PCF_STEPS", 0x100333F0, "u8", "B", [55, 32],
     "PCF pattern steps: 0 = no trigger, otherwise the velocity (odd = slow attack)."),
    ("PCF_DATA", 0x10033310, "u8", "B", [0x10034400 - 0x10033310],
     "The PCF pattern data as the block 0x100166D0 addresses it: the lengths (u16 at `+0`), the\n"
     "/// resolutions (u16 at `+0x70`) and the steps (`+0xE0`, 32 per pattern), then what follows them\n"
     "/// up to the end of `.data`'s file part (the `CPCFInsert` vtable 0x10033AD4 and later data).\n"
     "/// The block indexes all three with the raw wave byte and the DLL does not check it, so a wave\n"
     "/// above 54 reads past the 55 patterns into these bytes; every read of a wave 0..127 stays\n"
     "/// inside them (the 17 waves of length 0, 55, 111 and 113..127, fault before they read a step).\n"
     "///\n"
     "/// The bytes are the file's, so they hold what the DLL has when it loads at its preferred base\n"
     "/// 0x10000000, as it does in the app and in rbhost (decided by the user, 2026-10-27, task-027):\n"
     "/// 0x10033AD0..0x10034400 holds 171 base-relocated pointer words (the `CPCFInsert` vtable at\n"
     "/// 0x10033AD4 and later), and 57 waves of 55..127 have one among their 32 step bytes (35 of\n"
     "/// them within their pattern length), so their velocities assume that base."),
    ("TR808_SD_SAMPLE", 0x10034090, "i16", "h", [5],
     "TR-808 snare: the device's sample slot of tone sample A, indexed by `(tune*5) >> 7`\n"
     "/// (trigger at 0x1002A268)."),
    ("TR808_SD_NOISE_SEQ", 0x1003409C, "u16", "H", [32],
     "SEQ808: the fixed order of the snare noise samples, indexed by the global counter\n"
     "/// `word_1014CBD4` (pre-incremented, `& 31`)."),
    ("TR808_SD_NOISE_SAMPLE", 0x100340DC, "u16", "H", [4],
     "TR-808 snare: the device's sample slot of noise sample B, indexed by the SEQ808 value."),
    ("TR808_TONE", 0x100340E4, "f32", "f", [8],
     "TR-808 BD and CY low-pass coefficient, indexed by `tone >> 4` (0x1002A171)."),
    ("TR808_DECAY", 0x10034104, "f32", "f", [16],
     "TR-808 BD and CY gain multiplier per pair, indexed by `decay >> 3` (0x1002A18A)."),
    ("TR909_SD_SAMPLE", 0x10034210, "i16", "h", [5],
     "TR-909 snare: the device's sample slot of tone sample A, indexed by `(tune*5) >> 7`\n"
     "/// (0x1002DCCD)."),
    ("TR909_SD_NOISE_SEQ", 0x1003421C, "u16", "H", [32],
     "SEQ909: the fixed order of the snare noise samples, indexed by the global counter\n"
     "/// `word_1014E548` (pre-incremented, `& 31`)."),
    ("TR909_SD_NOISE_SAMPLE", 0x1003425C, "u16", "H", [4],
     "TR-909 snare: the device's sample slot of noise sample B, indexed by the SEQ909 value\n"
     "/// (0..2; the fourth entry is 0 and unused)."),
    ("TR909_BD_ATTACK", 0x10034264, "f32", "f", [8],
     "TR-909 BD low-pass coefficient `C8`, indexed by `attack >> 4` (0x1002DBC8)."),
]

HEADER = """\
//! The engine's static tables, as the original stores them in ReBirth.dll's data.
//!
//! Generated from ReBirth.dll by `tools/gen_static_tables.py`; the unit test
//! `static_tables_match_dll` reads every table back from the DLL by file offset and asserts it
//! equals these constants bit for bit. An f32 value is written as the shortest decimal that
//! parses to the same f32. Addresses are virtual addresses in the DLL (image base 0x10000000).
"""


class Pe:
    """A PE file addressed by virtual address."""

    def __init__(self, path):
        with open(path, "rb") as f:
            self.data = f.read()
        pe = struct.unpack_from("<I", self.data, 0x3C)[0]
        if self.data[pe:pe + 4] != b"PE\0\0":
            raise ValueError("%s: not a PE file" % path)
        count = struct.unpack_from("<H", self.data, pe + 6)[0]
        opt = struct.unpack_from("<H", self.data, pe + 20)[0]
        self.base = struct.unpack_from("<I", self.data, pe + 24 + 28)[0]
        self.sections = []
        for i in range(count):
            vsize, va, rsize, raw = struct.unpack_from("<IIII", self.data, pe + 24 + opt + 40 * i + 8)
            self.sections.append((self.base + va, vsize, raw, rsize))

    def read(self, va, n):
        for sva, vsize, raw, rsize in self.sections:
            if sva <= va < sva + max(vsize, rsize):
                off = va - sva
                if off + n > rsize:
                    raise ValueError("0x%08X: not in the file (zero-filled)" % va)
                return self.data[raw + off:raw + off + n]
        raise ValueError("0x%08X: no section" % va)


def literal(t, v):
    if t == "f32":
        # the shortest decimal that parses back to the same f32
        for p in range(1, 12):
            s = "%.*g" % (p, v)
            if struct.pack("<f", float(s)) == struct.pack("<f", v):
                break
        if "e" not in s and "." not in s:
            s += ".0"
        return s
    return str(v)


def rust_type(t, shape):
    s = t
    for n in reversed(shape):
        s = "[%s; %d]" % (s, n)
    return s


def array(t, vals, shape):
    if len(shape) == 1:
        return "[" + ", ".join(literal(t, v) for v in vals) + "]"
    step = len(vals) // shape[0]
    return "[" + ", ".join(array(t, vals[i * step:(i + 1) * step], shape[1:])
                           for i in range(shape[0])) + "]"


def generate(dll):
    out = [HEADER]
    for name, va, t, fmt, shape, doc in ITEMS:
        n = 1
        for s in shape or []:
            n *= s
        vals = list(struct.unpack("<%d%s" % (n, fmt), dll.read(va, n * struct.calcsize(fmt))))
        out.append("/// %s\n///\n/// At 0x%08X." % (doc, va))
        if shape is None:
            out.append("pub const %s: %s = %s;\n" % (name, t, literal(t, vals[0])))
        else:
            out.append("pub const %s: %s = %s;\n" % (name, rust_type(t, shape), array(t, vals, shape)))
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--dll", default=os.environ.get("ORB_REBIRTH_DLL"),
                    help="ReBirth.dll (default: $ORB_REBIRTH_DLL)")
    ap.add_argument("-o", "--out", default=os.path.join(ROOT, "crates", "orb-engine", "src", "tables", "data.rs"))
    args = ap.parse_args()
    if not args.dll:
        sys.exit("gen_static_tables.py: no DLL. Pass --dll PATH/ReBirth.dll (your ReBirth 2.01 "
                 "installation) or set ORB_REBIRTH_DLL.")
    if not os.path.isfile(args.dll):
        sys.exit("gen_static_tables.py: %s not found." % args.dll)
    text = generate(Pe(args.dll))
    with open(args.out, "w") as f:
        f.write(text)
    subprocess.run(["rustfmt", "--edition", "2024", args.out], check=True, cwd=ROOT)
    print(args.out)


if __name__ == "__main__":
    sys.exit(main())

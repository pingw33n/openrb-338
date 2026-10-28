"""Tests of tools/rbm.py: /usr/bin/python3 -m unittest tools/rbm_test.py

The synthetic tests build a mod and a sparse RB20FUL.DAT stand-in. The tests on real files skip
with a message unless the environment names them (Propellerhead material is never committed). The
user does not set these two variables: the cargo test `the_python_tool_tests_pass_on_the_real_files`
(crates/rbm/tests/rbm.rs) sets them for this script, with the files it takes out of the CD image
(`RB_CD_ISO`; task-121):
    RBM_TEST_DAT    path of RB20FUL.DAT from the ReBirth CD
    RBM_TEST_MODS   a folder with the shipped mods (PBE2.rbm, MSM2.rbm, Metallicon.rbm,
                    Orbit2.rbm), as shipped (not recrypted)
"""
import glob
import os
import random
import struct
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rbm  # noqa: E402
import rbs  # noqa: E402


def chunk(tag, body):
    return tag + struct.pack(">I", len(body)) + body + (b"\0" if len(body) & 1 else b"")


def aiff(samples):
    comm = struct.pack(">HIH", 1, len(samples), 16) + bytes([0x40, 0x0E, 0xAC, 0x44]) + bytes(6)
    ssnd = struct.pack(">II", 0, 0) + b"".join(struct.pack(">h", s) for s in samples)
    body = b"AIFF" + chunk(b"COMM", comm) + chunk(b"SSND", ssnd)
    return b"FORM" + struct.pack(">I", len(body)) + body


def info(name):
    b = bytearray(1280)
    b[0:5] = b"text\0"
    b[1003:1003 + len(name)] = name
    b[1073:1077] = bytes([0xBC, 1, 0, 0])
    return bytes(b)


def build_mod(files, table=None):
    """files: [(name, bytes)]; with table, scramble each payload with its chunk's key."""
    chunks = [chunk(b"HEAD", bytes(256))]
    for i, (name, data) in enumerate(files):
        payload = name.encode() + b"\0" + data
        if table is not None:
            payload = rbm.scramble(payload, (i + 1) & 0xFF, table)
        chunks.append(chunk(b"EMBF", payload))
    chunks.append(chunk(b"INFO", info(b"Synthetic")))
    body = b"PRBM" + b"".join(chunks)
    return b"CAT " + struct.pack(">I", len(body)) + body


def reference_sums(payloads):
    """sub_4149EC written out once more, from the disassembly, for one stride of 4."""
    acc = [0x2000, 0x3000]
    for p in payloads:
        buf = (p[:1024] + b"\x65" * 1024)[:1024]
        words = struct.unpack(">256I", buf)
        acc[0] = (acc[0] + sum(words[:128])) & 0xFFFFFFFF
        acc[1] = (acc[1] + sum(words[128:])) & 0xFFFFFFFF
    return acc


class Descramble(unittest.TestCase):
    def test_period_and_half(self):
        table = bytes(range(256)) * 16
        payload = bytes(1024 + 2000)
        for key in (0, 1, 2, 255):
            out = rbm.descramble(payload, key, table)
            self.assertEqual(out[:1024], bytes(1024))
            period = (17 * key) % 1925 + 123
            base = (key & 1) * 2048
            expect = bytes(table[base + n % period] for n in range(2000))
            self.assertEqual(out[1024:], expect, key)
        self.assertEqual((17 * 255) % 1925 + 123, 608)

    def test_equals_the_byte_loop(self):
        # the translate-per-residue implementation against the DLL's loop written out
        rnd = random.Random(4)
        table = bytes(rnd.randrange(256) for _ in range(4096))
        for size in (1025, 1024 + 123, 1024 + 124, 7000):
            payload = bytes(rnd.randrange(256) for _ in range(size))
            for key in (0, 1, 113, 255):
                period = (17 * key) % 1925 + 123
                base = (key & 1) * 2048
                expect = bytearray(payload)
                j = 0
                for n in range(1024, size):
                    expect[n] = (expect[n] + table[base + j]) & 0xFF
                    j = 0 if j + 1 == period else j + 1
                self.assertEqual(rbm.descramble(payload, key, table), bytes(expect), (size, key))

    def test_short_payload_untouched(self):
        self.assertEqual(rbm.descramble(b"\x01" * 1024, 3, bytes(4096)), b"\x01" * 1024)

    def test_inverse(self):
        rnd = random.Random(1)
        table = bytes(rnd.randrange(256) for _ in range(4096))
        payload = bytes(rnd.randrange(256) for _ in range(5000))
        for key in (0, 7, 200):
            self.assertEqual(rbm.descramble(rbm.scramble(payload, key, table), key, table), payload)


class Container(unittest.TestCase):
    def setUp(self):
        rnd = random.Random(2)
        self.samples = [rnd.randrange(-32768, 32768) for _ in range(3000)]
        self.files = [("tr808bd.aif", aiff(self.samples)), ("x.jpg", b"\xff\xd8" + bytes(10)),
                      ("TR909sd1.aif", aiff(self.samples[:100]))]

    def test_parse(self):
        r = rbm.parse(build_mod(self.files))
        self.assertEqual([c.tag for c in r.chunks], [b"HEAD", b"EMBF", b"EMBF", b"EMBF", b"INFO"])
        self.assertEqual([c.key for c in r.embf()], [1, 2, 3])
        self.assertEqual(r.info.name, b"Synthetic")
        self.assertEqual(r.info.version, (1, 0, 0))
        self.assertIs(r.find("TR808BD.aif"), r.embf()[0])     # case-insensitive
        self.assertIs(r.find("tr909SD1.AIF"), r.embf()[2])
        self.assertIsNone(r.find("TR808SD1.aif"))

    def test_key_sums(self):
        data = build_mod(self.files)
        r = rbm.parse(data)
        payloads = [r.data[c.offset:c.offset + c.size] for c in r.embf()]
        self.assertEqual(rbm.key_sums(r), reference_sums(payloads))
        self.assertEqual(rbm.key_sums(rbm.parse(build_mod([]))), [0x2000, 0x3000])

    def test_round_trip_through_dat(self):
        r0 = rbm.parse(build_mod(self.files))
        pages = rbm.key_pages(r0)
        rnd = random.Random(3)
        table = bytes(rnd.randrange(256) for _ in range(4096))
        with tempfile.TemporaryDirectory() as d:
            dat = os.path.join(d, "RB20FUL.DAT")
            with open(dat, "wb") as f:
                f.write(struct.pack(">I", rbm.DAT_MAGIC))
                f.truncate(rbm.DAT_HEADER + rbm.DAT_BODY)    # sparse
                for j, off in enumerate(pages):
                    f.seek(off)
                    f.write(table[2048 * j:2048 * (j + 1)])
            # the sums see only the clear first 1024 bytes, so scrambling keeps the pages
            r = rbm.parse(build_mod(self.files, table))
            self.assertEqual(rbm.key_pages(r), pages)
            self.assertEqual(rbm.key_table(r, dat), table)
            got = rbm.embedded_file(r, r.find("TR808BD.aif"), table)
            self.assertEqual(got, self.files[0][1])
            self.assertEqual(rbm.aiff_samples(got), self.samples)
            msg = rbm.mod_message(r, table, "C:\\m\\x.rbm", 412)
        self.assertEqual(len(msg), 1 + 9542)
        self.assertEqual(msg[0], 55)
        p = msg[1:]
        self.assertEqual(struct.unpack_from(">HI", p, 0), (412, rbm.FILESPEC_TAG))
        self.assertEqual(p[6:6 + 11], b"C:\\m\\x.rbm\0")
        self.assertEqual(p[4802:4802 + 4096], table)
        c = r.find("TR808BD.aif")
        self.assertEqual(p[8898:8898 + 14], bytes([1, 1]) + struct.pack(">III", c.offset, 12, c.size))
        self.assertEqual(p[8898 + 14:8898 + 28], bytes(14))                 # TR808SD1: built-in
        c = r.find("TR909sd1.aif")
        self.assertEqual(p[9234 + 4 * 14:9234 + 5 * 14], bytes([1, 3]) + struct.pack(">III", c.offset, 13, c.size))

    def test_bad(self):
        with self.assertRaises(rbm.FormatError):
            rbm.parse(b"FORM" + bytes(8))
        data = bytearray(build_mod(self.files))
        data[8:12] = b"XXXX"      # the exe does not check the CAT type: a warning only
        with self.assertWarns(UserWarning):
            self.assertEqual(len(rbm.parse(bytes(data)).embf()), 3)
        data = bytearray(build_mod(self.files))
        data[4:8] = struct.pack(">I", len(data))
        with self.assertRaises(rbm.FormatError):
            rbm.parse(bytes(data))

    def test_check_dir(self):
        import contextlib
        import io
        import json
        with tempfile.TemporaryDirectory() as d:
            dat = os.path.join(d, "RB20FUL.DAT")
            with open(dat, "wb") as f:
                f.write(struct.pack(">I", rbm.DAT_MAGIC))
                f.truncate(rbm.DAT_HEADER + rbm.DAT_BODY)    # sparse: a zero table
            mods = os.path.join(d, "Mods")
            os.mkdir(mods)
            # same INFO name: "AXIOM(B3).RBM" < "AXIOM.RBM" in NTFS order, so axiom(b3) owns it
            for name in ("AXIOM.rbm", "axiom(b3).rbm"):
                with open(os.path.join(mods, name), "wb") as f:
                    f.write(build_mod(self.files))
            with open(os.path.join(mods, "junk.rbm"), "wb") as f:
                f.write(b"not a mod")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                rc = rbm.main(["check-dir", mods, dat, "--json"])
        self.assertEqual(rc, 1)                                   # junk.rbm fails
        res = json.loads(out.getvalue())
        self.assertEqual([m["file"] for m in res], ["axiom(b3).rbm", "AXIOM.rbm", "junk.rbm"])
        self.assertNotIn("shadowed_by", res[0])
        self.assertEqual(res[1]["shadowed_by"], "axiom(b3).rbm")
        self.assertTrue(res[2]["error"].startswith("parse"))
        self.assertEqual(sorted(res[0]["slots"]), ["TR808BD.aif", "TR909sd1.aif"])
        self.assertEqual(res[0]["slots"]["TR808BD.aif"]["frames"], 3000)
        self.assertIn("44 of 46 slots built in", res[0]["notes"])
        self.assertEqual(len(res[0]["files"]), 3)

    def test_check_stride_pages(self):
        with tempfile.TemporaryDirectory() as d:
            dat = os.path.join(d, "RB20FUL.DAT")
            with open(dat, "wb") as f:
                f.write(struct.pack(">I", rbm.DAT_MAGIC))
                f.truncate(rbm.DAT_HEADER + rbm.DAT_BODY)
                for k in range(0x4000):
                    base = rbm.DAT_HEADER + ((k + 0x4000) << 11)
                    f.seek(base + 0xE0)
                    f.write(struct.pack("<I", ((k + 18) << 14) | 0x2ABC))
                    f.seek(base + 0x3AC)
                    f.write(struct.pack("<I", ((k + 8) << 12) | 0x123))
                f.seek(rbm.DAT_HEADER + ((5 + 0x4000) << 11) + 0xE0)
                f.write(struct.pack("<I", (5 + 19) << 14))
            x, y = rbm.check_stride_pages(dat)
        self.assertEqual(x, {18: 0x4000 - 1, 19: 1})
        self.assertEqual(y, {8: 0x4000})


DAT = os.environ.get("RBM_TEST_DAT")
MODS = os.environ.get("RBM_TEST_MODS")
# The key pages of the shipped mods (RB20FUL.DAT offsets), from the mods' own bytes (task-030).
PAGES = {"PBE2.rbm": [118280160, 44302526], "MSM2.rbm": [8878154, 9419548],
         "Metallicon.rbm": [17382559, 49264678], "Orbit2.rbm": [39986275, 69184465]}


@unittest.skipUnless(DAT, "RBM_TEST_DAT not set: no RB20FUL.DAT to check")
class RealDat(unittest.TestCase):
    def test_stride_is_4_on_every_page(self):
        # docs/rbm.md 5.2: the parent-window page unk_4DAD78 cancels the API value on every page
        x, y = rbm.check_stride_pages(DAT)
        self.assertEqual(x, {18: 0x4000})
        self.assertEqual(y, {8: 0x4000})


@unittest.skipUnless(MODS, "RBM_TEST_MODS not set: no real mods to test")
class RealMods(unittest.TestCase):
    def mods(self):
        paths = sorted(glob.glob(os.path.join(MODS, "*.rbm")))
        self.assertTrue(paths, "no .rbm in RBM_TEST_MODS")
        return paths

    def test_pages(self):
        for path in self.mods():
            name = os.path.basename(path)
            if name in PAGES:
                self.assertEqual(rbm.key_pages(rbm.read(path)), PAGES[name], name)

    @unittest.skipUnless(DAT, "RBM_TEST_DAT not set: cannot descramble")
    def test_descrambled_files_are_valid(self):
        for path in self.mods():
            r = rbm.read(path)
            table = rbm.key_table(r, DAT)
            for slot in rbm.SLOT_NAMES[1:]:
                c = r.find(slot)
                if c:
                    rbm.aiff_info(rbm.embedded_file(r, c, table))
            for c in r.embf():
                b = rbm.embedded_file(r, c, table)
                low = c.name.lower()
                if low.endswith(".jpg") and c.size > 1024:
                    self.assertTrue(b.rstrip(b"\0").endswith(b"\xff\xd9"), (path, c.name))
                if low.endswith(".rbs"):
                    with tempfile.TemporaryDirectory() as d:
                        song = os.path.join(d, "demo.rbs")
                        with open(song, "wb") as f:
                            f.write(b)
                        rbs.read_song(song)


if __name__ == "__main__":
    unittest.main()

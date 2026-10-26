"""Tests of tools/rbcmp.py: /usr/bin/python3 -m unittest tools/rbcmp_test.py"""
import array
import contextlib
import io
import json
import os
import random
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rbcmp  # noqa: E402


def make_samples(frames, seed=1):
    r = random.Random(seed)
    return [r.randint(-20000, 20000) for _ in range(frames * 2)]


def write_wav(path, samples, rate=44100):
    pcm = array.array("h", samples).tobytes()
    hdr = b"RIFF" + struct.pack("<I", 36 + len(pcm)) + b"WAVE"
    hdr += b"fmt " + struct.pack("<IHHIIHH", 16, 1, 2, rate, rate * 4, 4, 16)
    hdr += b"data" + struct.pack("<I", len(pcm))
    with open(path, "wb") as f:
        f.write(hdr + pcm)


class T(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, self.dir)

    def wav(self, name, samples):
        p = os.path.join(self.dir, name)
        write_wav(p, samples)
        return p

    def run_cli(self, *args):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = rbcmp.main(list(args))
        return code, out.getvalue()

    def test_equal(self):
        s = make_samples(5000)
        a, b = self.wav("a.wav", s), self.wav("b.wav", s)
        code, out = self.run_cli(a, b, "--json")
        r = json.loads(out)
        self.assertEqual(code, 0)
        self.assertTrue(r["equal"])
        self.assertIsNone(r["first_diff"])
        self.assertEqual(r["differing_samples"], 0)

    def test_one_lsb(self):
        s = make_samples(5000)
        t = list(s)
        t[2 * 4410 + 1] += 1  # frame 4410, right channel
        a, b = self.wav("a.wav", s), self.wav("b.wav", t)
        code, out = self.run_cli(a, b, "--json", "--bpm", "120")
        r = json.loads(out)
        self.assertEqual(code, 1)
        self.assertFalse(r["equal"])
        self.assertEqual(r["first_diff"]["frame"], 4410)
        self.assertAlmostEqual(r["first_diff"]["time"], 0.1)
        # 120 bpm: 5512.5 frames per sixteenth -> sixteenth 1 of bar 1
        self.assertEqual((r["first_diff"]["bar"], r["first_diff"]["beat"], r["first_diff"]["sixteenth"]), (1, 1, 1))
        self.assertEqual(r["max_abs_diff"], [0, 1])
        self.assertEqual(r["differing_samples"], 1)
        self.assertEqual(r["histogram"]["1"], 1)
        self.assertAlmostEqual(r["rms_diff"], (1 / 10000) ** 0.5)
        self.assertEqual(self.run_cli(a, b, "--tolerance", "1")[0], 0)

    def test_bar_beat(self):
        p = rbcmp.position(int(44100 * 2.5), 44100, 120)  # 2.5 s = 5 beats = bar 2 beat 2
        self.assertEqual((p["bar"], p["beat"], p["sixteenth"]), (2, 2, 1))

    def test_histogram(self):
        s = [0] * 20
        t = list(s)
        t[0], t[1], t[2], t[3], t[4] = 1, -3, 15, 16, -1000
        r = rbcmp.compare({"channels": 2, "rate": 44100, "samples": array.array("h", s)},
                          {"channels": 2, "rate": 44100, "samples": array.array("h", t)})
        self.assertEqual(r["histogram"], {"1": 1, "2..3": 1, "4..15": 1, "16+": 2})

    def test_offset(self):
        s = make_samples(6000)
        shifted = [0] * 2 * 37 + s  # B delayed by 37 frames
        a, b = self.wav("a.wav", s), self.wav("b.wav", shifted)
        code, out = self.run_cli(a, b, "--align", "100", "--json")
        r = json.loads(out)
        self.assertEqual(code, 1)
        self.assertEqual(r["align"]["offset"], 37)
        self.assertEqual(r["align"]["window_mismatches"], 0)
        self.assertEqual(r["align"]["stats"]["differing_samples"], 0)
        r = json.loads(self.run_cli(b, a, "--align", "100", "--json")[1])
        self.assertEqual(r["align"]["offset"], -37)
        self.assertEqual(r["align"]["stats"]["differing_samples"], 0)

    def test_align_counts_frames(self):
        s = make_samples(3000)
        b = [0] * 2 * 3 + s  # delayed by 3
        b[2 * 18] += 1       # corrupt one frame in both channels
        b[2 * 18 + 1] += 1
        r = json.loads(self.run_cli(self.wav("a.wav", s), self.wav("b.wav", b), "--align", "5", "--json")[1])
        self.assertEqual(r["align"]["offset"], 3)
        self.assertEqual(r["align"]["window_mismatches"], 1)
        self.assertLessEqual(r["align"]["window_mismatches"], r["align"]["window_frames"])

    def test_malformed_wav(self):
        def riff(chunks):
            body = b"WAVE" + chunks
            return b"RIFF" + struct.pack("<I", len(body)) + body
        cases = {
            "trunc.wav": riff(b"fmt " + struct.pack("<I", 8) + b"\x01\x00\x02\x00\x44\xac\x00\x00"),
            "zero.wav": riff(b"fmt " + struct.pack("<IHHIIHH", 16, 1, 0, 44100, 0, 0, 16)
                             + b"data" + struct.pack("<I", 4) + b"\0\0\0\0"),
        }
        good = self.wav("g.wav", make_samples(10))
        for name, data in cases.items():
            p = os.path.join(self.dir, name)
            with open(p, "wb") as f:
                f.write(data)
            with self.assertRaises(ValueError):
                rbcmp.load(p)
            with contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(rbcmp.main([good, p]), 2)

    def test_lengths(self):
        s = make_samples(5000)
        a, b = self.wav("a.wav", s), self.wav("b.wav", s[:2 * 4000])
        code, out = self.run_cli(a, b, "--json", "--tolerance", "5")
        r = json.loads(out)
        self.assertEqual(code, 1)
        self.assertFalse(r["length_equal"])
        self.assertFalse(r["equal"])
        self.assertEqual(r["frames_compared"], 4000)
        self.assertEqual(r["differing_samples"], 0)

    def test_text_report_and_bad_file(self):
        a = self.wav("a.wav", make_samples(100))
        code, out = self.run_cli(a, a)
        self.assertEqual(code, 0)
        self.assertIn("EQUAL", out)
        junk = os.path.join(self.dir, "j.wav")
        with open(junk, "wb") as f:
            f.write(b"nope")
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(rbcmp.main([a, junk]), 2)

    @unittest.skipUnless(shutil.which("flac"), "flac not installed")
    def test_flac(self):
        s = make_samples(5000)
        t = list(s)
        t[100] += 2
        a = self.wav("a.wav", s)
        bw = self.wav("b.wav", t)
        bf = os.path.join(self.dir, "b.flac")
        subprocess.run(["flac", "-s", "-f", bw, "-o", bf], check=True)
        code, out = self.run_cli(a, bf, "--json")
        r = json.loads(out)
        self.assertEqual(code, 1)
        self.assertEqual(r["max_abs_diff"], [2, 0])
        af = os.path.join(self.dir, "a.flac")
        subprocess.run(["flac", "-s", "-f", a, "-o", af], check=True)
        self.assertEqual(self.run_cli(a, af)[0], 0)


if __name__ == "__main__":
    unittest.main()

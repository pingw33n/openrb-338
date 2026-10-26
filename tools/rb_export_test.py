"""Tests of the offline parts of tools/rb_export.py: /usr/bin/python3 -m unittest tools/rb_export_test.py"""
import os
import struct
import sys
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import rb_export  # noqa: E402
import rbs  # noqa: E402

SONGS = os.path.join(os.path.dirname(HERE), "tests", "songs", "shipped")


def wav_bytes(frames, rate=44100, riff_delta=0):
    data = bytes(4 * frames)
    fmt = struct.pack("<HHIIHH", 1, 2, rate, rate * 4, 4, 16)
    body = b"WAVE" + b"fmt " + struct.pack("<I", 16) + fmt + b"data" + struct.pack("<I", len(data)) + data
    return b"RIFF" + struct.pack("<I", len(body) + riff_delta) + body


class ExportTest(unittest.TestCase):
    def check(self, data, frames):
        with tempfile.NamedTemporaryFile(suffix=".wav") as f:
            f.write(data)
            f.flush()
            return rb_export.check_wav(f.name, frames)

    def test_frames_match_the_app(self):
        # docs/test_machine.md section 4: Calm exported 14,003,446 frames
        song = rbs.read_song(os.path.join(SONGS, "Calm.rbs"))
        self.assertEqual(rb_export.export_frames(song), 14003446)

    def test_wav_ok(self):
        self.assertEqual(self.check(wav_bytes(100), 100)["frames"], 100)

    def test_wav_wrong_length(self):
        with self.assertRaises(rb_export.Failure):
            self.check(wav_bytes(100), 101)

    def test_wav_bad_riff_size(self):
        with self.assertRaises(rb_export.Failure):
            self.check(wav_bytes(100, riff_delta=4), 100)

    def test_wav_bad_format(self):
        with self.assertRaises(rb_export.Failure):
            self.check(wav_bytes(100, rate=48000), 100)


class TasklistTest(unittest.TestCase):
    class FakeClient:
        def __init__(self, rc, out):
            self.rc, self.out = rc, out

        def exec(self, program, args, cwd=""):
            return {"id": 1}

        def proc(self, pid):
            return {"running": False, "exit_code": self.rc}

        def output(self, pid):
            return self.out, b""

    def pids(self, rc, out):
        return rb_export.Machine(self.FakeClient(rc, out), None).rebirth_pids()

    def test_listing(self):
        self.assertEqual(self.pids(0, b'"Rebirth.exe","1234","Console","1","9,000 K"\r\n'), [1234])

    def test_none_running(self):
        self.assertEqual(self.pids(0, b"INFO: No tasks are running which match the specified criteria.\r\n"), [])

    def test_nonzero_exit(self):
        with self.assertRaises(rb_export.Failure):
            self.pids(1, b"")

    def test_unparseable(self):
        with self.assertRaises(rb_export.Failure):
            self.pids(0, b"garbage\r\n")
        with self.assertRaises(rb_export.Failure):
            self.pids(0, b"")


if __name__ == "__main__":
    unittest.main()

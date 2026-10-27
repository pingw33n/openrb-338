"""Tests of the mod name check of tools/rbhost/run.py and fit.py:
/usr/bin/python3 -m unittest tools/rbhost/run_test.py"""
import os
import sys
import types
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.dirname(HERE))
import run  # noqa: E402


def fake_mod(name):
    return types.SimpleNamespace(info=types.SimpleNamespace(name=name.encode("latin-1")) if name else None)


class ModNameCheck(unittest.TestCase):
    def check(self, mod_name, song_mod):
        with mock.patch.object(run.rbm, "read", return_value=fake_mod(mod_name)):
            return run.check_mod_name("x.rbm", song_mod)

    def test_exact_name_accepted(self):
        self.assertIsNotNone(self.check("Pitch Black Edition 2.0", "Pitch Black Edition 2.0"))

    def test_no_song_mod_no_check(self):
        self.assertIsNotNone(self.check("Pitch Black Edition 2.0", None))

    def test_mismatch_refused(self):
        for song_mod in ("Massen's MSM 2.0", "pitch black edition 2.0", "Pitch Black Edition 2.0 ",
                         " Pitch Black Edition 2.0", "Standard ReBirth"):
            with self.assertRaises(run.Failure, msg=song_mod):
                self.check("Pitch Black Edition 2.0", song_mod)

    def test_mod_without_info_refused(self):
        with self.assertRaises(run.Failure):
            self.check(None, "Pitch Black Edition 2.0")

    def test_upload_mod_checks_the_name(self):
        args = types.SimpleNamespace(mod="x.rbm", rb20ful="d.dat", tag="x", mod_id=412)
        with mock.patch.object(run.rbm, "read", return_value=fake_mod("Metallicon+")):
            with self.assertRaises(run.Failure):
                run.upload_mod(args, None, ".", "Massen's MSM 2.0")


if __name__ == "__main__":
    unittest.main()

"""Tests of tools/rbhost/thirdparty_songs.py with a fake renderer (no machine)."""
import os
import shutil
import sys
import tempfile
import types
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
TOOLS = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, TOOLS)
import rb1x_test  # noqa: E402
import thirdparty_songs as ts  # noqa: E402

SHIPPED = os.path.join(os.path.dirname(TOOLS), "tests", "songs", "shipped")


class FakeRenderer:
    """Returns a hash made of the settings, or a failure for the names in FAIL."""

    def __init__(self, fail=()):
        self.fail, self.calls = set(fail), []

    def render(self, entry, song):
        self.calls.append(dict(entry))
        if entry["mod_name"] in self.fail:
            return None, "rbhost exited with 1: boom"
        return "h-%s-%s-%d" % (entry["form"], entry["mod"], entry["idle_blocks"]), None


class Order(unittest.TestCase):
    def test_ntfs_order(self):
        names = ["AXIOM.rbm", "axiom(b3).rbm", "b.rbm", "Alpha2.rbm", "alpha3.rbm"]
        self.assertEqual(sorted(names, key=ts.ntfs_key),
                         ["Alpha2.rbm", "alpha3.rbm", "axiom(b3).rbm", "AXIOM.rbm", "b.rbm"])

    def test_resolve_first_listed_wins(self):
        mods = [("axiom(b3).rbm", "AXIOM"), ("AXIOM.rbm", "AXIOM"), ("x.rbm", None)]
        self.assertEqual(ts.resolve(mods, "AXIOM"), "axiom(b3).rbm")
        self.assertIsNone(ts.resolve(mods, "axiom"))          # the INFO name, exact
        self.assertIsNone(ts.resolve(mods, "Standard ReBirth"))
        self.assertIsNone(ts.resolve([("Standard.rbm", "Standard ReBirth")], "Standard ReBirth"))

    def test_mod_names_reads_info_in_order(self):
        d = tempfile.mkdtemp()
        try:
            for f in ("b.rbm", "A.rbm", "c.txt"):
                open(os.path.join(d, f), "wb").close()
            info = {"A.rbm": "Alpha", "b.rbm": None}

            def read(p):
                n = info[os.path.basename(p)]
                return types.SimpleNamespace(
                    info=types.SimpleNamespace(name=n.encode("latin-1")) if n else None)
            with mock.patch.object(ts.rbm, "read", side_effect=read):
                self.assertEqual(ts.mod_names(d), [("A.rbm", "Alpha"), ("b.rbm", None)])
        finally:
            shutil.rmtree(d)


class Refusal(unittest.TestCase):
    def test_the_exe_file_check(self):
        with open(os.path.join(SHIPPED, "Calm.rbs"), "rb") as f:
            calm = f.read()
        self.assertIsNone(ts.refusal(calm))
        self.assertEqual(ts.refusal(calm + b"\x0c\x1a"),     # yawn.rbs: 2 bytes after the CAT
                         "the first chunk is %d bytes long, the file %d" % (len(calm),
                                                                             len(calm) + 2))
        self.assertIn("not FORM", ts.refusal(b"\r\n(This file must be converted"))
        self.assertIsNotNone(ts.refusal(calm[:-1]))
        self.assertIsNotNone(ts.refusal(b"CAT "))


class Manifest(unittest.TestCase):
    def setUp(self):
        self.d = tempfile.mkdtemp()
        shutil.copy(os.path.join(SHIPPED, "Calm.rbs"), os.path.join(self.d, "Calm.rbs"))
        os.makedirs(os.path.join(self.d, "Pack", "deeper"))
        shutil.copy(os.path.join(SHIPPED, "Calm.rbs"), os.path.join(self.d, "Pack", "Calm.rbs"))
        shutil.copy(os.path.join(SHIPPED, "Metallicon City.rbs"),
                    os.path.join(self.d, "Pack", "deeper", "M.RBS"))
        with open(os.path.join(self.d, "old.rbs"), "wb") as f:
            f.write(rb1x_test.song(rb1x_test.block(), []))
        with open(os.path.join(self.d, "bad.rbs"), "wb") as f:
            f.write(b"neither")
        with open(os.path.join(self.d, "notes.txt"), "wb") as f:
            f.write(b"not a song")
        os.mkdir(os.path.join(self.d, "sub.rbs"))
        self.mods = [("Metallicon.rbm", "Metallicon+")]

    def tearDown(self):
        shutil.rmtree(self.d)

    def test_entries(self):
        r = FakeRenderer()
        m = ts.build_manifest(self.d, self.mods, r, log=lambda s: None)
        by = {e["file"]: e for e in m["songs"]}
        self.assertEqual(sorted(by), ["Calm.rbs", "Pack/deeper/M.RBS", "old.rbs"])
        calm = by["Calm.rbs"]
        self.assertEqual(calm["files"], ["Calm.rbs", "Pack/Calm.rbs"])   # recursive, deduplicated
        self.assertEqual((calm["form"], calm["mod"], calm["idle_blocks"]), ("rbs", None, 9088))
        self.assertEqual(calm["render_sha256"], "h-rbs-None-9088")
        met = by["Pack/deeper/M.RBS"]
        self.assertEqual((met["mod_name"], met["mod"]), ("Metallicon+", "Metallicon.rbm"))
        old = by["old.rbs"]
        self.assertEqual((old["form"], old["version"], old["mod"], old["idle_blocks"]),
                         ("1x", "3.1", None, 0))
        self.assertEqual([s["file"] for s in m["skipped"]], ["bad.rbs"])
        self.assertIn("refuses", m["skipped"][0]["reason"])
        self.assertEqual(len(r.calls), 3)
        self.assertNotIn("sub.rbs", str(m))   # a folder named like a song is not a song

    def test_missing_mod_is_the_builtin_kit(self):
        m = ts.build_manifest(self.d, [], FakeRenderer(), log=lambda s: None)
        e = next(e for e in m["songs"] if e["file"] == "Pack/deeper/M.RBS")
        self.assertEqual((e["mod_name"], e["mod"]), ("Metallicon+", None))

    def test_render_failure_is_skipped_with_the_reason(self):
        m = ts.build_manifest(self.d, self.mods, FakeRenderer({"Metallicon+"}), log=lambda s: None)
        s = next(s for s in m["skipped"] if s["file"] == "Pack/deeper/M.RBS")
        self.assertIn("boom", s["reason"])
        self.assertNotIn("Pack/deeper/M.RBS", [e["file"] for e in m["songs"]])

    def test_only_renders_again_and_keeps_the_rest(self):
        full = ts.build_manifest(self.d, self.mods, FakeRenderer(), log=lambda s: None)
        full["songs"][0]["render_sha256"] = "old"
        r2 = FakeRenderer()
        m = ts.build_manifest(self.d, self.mods, r2, only=["Pack/deeper/M.RBS"], previous=full,
                              log=lambda s: None)
        self.assertEqual([c["mod_name"] for c in r2.calls], ["Metallicon+"])
        self.assertEqual([e["file"] for e in m["songs"]], [e["file"] for e in full["songs"]])
        self.assertEqual(m["songs"][0]["render_sha256"], "old")
        self.assertEqual(m["skipped"], full["skipped"])

    def test_batches_resume_where_the_last_stopped(self):
        saves = []
        r1 = FakeRenderer()
        # the order: Calm.rbs, Pack/deeper/M.RBS, bad.rbs, old.rbs
        part = ts.build_manifest(self.d, self.mods, r1, limit=3, save=saves.append,
                                 log=lambda s: None)
        self.assertEqual(len(r1.calls), 2)       # bad.rbs does not reach the renderer
        self.assertEqual(len(saves), 3)
        self.assertEqual(len(part["songs"]) + len(part["skipped"]), 3)
        r2 = FakeRenderer()
        rest = ts.build_manifest(self.d, self.mods, r2, previous=part, log=lambda s: None)
        self.assertEqual(len(r2.calls), 1)       # only the song the first batch did not record
        full = ts.build_manifest(self.d, self.mods, FakeRenderer(), log=lambda s: None)
        self.assertEqual(rest, full)
        r3 = FakeRenderer()
        self.assertEqual(ts.build_manifest(self.d, self.mods, r3, previous=rest,
                                           log=lambda s: None), full)
        self.assertEqual(r3.calls, [])

    def test_an_excluded_file_is_kept(self):
        with open(os.path.join(self.d, "Pack", "x.zip"), "wb") as f:
            f.write(b"PK")
        m = ts.build_manifest(self.d, self.mods, FakeRenderer(), log=lambda s: None)
        ts.exclude(m, self.d, "Pack/x.zip", "zipped; excluded by the user")
        e = next(e for e in m["skipped"] if e["file"] == "Pack/x.zip")
        self.assertEqual((e["reason"], e["excluded"], e["files"]),
                         ("zipped; excluded by the user", True, ["Pack/x.zip"]))
        r = FakeRenderer()
        again = ts.build_manifest(self.d, self.mods, r, previous=m, log=lambda s: None)
        self.assertEqual(again, m)
        self.assertEqual(r.calls, [])

    def test_merged_keeps_the_entries_not_reached(self):
        full = ts.build_manifest(self.d, self.mods, FakeRenderer(), log=lambda s: None)
        now = {"songs": [dict(full["songs"][0], render_sha256="new")], "skipped": []}
        m = ts.merged(full, now)
        self.assertEqual([e["file"] for e in m["songs"]], [e["file"] for e in full["songs"]])
        self.assertEqual(m["songs"][0]["render_sha256"], "new")
        self.assertEqual(m["skipped"], full["skipped"])

    def test_write_manifest_is_whole(self):
        p = os.path.join(self.d, "m.json")
        ts.write_manifest(p, {"songs": [], "skipped": []})
        with open(p) as f:
            self.assertEqual(f.read(), '{\n "songs": [],\n "skipped": []\n}\n')
        self.assertFalse(os.path.exists(p + ".tmp"))


if __name__ == "__main__":
    unittest.main()

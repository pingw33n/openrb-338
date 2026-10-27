"""Tests of the pure parts of tools/rbhost/fit.py: /usr/bin/python3 -m unittest tools/rbhost/fit_test.py"""
import argparse
import json
import os
import sys
import tempfile
import unittest
from unittest import mock

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.dirname(HERE))
import fit  # noqa: E402


class CandidateOrder(unittest.TestCase):
    def test_nearest_first_in_steps_of_16(self):
        self.assertEqual(fit.candidate_order(9100, 7), [9104, 9120, 9088, 9136, 9072, 9152, 9056])

    def test_never_negative(self):
        order = fit.candidate_order(20, 5)
        self.assertEqual(order, [16, 32, 0, 48, 64])
        self.assertTrue(all(n >= 0 and n % 16 == 0 for n in fit.candidate_order(5, 40)))

    def test_no_duplicates(self):
        order = fit.candidate_order(100, 60)
        self.assertEqual(len(order), len(set(order)))


class TimelineN(unittest.TestCase):
    def sidecar(self, steps):
        return {"timing": {"timeline": [{"t": t, "step": s} for s, t in steps]}}

    def test_blocks_between_open_and_export(self):
        sc = self.sidecar([("song opened", 7.491), ("export started", 20.668)])
        self.assertEqual(fit.timeline_n(sc), round(13.177 * 44100 / 64))

    def test_missing_step(self):
        self.assertIsNone(fit.timeline_n(self.sidecar([("song opened", 1.0)])))


class FakePermits:
    def __init__(self, n=1):
        self.n, self.last, self.released = n, 1e18, False

    def refresh(self):
        pass

    def release(self):
        self.released = True


class FakeFitter(fit.Fitter):
    """Fitter whose renders hash to "hit" at the N in `hits` and to "miss" elsewhere."""

    def __init__(self, args, hits, n=1):
        super().__init__(args, FakePermits(n), "x")
        self.hits, self.rendered = hits, []
        self.new_client = lambda: None

    def render_hash(self, client, wid, rbhm, idle, extra, frames):
        self.rendered.append(idle)
        return ("hit" if idle in self.hits else "miss"), 0.01


def fake_args(**kw):
    d = dict(all=False, range=None, offset=48.0, max_candidates=20)
    d.update(kw)
    return argparse.Namespace(**d)


def sidecar(nt, sha="hit"):
    return {"wav_sha256": sha, "timing": {"timeline": [{"t": 0.0, "step": "song opened"},
                                                       {"t": nt * 64 / 44100, "step": "export started"}]}}


class Fitting(unittest.TestCase):
    def test_median_offset(self):
        self.assertEqual(fit.next_offset([], 48.0), 48.0)
        self.assertEqual(fit.next_offset([40, 90, 60], 48.0), 60)

    def test_stops_at_the_first_match_but_all_goes_on(self):
        # N_t 992 + 48 -> centre 1040; the match is the second candidate (1056)
        f = FakeFitter(fake_args(), {1056})
        res = fit.fit_songs(f.args, f, [("a", "a.rbhm", 100, sidecar(992), False)], [], [])
        self.assertEqual(res[0]["matches"], [1056])
        self.assertEqual(f.rendered, [1040, 1056])
        f = FakeFitter(fake_args(all=True, max_candidates=5), {1056, 1008})
        res = fit.fit_songs(f.args, f, [("a", "a.rbhm", 100, sidecar(992), False)], [], [])
        self.assertEqual(res[0]["matches"], [1008, 1056])
        self.assertEqual(len(f.rendered), 5)

    def test_offset_is_learned_between_songs(self):
        f = FakeFitter(fake_args(), {1104, 2112})   # N - N_t = 104 for both songs
        jobs = [("a", "a", 1, sidecar(1000), False), ("b", "b", 1, sidecar(2008), False)]
        res = fit.fit_songs(f.args, f, jobs, [], [])
        self.assertEqual([e["N_minus_N_t"] for e in res], [104, 104])
        self.assertEqual(res[1]["offset_used"], 104)
        self.assertEqual(res[1]["candidates_tried"], 1)

    def test_silent_song_is_all_n_and_stays_out_of_the_offset(self):
        f = FakeFitter(fake_args(), {1056})
        jobs = [("s", "s", 1, sidecar(1000), True), ("b", "b", 1, sidecar(2000), False)]
        res = fit.fit_songs(f.args, f, jobs, [], [])
        self.assertTrue(res[0]["all_n"])
        self.assertEqual(res[0]["matches"], [])
        self.assertEqual(res[0]["candidates_tried"], 1)
        self.assertNotIn("N_minus_N_t", res[0])
        self.assertEqual(res[1]["offset_used"], 48.0)

    def test_silent_wav_detection(self):
        with tempfile.TemporaryDirectory() as d:
            z, nz = os.path.join(d, "z.wav"), os.path.join(d, "n.wav")
            with open(z, "wb") as f:
                f.write(b"RIFF" + bytes(40) + bytes(1000))
            with open(nz, "wb") as f:
                f.write(b"RIFF" + bytes(40) + bytes(500) + b"\x01" + bytes(10))
            self.assertTrue(fit.is_silent_wav(z))
            self.assertFalse(fit.is_silent_wav(nz))
            self.assertFalse(fit.is_silent_wav(os.path.join(d, "none.wav")))

    def test_exec_is_not_retried(self):
        calls = []

        class C:
            def exec(self, *a, **k):
                calls.append(1)
                raise fit.rb_export.rcs_client.RcsError(0, "net")

        f = FakeFitter(fake_args(), set())
        f.args.tag, f.args.timeout = "t", 5
        f.render_hash = fit.Fitter.render_hash.__get__(f)
        with mock.patch.object(fit.run, "rbhost_options", return_value=[]):
            with self.assertRaises(fit.rb_export.rcs_client.RcsError):
                f.render_hash(C(), 0, "x.rbhm", 16, [], 1)
        self.assertEqual(len(calls), 1)


class Permits(unittest.TestCase):
    def test_released_and_fit_json_written_after_an_unexpected_error(self):
        perm = FakePermits(1)
        perm.acquire = lambda: None

        class C:
            def upload_if_changed(self, *a):
                raise RuntimeError("boom")

        with tempfile.TemporaryDirectory() as d, \
                mock.patch.object(fit, "SomePermits", return_value=perm), \
                mock.patch.object(fit.run, "build"), \
                mock.patch.object(fit.rb_export, "sha256_file", return_value="x"), \
                mock.patch.object(fit.rb_export.rcs_client.Client, "from_config", return_value=C()):
            rc = fit.main(["--app-dir", d, "-o", d, "x.rbs"])
            self.assertEqual(rc, 1)
            self.assertTrue(perm.released)
            with open(os.path.join(d, "fit.json")) as f:
                self.assertEqual(json.load(f)["songs"], [])


if __name__ == "__main__":
    unittest.main()

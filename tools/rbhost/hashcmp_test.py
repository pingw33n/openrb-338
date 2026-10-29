"""Tests of tools/rbhost/hashcmp.py with a fake rcs client (no machine)."""
import hashlib
import os
import sys
import tempfile
import time
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
TOOLS = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, TOOLS)
import hashcmp  # noqa: E402
import rb1x_test  # noqa: E402

CALM = os.path.join(os.path.dirname(TOOLS), "tests", "songs", "shipped", "Calm.rbs")


def sha(b):
    return hashlib.sha256(b).hexdigest()


class FakeClient:
    """rbhost exits with the codes given in order; the machine's WAV hashes are given in order."""

    def __init__(self, codes, hashes):
        self.codes, self.hashes = list(codes), list(hashes)
        self.uploads, self.deletes, self.rbhost_runs = [], [], 0
        self.ids = {}

    def upload_if_changed(self, local, remote):
        self.uploads.append(remote)

    def upload(self, local, remote):
        self.uploads.append(remote)

    def exec(self, program, args, cwd=""):
        pid = len(self.ids) + 1
        if program.endswith("rbhost.exe"):
            self.rbhost_runs += 1
            self.ids[pid] = ("rbhost", self.codes.pop(0))
        else:   # tasklist: no Rebirth.exe
            self.ids[pid] = ("tasklist", 0)
        return {"id": pid}

    def proc(self, pid):
        return {"running": False, "exit_code": self.ids[pid][1]}

    def output(self, pid):
        if self.ids[pid][0] == "tasklist":
            return b"INFO: No tasks are running which match the specified criteria.\r\n", b""
        return b"rbhost: ok\n", b""

    def hash(self, remote, algo="sha256"):
        return {"hash": self.hashes.pop(0)}

    def delete(self, remote, recursive=False):
        self.deletes.append(remote)


class FakePermits:
    def __init__(self):
        self.held, self.acquired, self.released, self.last = False, 0, 0, time.time()

    def acquire(self):
        self.held, self.acquired = True, self.acquired + 1

    def refresh(self):
        self.last = time.time()

    def release(self):
        self.held, self.released = False, self.released + 1


class HashcmpTest(unittest.TestCase):
    def setUp(self):
        self.dir = tempfile.mkdtemp()
        self.song1x = os.path.join(self.dir, "one.rbs")
        with open(self.song1x, "wb") as f:
            f.write(rb1x_test.song(rb1x_test.block(), []))
        self.songs = hashcmp.unique_songs([CALM, self.song1x, CALM])

    def tearDown(self):
        os.remove(self.song1x)
        os.rmdir(self.dir)

    def run_compare(self, client, contents):
        permits = FakePermits()
        out = list(contents)

        def render(p, wav):
            with open(wav, "wb") as f:
                f.write(out.pop(0))
            return True

        try:
            return hashcmp.compare(self.songs, client, permits, 0, render, log=lambda s: None), permits
        finally:
            self.permits = permits

    def assert_cleaned(self, client):
        for name in hashcmp.REMOTE_FILES:
            self.assertIn(hashcmp.run.REMOTE_DIR + "/" + name, client.deletes)
        self.assertEqual((self.permits.acquired, self.permits.released, self.permits.held), (1, 1, False))

    def test_duplicates_skipped(self):
        self.assertEqual([p for p, _ in self.songs], [CALM, self.song1x])

    def test_equal_run(self):
        client = FakeClient([0, 0], [sha(b"A"), sha(b"B")])
        res, _ = self.run_compare(client, [b"A", b"B"])
        self.assertEqual([e["equal"] for e in res], [True, True])
        self.assertEqual(res[1]["song"], "one.rbs")
        self.assertEqual(client.rbhost_runs, 2)
        self.assert_cleaned(client)

    def test_mismatch(self):
        client = FakeClient([0, 0], [sha(b"A"), sha(b"X")])
        res, _ = self.run_compare(client, [b"A", b"B"])
        self.assertEqual([e["equal"] for e in res], [True, False])
        self.assertEqual((res[1]["rbhost_sha256"], res[1]["orb_sha256"]), (sha(b"X"), sha(b"B")))
        self.assert_cleaned(client)

    def test_failure_mid_run_cleans_up(self):
        client = FakeClient([0, 3], [sha(b"A")])
        with self.assertRaises(hashcmp.run.Failure):
            self.run_compare(client, [b"A"])
        self.assertEqual(client.rbhost_runs, 2)
        self.assert_cleaned(client)


if __name__ == "__main__":
    unittest.main()

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


class PrefsRestoreTest(unittest.TestCase):
    """The user's preferences are restored after every app run, before the permits are released."""

    class FakeClient:
        def __init__(self, saved=rb_export.PRF_SAVED_SHA256):
            self.files = {rb_export.PRF_REMOTE: "user"}
            if saved is not None:
                self.files[rb_export.PRF_SAVED] = saved
            self.pids = []
            self.n = 0

        def hash(self, remote, algo="sha256"):
            if remote not in self.files:
                raise RuntimeError("no such file")
            return {"hash": self.files[remote]}

        def exec(self, program, args, cwd=""):
            self.n += 1
            self.last = (program, list(args))
            return {"id": self.n}

        def proc(self, pid):
            return {"running": False, "exit_code": 0}

        def output(self, pid):
            program, args = self.last
            if program == "tasklist":
                if not self.pids:
                    return b"INFO: No tasks are running which match the specified criteria.\r\n", b""
                return b"".join(b'"Rebirth.exe","%d","Console","1","9 K"\r\n' % p for p in self.pids), b""
            if program == "cmd.exe" and args[:3] == ["/c", "copy", "/Y"]:
                self.files[rb_export.PRF_REMOTE] = self.files[rb_export.PRF_SAVED]
            return b"", b""

    class FakePermits:
        def __init__(self, c):
            self.c, self.released_with = c, None

        def acquire(self):
            pass

        def release(self):
            self.released_with = self.c.files[rb_export.PRF_REMOTE]

    def run_with(self, client, export):
        m = rb_export.Machine(client, self.FakePermits(client))
        m.restore_wait = 0
        orig = rb_export.export_songs

        def fake(args, c, mm, outdir, owner):
            mm.prf_touched = True
            client.files[rb_export.PRF_REMOTE] = "rewritten by the app"
            return export()
        rb_export.export_songs = fake
        try:
            rc = rb_export.run_guarded(None, client, m, m.permits, ".", "t")
        except BaseException as e:
            rc = type(e).__name__
        finally:
            rb_export.export_songs = orig
        return rc, m

    def test_normal_run(self):
        c = self.FakeClient()
        rc, m = self.run_with(c, lambda: ["x.wav"])
        self.assertEqual(rc, 0)
        self.assertEqual(c.files[rb_export.PRF_REMOTE], rb_export.PRF_SAVED_SHA256)
        self.assertEqual(m.permits.released_with, rb_export.PRF_SAVED_SHA256)

    def test_failed_run(self):
        c = self.FakeClient()
        rc, m = self.run_with(c, lambda: None)
        self.assertEqual(rc, 1)
        self.assertEqual(m.permits.released_with, rb_export.PRF_SAVED_SHA256)

    def test_interrupt(self):
        def boom():
            raise KeyboardInterrupt()
        c = self.FakeClient()
        rc, m = self.run_with(c, boom)
        self.assertEqual(rc, "KeyboardInterrupt")
        self.assertEqual(m.permits.released_with, rb_export.PRF_SAVED_SHA256)

    def test_app_still_running_fails_and_releases(self):
        c = self.FakeClient()
        c.pids = [77]
        rc, m = self.run_with(c, lambda: ["x.wav"])
        self.assertEqual(rc, 1)
        self.assertEqual(m.permits.released_with, "rewritten by the app")

    def test_missing_saved_copy_fails_before_the_app(self):
        c = self.FakeClient(saved=None)
        rc, m = self.run_with(c, lambda: self.fail("the export must not run"))
        self.assertEqual(rc, 1)
        self.assertFalse(m.prf_touched)
        self.assertEqual(c.files[rb_export.PRF_REMOTE], "user")

    def test_saved_copy_with_wrong_hash(self):
        c = self.FakeClient(saved="00")
        rc, m = self.run_with(c, lambda: self.fail("the export must not run"))
        self.assertEqual(rc, 1)


class RestoreWaitTest(unittest.TestCase):
    class Client(PrefsRestoreTest.FakeClient):
        def __init__(self, gone_after):
            super().__init__()
            self.gone_after, self.polls, self.killed = gone_after, 0, False
            self.pids = [77]
            self.files[rb_export.PRF_REMOTE] = "rewritten by the app"

        def output(self, pid):
            if self.last[0] == "tasklist":
                self.polls += 1
                if self.gone_after is not None and self.polls > self.gone_after:
                    self.pids = []
            if self.last[0] == "taskkill":
                self.killed = True
            return super().output(pid)

    def machine(self, client, wait):
        m = rb_export.Machine(client, None)
        m.restore_wait = wait
        m.started_pids = [77]
        return m

    def test_app_exits_during_the_wait(self):
        c = self.Client(gone_after=1)
        self.machine(c, 5).restore_saved_prf()
        self.assertEqual(c.files[rb_export.PRF_REMOTE], rb_export.PRF_SAVED_SHA256)
        self.assertFalse(c.killed)

    def test_app_stays_after_the_wait(self):
        c = self.Client(gone_after=None)
        with self.assertRaises(rb_export.Failure):
            self.machine(c, 0.5).restore_saved_prf()
        self.assertFalse(c.killed)
        self.assertEqual(c.files[rb_export.PRF_REMOTE], "rewritten by the app")


class RealFlowTest(unittest.TestCase):
    """run_guarded + export_songs + Session.start for real, against a fake machine."""

    SONG = os.path.join(os.path.dirname(HERE), "tests", "songs", "diag", "songs", "silence.rbs")

    class Client(PrefsRestoreTest.FakeClient):
        def __init__(self, pids=(), bad_upload=False, start_fails=False):
            super().__init__()
            self.pids = list(pids)
            self.bad_upload, self.start_fails = bad_upload, start_fails
            self.prf_writes = []

        def upload(self, local, remote):
            if remote == rb_export.PRF_REMOTE:
                self.prf_writes.append(remote)
                self.files[remote] = "bad" if self.bad_upload else rb_export.sha256_file(local)

        def list(self, remote=""):
            return []

        def delete(self, remote, recursive=False):
            pass

        def screenshot(self, window=None):
            return b""

        def output(self, pid):
            program, args = self.last
            if program == "cmd.exe" and args[:2] == ["/c", "start"]:
                if self.start_fails:
                    raise RuntimeError("start failed")
                self.pids = [77]
            if program == "taskkill":
                self.pids = []
            return super().output(pid)

    def run_real(self, client, fail_after_start=False):
        import types
        prf = os.path.join(tempfile.mkdtemp(), "user.prf")
        with open(prf, "wb") as f:
            f.write(b"run prefs")
        args = types.SimpleNamespace(
            songs=[self.SONG], remote_dir="rb/run", restore_prf=prf, startup_sound=None,
            reuse_process=False, startup_boxes=1, start_wait=0.0, test_fail=False)
        m = rb_export.Machine(client, PrefsRestoreTest.FakePermits(client))
        m.restore_wait = 0
        orig = rb_export.Machine.wait_frame
        if fail_after_start:
            def boom(self, box, timeout=15):
                client.files[rb_export.PRF_REMOTE] = "rewritten by the app"
                raise rb_export.Failure("boom")
            rb_export.Machine.wait_frame = boom
        try:
            rc = rb_export.run_guarded(args, client, m, m.permits, tempfile.mkdtemp(), "t")
        except Exception as e:
            rc = type(e).__name__
        finally:
            rb_export.Machine.wait_frame = orig
        return rc, m

    def test_user_app_running_prefs_untouched(self):
        c = self.Client(pids=[5])
        rc, m = self.run_real(c)
        self.assertEqual(rc, 1)
        self.assertEqual(c.prf_writes, [])
        self.assertEqual(c.files[rb_export.PRF_REMOTE], "user")
        self.assertEqual(m.permits.released_with, "user")
        self.assertEqual(c.pids, [5])

    def test_upload_hash_mismatch_restored(self):
        c = self.Client(bad_upload=True)
        rc, m = self.run_real(c)
        self.assertEqual(rc, 1)
        self.assertEqual(m.permits.released_with, rb_export.PRF_SAVED_SHA256)

    def test_start_failure_restored(self):
        c = self.Client(start_fails=True)
        rc, m = self.run_real(c)
        self.assertEqual(rc, "RuntimeError")
        self.assertEqual(m.permits.released_with, rb_export.PRF_SAVED_SHA256)

    def test_failure_after_start_restored(self):
        c = self.Client()
        rc, m = self.run_real(c, fail_after_start=True)
        self.assertEqual(rc, 1)
        self.assertEqual(c.pids, [])   # the app we started was stopped
        self.assertEqual(m.permits.released_with, rb_export.PRF_SAVED_SHA256)


if __name__ == "__main__":
    unittest.main()

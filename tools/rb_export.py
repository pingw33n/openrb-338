#!/usr/bin/python3
"""Export songs with the original ReBirth's File > Export Loop as Audio File, on the test machine.

    rb_export.py [options] SONG.rbs... -o OUTDIR

For each song the runner uploads it to the machine, starts ReBirth, opens the song, exports the
loop as a WAV through the menus and dialogs, waits until the file is complete, downloads it and
writes a JSON sidecar (OUTDIR/<stem>.wav + OUTDIR/<stem>.json) with the protocol and the timings.

How it drives the app: the rcs server's `input` (keys, typed text) and `screenshot` (also used to
tell whether a dialog is up: a window screenshot of a missing window fails). The machine has no UI
automation library; the route is the one of docs/test_machine.md sections 2-4. Dialogs are found by
their titles ("Open Song:", "Export audio as:").

Permits: it takes all the permits of the locky semaphore `redc-win-run` (owner: --owner, default
the git branch), refreshes them while it works and releases them at the end, also on failure.
It stops only the Rebirth.exe it started (found by PID), never another one; if a Rebirth.exe is
already running it refuses to start.

Protocol options (docs/export_state.md section 9):
    --restore-prf [LOCAL]   restore the preferences file before every start, from LOCAL or from the
                            machine's C:\\re\\rb\\prf_before.prf (docs/test_machine.md section 5)
    --startup-sound on|off  set the "Play Mod Startup Sound" byte of the restored file
    --wait-after-open S     seconds to wait after the song is open (default 10, the protocol's T)
    --play-seconds S        press Space, play S seconds, press Space (stop) before the export
    --exports K             export K times in a row in the same session (default 1)
    --reuse-process         one app for all the songs given (default: a fresh process per song)

Exit status: 0 when every export is complete and verified, 1 otherwise (a screenshot of the
screen is saved as OUTDIR/<stem>.fail.bmp, the app it started is stopped, the permits released).
"""
import argparse
import hashlib
import json
import os
import shutil
import signal
import struct
import subprocess
import sys
import time

TOOLS = os.environ.get("RCS_TOOLS", os.path.expanduser("~/dev/dcre/common/tools"))
sys.path.insert(0, TOOLS)
import rcs_client  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import rbs  # noqa: E402

SEMAPHORE = "redc-win-run"
APP_DIR = "C:/ReBirth RB-338"
APP_EXE = "Rebirth.exe"
FRAME_TITLE = "ReBirth RB-338"
PRF_REMOTE = "C:/Users/pingw33n/AppData/Local/VirtualStore/Windows/ReBirth RB-338 2.prf"
PRF_SAVED = "C:/re/rb/prf_before.prf"
PRF_SAVED_SHA256 = "6309526e1574414dd3c093f97fedb5e7978428c789db94099c3af81ee8cbd51e"
# Offset of the "Play Mod Startup Sound" byte in the preferences file: 1 = checked (the saved copy),
# 0 = cleared. Found by clearing the checkbox in Preferences and diffing the file written at exit
# (task-008): the only change besides the recent-file list.
STARTUP_SOUND_OFFSET = 0x20A
EXPORT_DOWNS = 6   # File menu: Song Information is the 5th item, Export the 6th (Save is greyed)
WAV_HEADER = 44


class Failure(Exception):
    pass


def now():
    return time.time()


def log(msg):
    print("[%s] %s" % (time.strftime("%H:%M:%S"), msg), file=sys.stderr, flush=True)


def win_path(p):
    return p.replace("/", "\\")


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for b in iter(lambda: f.read(1 << 20), b""):
            h.update(b)
    return h.hexdigest()


def export_frames(song):
    """Frames of the export: docs/initial_investigation.md section 3, step 4 (double accumulation)."""
    g = song.glob
    tempo, pos, end = float(g.tempo), float(g.loop_start), float(g.loop_end)
    k = 192.0 / 2646000000.0
    total = 0
    while True:
        n = int((end - pos) / (tempo * k))
        last = n < 2048
        if not last:
            n = 2048
        total += max(n, 0)
        if last:
            return total
        pos += tempo * n * k


def check_wav(path, frames):
    """RIFF consistency of a downloaded export; returns a dict, raises Failure."""
    with open(path, "rb") as f:
        data = f.read()
    if len(data) < WAV_HEADER or data[:4] != b"RIFF" or data[8:12] != b"WAVE":
        raise Failure("not a RIFF/WAVE file")
    if struct.unpack("<I", data[4:8])[0] != len(data) - 8:
        raise Failure("RIFF size %d != file size - 8 (%d)" % (struct.unpack("<I", data[4:8])[0], len(data) - 8))
    pos, fmt, dsize = 12, None, None
    while pos + 8 <= len(data):
        tag, size = data[pos:pos + 4], struct.unpack("<I", data[pos + 4:pos + 8])[0]
        if tag == b"fmt ":
            fmt = struct.unpack("<HHIIHH", data[pos + 8:pos + 24])
        elif tag == b"data":
            dsize = size
            if pos + 8 + size != len(data):
                raise Failure("data chunk ends at %d, file is %d bytes" % (pos + 8 + size, len(data)))
            break
        pos += 8 + size + (size & 1)
    if fmt is None or dsize is None:
        raise Failure("no fmt or data chunk")
    if fmt[0] != 1 or fmt[1] != 2 or fmt[2] != 44100 or fmt[5] != 16:
        raise Failure("unexpected format %r (want PCM, 2 ch, 44100, 16 bit)" % (fmt,))
    if dsize != 4 * frames:
        raise Failure("data is %d bytes = %d frames, the length formula gives %d frames"
                      % (dsize, dsize // 4, frames))
    return {"format": "pcm_s16le", "channels": 2, "rate": 44100, "frames": dsize // 4,
            "header_bytes": len(data) - dsize}


class Permits:
    def __init__(self, owner):
        self.owner = owner
        self.held = False

    def _locky(self, *args):
        p = subprocess.run(["locky"] + list(args), capture_output=True, text=True)
        return p.returncode, p.stdout

    def acquire(self, timeout=3600, lease=1800):
        rc, out = self._locky("list")
        n = json.loads(out)["semaphores"][SEMAPHORE]["permits"]
        log("acquiring %d permits of %s as %s" % (n, SEMAPHORE, self.owner))
        rc, out = self._locky("acquire", SEMAPHORE, str(n), "--owner", self.owner,
                              "--acquire-timeout", str(timeout), "--lease-timeout", str(lease))
        if rc != 0:
            raise Failure("could not acquire the permits: " + out.strip())
        self.held = True
        self.lease = lease
        self.n = n
        self.last = now()

    def refresh(self):
        if self.held:
            rc, out = self._locky("refresh", SEMAPHORE, "--owner", self.owner, "--lease-timeout", str(self.lease))
            if rc != 0:
                raise Failure("could not refresh the permits (lease lost?): " + out.strip())
            self.last = now()

    def release(self):
        if self.held:
            rc, out = self._locky("release", SEMAPHORE, "all", "--owner", self.owner)
            self.held = False
            log("released the permits (%s)" % ("ok" if rc == 0 else out.strip()))


class Machine:
    """The test machine: files, processes and the app's UI through rcs."""

    def __init__(self, client, permits):
        self.c = client
        self.permits = permits

    # ---- processes
    def run(self, program, args=(), timeout=60, cwd=""):
        rec = self.c.exec(program, list(args), cwd=cwd)
        pid = rec["id"]
        t0 = now()
        while True:
            r = self.c.proc(pid)
            if not r["running"]:
                break
            if now() - t0 > timeout:
                self.c.kill(pid)
                raise Failure("%s timed out" % program)
            time.sleep(0.2)
        out, _ = self.c.output(pid)
        return r["exit_code"], out.decode("cp437", "replace")

    def rebirth_pids(self):
        rc, out = self.run("tasklist", ["/FI", "IMAGENAME eq " + APP_EXE, "/FO", "CSV", "/NH"])
        pids = []
        for line in out.splitlines():
            parts = [x.strip('"') for x in line.split('","')]
            if len(parts) >= 2 and parts[0].strip('"').lower() == APP_EXE.lower():
                pids.append(int(parts[1]))
        return pids

    def start_app(self):
        before = self.rebirth_pids()
        if before:
            raise Failure("a %s is already running (PIDs %s): not started by this run, not touching it"
                          % (APP_EXE, before))
        self.run("cmd.exe", ["/c", "start", "", "/D", win_path(APP_DIR), win_path(APP_DIR + "/" + APP_EXE)])
        t0 = now()
        while now() - t0 < 30:
            pids = self.rebirth_pids()
            if pids:
                return pids[0]
            time.sleep(0.5)
        raise Failure("%s did not start" % APP_EXE)

    def stop_app(self, pid):
        """Stops the app we started: a plain taskkill first (clean exit, rewrites the prefs), then /F."""
        if pid not in self.rebirth_pids():
            return "gone"
        self.run("taskkill", ["/PID", str(pid)])
        for _ in range(30):
            if pid not in self.rebirth_pids():
                return "taskkill"
            time.sleep(0.5)
        self.run("taskkill", ["/F", "/PID", str(pid)])
        time.sleep(1)
        if pid in self.rebirth_pids():
            raise Failure("pid %d is still running after taskkill /F" % pid)
        return "taskkill /F"

    # ---- UI
    def keys(self, events, window=FRAME_TITLE):
        self.c.input([rcs_client.parse_event(e) for e in events], window)

    def text(self, s, window):
        self.c.input([{"type": "text", "text": s}], window)

    def window_up(self, title):
        try:
            self.c.screenshot(title)
            return True
        except rcs_client.RcsError:
            return False

    def wait_window(self, title, up=True, timeout=15):
        t0 = now()
        while now() - t0 < timeout:
            self.permits_tick()
            if self.window_up(title) == up:
                return now() - t0
            time.sleep(0.3)
        raise Failure("window %r did not %s within %d s" % (title, "appear" if up else "go away", timeout))

    def permits_tick(self):
        if now() - self.permits.last > 300:
            self.permits.refresh()

    def sleep(self, seconds):
        """Sleeps, refreshing the permits' lease on the way."""
        end = now() + seconds
        while True:
            self.permits_tick()
            left = end - now()
            if left <= 0:
                return
            time.sleep(min(left, 5.0))

    def screenshot_to(self, path):
        try:
            with open(path, "wb") as f:
                f.write(self.c.screenshot())
            return path
        except Exception as e:  # best effort
            log("screenshot failed: %s" % e)
            return None


def make_prf(args):
    """Local file with the preferences to upload, or None."""
    if not args.restore_prf and args.startup_sound is None:
        return None, None
    import tempfile
    local = args.restore_prf if isinstance(args.restore_prf, str) else None
    tmp = os.path.join(tempfile.mkdtemp(prefix="rb_export_"), "prefs.prf")
    if local is None:
        rcs_client.Client.from_config().download(PRF_SAVED, tmp)
        if sha256_file(tmp) != PRF_SAVED_SHA256:
            raise Failure("the machine's %s has sha256 %s, not the documented %s (docs/test_machine.md 5)"
                          % (PRF_SAVED, sha256_file(tmp), PRF_SAVED_SHA256))
    else:
        shutil.copyfile(local, tmp)
    if args.startup_sound is not None:
        with open(tmp, "rb") as f:
            b = bytearray(f.read())
        if b[STARTUP_SOUND_OFFSET] not in (0, 1):
            raise Failure("--startup-sound: byte 0x%X of the preferences is %d, not a flag"
                          % (STARTUP_SOUND_OFFSET, b[STARTUP_SOUND_OFFSET]))
        b[STARTUP_SOUND_OFFSET] = 1 if args.startup_sound == "on" else 0
        with open(tmp, "wb") as f:
            f.write(bytes(b))
    return tmp, sha256_file(tmp)


class Session:
    def __init__(self, m, args, outdir, run_id):
        self.m, self.args, self.outdir, self.run_id = m, args, outdir, run_id
        self.pid = None
        self.timeline = []
        self.t_start = None
        self.prf_sha = None

    def mark(self, what):
        t = now()
        self.timeline.append({"t": round(t - self.t_start, 3), "step": what})
        log(what)

    # ---- the app
    def start(self, prf_local, prf_sha):
        m = self.m
        self.t_start = now()
        self.timeline = []
        if prf_local:
            m.c.upload(prf_local, PRF_REMOTE)
            got = m.c.hash(PRF_REMOTE)["hash"]
            if got != prf_sha:
                raise Failure("preferences upload hash mismatch")
            self.prf_sha = prf_sha
            self.mark("preferences restored (sha256 %s)" % prf_sha[:12])
        self.pid = m.start_app()
        self.mark("app started, pid %d" % self.pid)
        m.sleep(self.args.start_wait)
        m.keys(["key:enter"])  # "Failed opening MIDI input" box
        self.mark("MIDI box dismissed")
        m.sleep(1)

    def stop(self):
        if self.pid is None:
            return None
        how = self.m.stop_app(self.pid)
        self.mark("app stopped (%s)" % how)
        self.pid = None
        return how

    def open_song(self, remote_song_win, info_box):
        m = self.m
        m.keys(["keydown:ctrl", "key:o", "keyup:ctrl"])
        m.wait_window("Open Song:", True)
        m.text(remote_song_win, "Open Song:")
        m.keys(["key:enter"], "Open Song:")
        m.wait_window("Open Song:", False)
        self.mark("song opened")
        m.sleep(1.5)
        if info_box:  # USRI show_on_open: a modal box with the song's text
            m.keys(["key:enter"])
            self.mark("song info box dismissed")
            m.sleep(0.5)

    def export(self, remote_wav_win, remote_wav, retries_out):
        m = self.m
        menu = ["keydown:alt", "key:f", "keyup:alt"] + ["key:down"] * EXPORT_DOWNS + ["key:enter"]
        if self.args.test_fail:  # a forced dialog timeout, for testing the failure path
            m.wait_window("rb_export forced timeout", True, timeout=3)
        for attempt in range(2):
            m.keys(menu)
            try:
                m.wait_window("Export audio as:", True, timeout=6 if attempt == 0 else 15)
                break
            except Failure:
                if attempt == 1:
                    raise Failure("the export dialog did not open")
                # Probably a modal box took the menu keys. Close it (Enter, as at the song info box)
                # and replay the menu once.
                retries_out.append("export dialog not up after the menu keys, Enter sent, retried once")
                log(retries_out[-1])
                m.keys(["key:enter"])
                m.sleep(0.5)
        self.mark("export dialog up")
        m.text(remote_wav_win, "Export audio as:")
        m.keys(["key:enter"], "Export audio as:")
        self.mark("export started")

    def wait_file(self, remote, remote_dir, frames):
        m = self.m
        want = WAV_HEADER + 4 * frames
        t0 = now()
        last_size, stable_since = -1, now()
        while True:
            m.permits_tick()
            ent = [e for e in m.c.list(remote_dir) if e["name"].lower() == os.path.basename(remote).lower()]
            size = ent[0]["size"] if ent else -1
            if size != last_size:
                last_size, stable_since = size, now()
            if size == want and now() - stable_since >= 1.5:
                return size
            if size >= 0 and now() - stable_since > 20:
                raise Failure("export file stuck at %d bytes (want %d)" % (size, want))
            if now() - t0 > self.args.export_timeout:
                raise Failure("export timed out, file is %d bytes (want %d)" % (size, want))
            m.sleep(0.5)


def sidecar_write(path, d):
    with open(path, "w") as f:
        json.dump(d, f, indent=1)
        f.write("\n")


def export_songs(args, client, m, outdir, owner):
    prf_local, prf_sha = make_prf(args)
    remote_dir = args.remote_dir.strip("/")
    win_dir = "C:\\re\\" + win_path(remote_dir)
    results = []
    session = None
    current_stem = None
    try:
        for si, song_path in enumerate(args.songs):
            stem = os.path.splitext(os.path.basename(song_path))[0]
            current_stem = stem
            song = rbs.read_song(song_path)
            if not song.glob.song_mode:
                # "Export Loop as Audio File..." is greyed in pattern mode (task-008, tested with a built song)
                raise Failure("%s is in pattern mode (GLOB song_mode 0): the app's Export item is disabled"
                              % song_path)
            frames = export_frames(song)
            song_sha = sha256_file(song_path)
            safe = "song%d.rbs" % si
            remote_song = "%s/%s" % (remote_dir, safe)
            m.c.upload(song_path, remote_song)
            if session is None or not args.reuse_process:
                if session is not None:
                    session.stop()
                session = Session(m, args, outdir, owner)
                session.start(prf_local, prf_sha)
            else:
                session.mark("next song in the same process")
            retries = []
            session.open_song(win_dir + "\\" + safe, bool(song.usri.show_on_open))
            t_open = now()
            m.sleep(args.wait_after_open)
            session.mark("waited %.1f s after open" % args.wait_after_open)
            play = None
            if args.play_seconds:
                m.keys(["key:space"])
                session.mark("play pressed")
                m.sleep(args.play_seconds)
                m.keys(["key:space"])
                session.mark("stop pressed after %.1f s" % args.play_seconds)
                play = args.play_seconds
                m.sleep(0.5)
            for k in range(1, args.exports + 1):
                suffix = "" if args.exports == 1 else ".%d" % k
                name = "%s_%d%s.wav" % (args.tag or "x", si, suffix)
                remote_wav = "%s/%s" % (remote_dir, name)
                for e in m.c.list(remote_dir):
                    if e["name"].lower() == name.lower():
                        m.c.delete(remote_wav)
                t_exp = now()
                session.export(win_dir + "\\" + name, remote_wav, retries)
                size = session.wait_file(remote_wav, remote_dir, frames)
                session.mark("export %d complete, %d bytes" % (k, size))
                m.sleep(1.0)  # the progress box closes
                local_wav = os.path.join(outdir, stem + suffix + ".wav")
                m.c.download(remote_wav, local_wav)
                info = check_wav(local_wav, frames)
                side = {
                    "tool": "tools/rb_export.py", "wav": os.path.basename(local_wav),
                    "wav_sha256": sha256_file(local_wav), "wav_bytes": os.path.getsize(local_wav),
                    "wav_check": info, "expected_frames": frames,
                    "song": {"path": song_path, "sha256": song_sha, "bytes": os.path.getsize(song_path),
                             "tempo": song.glob.tempo, "loop_start": song.glob.loop_start,
                             "loop_end": song.glob.loop_end},
                    "protocol": {
                        "fresh_process": not args.reuse_process, "reuse_process": args.reuse_process,
                        "export_index": k, "exports_in_session": args.exports,
                        "play_seconds_before_export": play, "wait_after_open_s": args.wait_after_open,
                        "start_wait_s": args.start_wait,
                        "restore_prf": bool(prf_local), "prf_sha256": session.prf_sha,
                        "startup_sound": args.startup_sound, "info_box_dismissed": bool(song.usri.show_on_open), "song_index_in_session": si if args.reuse_process else 0,
                    },
                    "timing": {"export_seconds": round(now() - t_exp, 2),
                               "open_to_export_start_s": round(t_exp - t_open, 2),
                               "timeline": session.timeline},
                    "machine": {"pid": session.pid, "retries": retries,
                                "remote_file": remote_wav, "time": client.info()["time"]},
                    "permits_owner": owner, "argv": sys.argv[1:],
                }
                sidecar_write(os.path.join(outdir, stem + suffix + ".json"), side)
                results.append(local_wav)
                log("OK %s" % local_wav)
        if session is not None:
            session.stop()
            session = None
        return results
    except BaseException as e:
        if not isinstance(e, Failure):
            log("unexpected %s: %s" % (type(e).__name__, e))
        fail = os.path.join(outdir, (current_stem or "run") + ".fail.bmp")
        shot = m.screenshot_to(fail)
        log("FAILED: %s (screenshot: %s)" % (e, shot))
        if session is not None:
            try:
                session.stop()
            except Exception as e2:
                log("stopping the app failed: %s" % e2)
        if isinstance(e, Failure):
            return None
        raise


def default_owner():
    try:
        return subprocess.run(["git", "-C", ROOT, "branch", "--show-current"], capture_output=True,
                              text=True).stdout.strip() or "rb_export"
    except Exception:
        return "rb_export"


def main(argv):
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("songs", nargs="+", metavar="SONG.rbs")
    ap.add_argument("-o", "--outdir", default=".")
    ap.add_argument("--owner", help="locky owner (default: the git branch)")
    ap.add_argument("--remote-dir", default="rb/run", help="work folder under the rcs root (default rb/run)")
    ap.add_argument("--tag", help="prefix of the remote file names (default x)")
    ap.add_argument("--restore-prf", nargs="?", const=True, default=False, metavar="LOCAL",
                    help="restore the preferences file before each start (from LOCAL, or the machine's saved copy)")
    ap.add_argument("--startup-sound", choices=["on", "off"])
    ap.add_argument("--wait-after-open", type=float, default=10.0, metavar="S")
    ap.add_argument("--play-seconds", type=float, default=0.0, metavar="S")
    ap.add_argument("--exports", type=int, default=1, metavar="K")
    ap.add_argument("--reuse-process", action="store_true", help="one app process for all the songs")
    ap.add_argument("--start-wait", type=float, default=4.0, help="seconds from start to the MIDI box key")
    ap.add_argument("--test-fail", action="store_true", help="force a dialog timeout (tests the failure path)")
    ap.add_argument("--export-timeout", type=float, default=900.0)
    args = ap.parse_args(argv)
    outdir = os.path.abspath(args.outdir)
    os.makedirs(outdir, exist_ok=True)
    owner = args.owner or default_owner()
    signal.signal(signal.SIGTERM, lambda *_: sys.exit(143))

    client = rcs_client.Client.from_config()
    permits = Permits(owner)
    m = Machine(client, permits)
    rc = 1
    try:
        permits.acquire()
        res = export_songs(args, client, m, outdir, owner)
        rc = 0 if res is not None else 1
    except Failure as e:
        log("FAILED: %s" % e)
    finally:
        permits.release()
    return rc


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

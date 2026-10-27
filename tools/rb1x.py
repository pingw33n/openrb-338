#!/usr/bin/python3
"""ReBirth 1.x songs: a Standard MIDI File with the .rbs extension, as Rebirth.exe 2.01 imports it.

    rb1x.py --dump SONG.rbs      the decoded globals and the message list
    rb1x.py --survey DIR         every 1.x song of DIR: version, events, unmapped controllers

The import is traced in docs/song_load.md section 12 (exe sub_430BA0, sub_4304C5, sub_431747).
`Song1x.load_messages()` gives the song messages the exe sends after msg 52, in its order: msg 36,
the msg-41 blocks of the mixer, delay, distortion, PCF, the two 303s and the 808, the 909's
`4/74 {0, 0}`, the msg-37 batches as the exe's nine track buffers flush (sub_4300E0 at 1284
events, then tracks 0..8 at the end), and msg 32. `tools/rbhost/rbsmsg.py` writes them as a
message file; `crates/rbs/src/rb1x.rs` and `crates/rb-engine/src/load.rs` are the Rust side.
Stdlib only.
"""
import os
import re
import struct
import sys

STANDARD_MOD = "Standard ReBirth"
DECODED = 0x4000                   # sub_4304C5: the decoded size (its copy-protected formula's intended value)
BATCH = 1284                       # sub_4300E0: a track buffer is sent when it holds 1284 events


class Error(Exception):
    pass


def is_1x(data):
    return data[:4] == b"MThd"


# ------------------------------------------------------------------ the MIDI file (sub_4A0xxx)


def _vlq(b, p):
    v = 0
    while True:
        if p >= len(b):
            raise Error("truncated variable-length number")
        c = b[p]
        p += 1
        v = (v << 7) | (c & 0x7F)
        if not c & 0x80:
            return v & 0xFFFFFFFF, p


def _data_bytes(status):
    """byte_4D2A60[4 * ((status & 0x7F) >> 4)]: the data bytes of a channel message."""
    return 1 if status & 0xF0 in (0xC0, 0xD0) else 2


def parse_midi(data):
    """(format, [track events]); an event is (abs time, kind, a, b): ('cc', ctrl, value),
    ('sysex', bytes), ('meta', type, bytes), ('other', status, bytes). Times are absolute in file
    units; the status is kept across events and tracks (running status, sub_4A0E8A)."""
    if data[:4] != b"MThd":
        raise Error("not a MIDI file")
    hlen = struct.unpack(">I", data[4:8])[0]
    fmt, ntracks = struct.unpack(">HH", data[8:12])
    p = 8 + hlen
    tracks = []
    status = 0
    while p + 8 <= len(data):
        tag, ln = data[p:p + 4], struct.unpack(">I", data[p + 4:p + 8])[0]
        body = data[p + 8:p + 8 + ln]
        p += 8 + ln
        if tag != b"MTrk":
            continue
        evs, q, t = [], 0, 0
        while q < len(body):
            d, q = _vlq(body, q)
            t = (t + d) & 0xFFFFFFFF
            if q >= len(body):
                raise Error("truncated event")
            if body[q] & 0x80:
                status = body[q]
                q += 1
            if status == 0xFF:
                ty = body[q]
                n, q = _vlq(body, q + 1)
                evs.append((t, "meta", ty, body[q:q + n]))
                q += n
                if ty == 0x2F:
                    break
            elif status in (0xF0, 0xF7):
                n, q = _vlq(body, q)
                evs.append((t, "sysex", body[q:q + n], None))
                q += n
            else:
                n = _data_bytes(status)
                b = body[q:q + n]
                q += n
                if status & 0xF0 == 0xB0:
                    evs.append((t, "cc", b[0], b[1]))
                else:
                    evs.append((t, "other", status, b))
        tracks.append(evs)
    if len(tracks) < ntracks:
        raise Error("%d tracks of %d" % (len(tracks), ntracks))
    return fmt, tracks[:ntracks]


def file_version(tracks):
    """sub_431341: the first 'File version %s' text event (FF 02) of track 0; major.minor < 100."""
    for ev in tracks[0]:
        if ev[1] == "meta" and ev[2] == 2:
            m = re.match(rb"File version (\S+)", ev[3][:200])
            if not m:
                raise Error("bad version text")
            parts = m.group(1).split(b".")
            num = lambda s: int(re.match(rb"\d*", s).group(0) or b"0")
            if len(parts) < 2:
                raise Error("bad version %r" % m.group(1))
            major, minor = num(parts[0]), num(parts[1])
            if major >= 100 or minor >= 100:
                raise Error("bad version %r" % m.group(1))
            return major, minor
    return 2, 0                    # sub_47D768(2, 0): the default when there is no text


def decode_sysex(sx):
    """sub_4304C5 + sub_4665CD/sub_4663F0: F0 00 00 66, [5] != 0, [6] == 1, [7] == 0, then 8-byte
    groups of 7 data bytes whose high bits are in the 8th byte (0x40 for the first). 0x4000 bytes."""
    v = b"\xf0" + bytes(sx)
    if len(v) < 8 or v[1:4] != b"\x00\x00\x66" or not v[5] or v[6] != 1 or v[7]:
        raise Error("not a ReBirth song sysex")
    src = v[8:]
    out = bytearray()
    p = 0
    while len(out) < DECODED:
        g = src[p:p + 8].ljust(8, b"\0")
        for k in range(7):
            if len(out) < DECODED:
                out.append(g[k] | (0x80 if g[7] & (0x40 >> k) else 0))
        p += 8
    return bytes(out)


# ------------------------------------------------------------------ the decoded block (sub_431747)


def _u32(b, o):
    return struct.unpack(">I", b[o:o + 4])[0]


def tom_tune(v):
    """sub_43132C: the 1.x 808 tom tunings, v/2 + 64."""
    return v // 2 + 64


PCF_TARGET = {0: 1, 1: 2, 2: 3, 3: 0}   # 0x431920, 0x430EFF: 303 #1, 303 #2, 808, off


class Block:
    """The decoded 16 KB block and the packers' inputs built from it."""

    def __init__(self, d):
        if d[5:9] != b"TinY":
            raise Error("no TinY tag")
        self.d = d

    def globals(self):
        d = self.d
        start, length = _u32(d, 10), _u32(d, 14)
        out = bytearray(23)
        out[0] = d[0x12] != 0              # song mode
        out[3] = d[0x2E] != 0              # loop on
        out[4:8] = d[0x2A:0x2E]            # tempo
        out[12:16] = struct.pack(">I", 768 * start & 0xFFFFFFFF)
        out[16:20] = struct.pack(">I", 768 * (start + length) & 0xFFFFFFFF)
        out[20] = 64                       # sub_420A81's shuffle
        out[21] = 1                        # 0x43186A: the sound model, Vintage
        out[22] = 1
        return bytes(out)

    def mixer(self):
        d = self.d
        out = bytearray(29)
        out[0] = d[9]
        out[2] = PCF_TARGET.get(d[0x40], 0)
        for j in range(4):
            if j < 3:
                r = d[0x100 + 32 * j:0x120 + 32 * j]
                ch = [r[0x17], r[2], r[3], r[4], int(d[0x37] == j)]
            else:
                ch = [1, 100, 64, 0, 0]    # sub_423560
            out[5 + 6 * j:10 + 6 * j] = bytes(ch)
        return bytes(out)

    def delay(self):
        d = self.d
        return bytes([d[0x35], d[0x2F] & 0x7F, int(d[0x2F] & 0x80 != 0), d[0x33], d[0x34], 0])

    def dist(self):
        d = self.d
        return bytes([d[0x38], d[0x36], 0, 0])

    def pcf(self):
        d = self.d
        return bytes([d[0x3F]]) + d[0x39:0x3F] + b"\0"

    def _lengths(self, rec):
        out = []
        for k in range(16):
            b = rec[7 + k]
            out += [(b >> 4) + 1, (b & 15) + 1]
        return out

    def tb303(self, j):
        d = self.d
        rec = d[0x100 + 32 * j:0x120 + 32 * j]
        p = d[352 + 4096 * j:352 + 4096 * (j + 1)]
        out = bytearray(3148)
        out[0] = rec[5] == 0
        out[1] = rec[1]
        out[2:9] = p[16:23]
        lengths = self._lengths(rec)
        for pat in range(32):
            o = 12 + 98 * pat
            out[o], out[o + 1] = 0, lengths[pat]
            for s in range(16):
                st = p[30 + 68 * pat + 4 * s:34 + 68 * pat + 4 * s]
                note, fl = st[0], st[2]
                up = fl >> 2 & 1
                if note > 12:
                    up, note = 1, note - 12
                out[o + 2 + 6 * s:o + 8 + 6 * s] = bytes([note, fl & 1, fl >> 1 & 1, up, fl >> 3 & 1, st[3]])
        return bytes(out)

    def tr808(self):
        d = self.d
        rec = d[0x100 + 64:0x120 + 64]
        p = d[352 + 8192:352 + 12288]
        out = bytearray(6240)
        out[0] = rec[5] == 0
        out[1] = rec[1]
        out[2:29] = p[16:43]
        for i in (10, 13, 16):
            out[i] = tom_tune(p[i + 14])
        lengths = self._lengths(rec)
        for pat in range(32):
            o = 32 + 194 * pat
            out[o], out[o + 1] = 0, lengths[pat]
            src = p[48 + 64 * pat:112 + 64 * pat]
            v = []
            for b in src:              # sub_43285C: four 2-bit values per byte, high first
                v += [b >> 6 & 3, b >> 4 & 3, b >> 2 & 3, b & 3]
            for s in range(16):
                for i in range(12):
                    out[o + 2 + 12 * s + i] = v[s + 16 * i]
        return bytes(out)


# ------------------------------------------------------------------ the controller tracks (sub_430BA0)


def map_cc(track, cc):
    """sub_43067F (device) and sub_430847 (controller) for MIDI track `track`; None when unmapped."""
    if track in (2, 3, 4):
        dev = track - 1
        if cc in (7, 10, 12):
            return 0, 6 * (track - 2) + {7: 6, 10: 7, 12: 8}[cc]
        if cc == 14:
            return dev, 1
        if cc == 15:
            return dev, 0
        if 16 <= cc <= (22 if track < 4 else 42):
            return dev, cc - 14
        return None
    if track == 5:
        m = {16: (6, 1), 18: (6, 0), 19: (5, 1), 20: (5, 3), 21: (5, 4), 22: (5, 0), 23: (5, 2),
             24: (7, 1), 25: (7, 2), 26: (7, 3), 27: (7, 4), 28: (7, 5), 29: (7, 6), 30: (7, 0)}
        return m.get(cc)
    return None


class Buffers:
    """The nine track buffers (sub_43005E, 0x282C bytes): events go in with sub_4300E0 and leave
    as one msg 37 when 1284 are held (or at the end, sub_43035E)."""

    def __init__(self, out):
        self.out = out
        self.events = [[] for _ in range(9)]
        self.sent = [0] * 9

    def add(self, track, tick, ctrl, value):
        self.events[track].append((tick & 0xFFFFFFFF, ctrl & 0xFF, value & 0xFF))
        if len(self.events[track]) == BATCH:
            self.flush(track)

    def flush(self, track):
        ev = self.events[track]
        if not ev:
            return
        first = self.sent[track]
        body = bytearray([track]) + struct.pack(">II", first, first + len(ev))
        for t, c, v in ev:
            body += bytes([99]) + struct.pack(">I", t) + bytes([c, v])
        self.out.append(song_msg(37, bytes(body)))
        self.sent[track] += len(ev)
        self.events[track] = []


def song_msg(msg, payload=b""):
    return bytes([78, 0, msg]) + payload


def device_msg(dev, sub, payload):
    return song_msg(41, bytes([dev, sub]) + payload)


class Song1x:
    """A 1.x song as the exe imports it. `glob` has the fields the tools read from an RBS song."""

    def __init__(self, data):
        self.format, self.tracks = parse_midi(data)
        if self.format != 1:
            raise Error("MIDI format %d (sub_430BA0 wants 1)" % self.format)
        self.version = file_version(self.tracks)
        if self.version < (3, 1):          # sub_47DA1C against CVersion(3, 1) at 0x4DAD60
            raise Error("unsupported 1.x file version %d.%d" % self.version)
        self.unmapped = []
        self._messages = self._import()
        g = self.block.globals()
        self.glob = type("Glob", (), {})()
        self.glob.song_mode = g[0]
        self.glob.loop_on = g[3]
        self.glob.tempo = _u32(g, 4)
        self.glob.loop_start = _u32(g, 12)
        self.glob.loop_end = _u32(g, 16)
        self.glob.shuffle = g[20]
        self.glob.mod_name = STANDARD_MOD   # the built-in mod, dword_4DA784 (0x425E17)
        # the song information (sub_42FFEC at 12640; 0x431078 copies the show-on-open byte)
        self.usri = type("Usri", (), {})()
        self.usri.show_on_open = int(self.block.d[12640 + 343] != 0)

    def _import(self):
        out = []
        buf = Buffers(out)
        buf.add(4, 0, 0, 0)                # 0x430CFA: the 909 off at tick 0
        self.block = None
        for ti in range(1, len(self.tracks)):
            for t, kind, a, b in self.tracks[ti]:
                tick = 24 * t                # 0x430D72: division forced to 8, 192 / 8 = 24
                if kind == "cc":
                    if ti == 5 and a == 17:      # 0x430E47: the distortion's channel
                        buf.add(0, tick, 9, int(b == 0))
                        buf.add(0, tick, 15, int(b == 1))
                        buf.add(0, tick, 21, int(b == 2))
                        buf.add(0, tick, 27, 0)
                    elif ti == 5 and a == 31:    # 0x430ED2: the PCF's channel
                        buf.add(0, tick, 2, PCF_TARGET.get(b, 0))
                    else:
                        m = map_cc(ti, a)
                        if m is None:
                            self.unmapped.append((ti, a, t))
                            raise Error("track %d: controller %d is not mapped (the exe indexes "
                                        "its track buffers with -1)" % (ti, a))
                        dev, ctrl = m
                        if dev == 3 and ctrl in (10, 13, 16):
                            b = tom_tune(b)
                        buf.add(dev, tick, ctrl, b)
                elif kind == "sysex":
                    self.block = Block(decode_sysex(a))
                    out += self._device_messages()
        if self.block is None:
            raise Error("no song sysex")
        for t in range(9):
            buf.flush(t)
        out.append(song_msg(32))
        return out

    def _device_messages(self):
        b = self.block
        return [song_msg(36, b.globals()),
                device_msg(0, 143, b.mixer()),
                device_msg(5, 86, b.delay()),
                device_msg(6, 94, b.dist()),
                device_msg(7, 116, b.pcf()),
                device_msg(1, 39, b.tb303(0)),
                device_msg(2, 39, b.tb303(1)),
                device_msg(3, 56, b.tr808()),
                device_msg(4, 74, b"\0\0")]

    def load_messages(self):
        return list(self._messages)


def read_song(path):
    with open(path, "rb") as f:
        return Song1x(f.read())


def read_any(path):
    """A 1.x song (Song1x) or an RBS song (tools/rbs.py)."""
    with open(path, "rb") as f:
        data = f.read()
    if is_1x(data):
        return Song1x(data)
    import rbs   # tools/, next to this file
    return rbs.read_song(path)


def main(argv):
    if len(argv) == 2 and argv[0] == "--dump":
        s = read_song(argv[1])
        print("version %d.%d, song mode %d, tempo %d, loop %d..%d" % (
            s.version + (s.glob.song_mode, s.glob.tempo, s.glob.loop_start, s.glob.loop_end)))
        for m in s.load_messages():
            extra = (" dev %d sub %d" % (m[3], m[4])) if m[2] == 41 else (" track %d" % m[3]) if m[2] == 37 else ""
            print("msg %d len %d%s" % (m[2], len(m), extra))
        return 0
    if len(argv) == 2 and argv[0] == "--survey":
        for name in sorted(os.listdir(argv[1])):
            path = os.path.join(argv[1], name)
            with open(path, "rb") as f:
                data = f.read()
            if not is_1x(data):
                continue
            try:
                s = Song1x(data)
                n = sum(len(m) for m in s.load_messages() if m[2] == 37)
                print("ok   %s: %d.%d mode %d tempo %d loop %d..%d" % (
                    name, s.version[0], s.version[1], s.glob.song_mode, s.glob.tempo,
                    s.glob.loop_start, s.glob.loop_end))
            except Error as e:
                print("FAIL %s: %s" % (name, e))
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

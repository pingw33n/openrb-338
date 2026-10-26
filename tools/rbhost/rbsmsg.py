#!/usr/bin/python3
"""Convert an .rbs song into the engine messages Rebirth.exe sends when it loads the song.

    rbsmsg.py SONG.rbs OUT.rbhm      write the message file rbhost replays
    rbsmsg.py --dump SONG.rbs          list the messages

The messages and their order are those of docs/song_load.md section 2 (step 5 and 6): msg 36
(globals), the nine msg 41 device blocks, the msg 37 track-event batches with the exe's validation
(section 5), and msg 32. Each is a song message `[78, song, msg, payload...]`; the song byte is
written as 0 and rbhost replaces it with the slot the engine returns for msg 52.

Message file (all integers little-endian unless noted):
    "RBHM", u32 version (1), u16 mod id, u32 tempo (BPM*1000), u32 loop start tick,
    u32 loop end tick, u32 message count, then per message: u32 length, the bytes.
The tempo and loop values are those of msg 36, which the engine reports to the export's queries.

Only the built-in mod "Standard ReBirth" (id 411, exe sub_417AE9) is supported; mods are m-2.
Stdlib only; the song model is tools/rbs.py.
"""
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.dirname(HERE))
import rbs  # noqa: E402

MAGIC = b"RBHM"
VERSION = 1
STANDARD_MOD = "Standard ReBirth"
STANDARD_MOD_ID = 411              # exe sub_417AE9: COpenMod id of the built-in mod

# Device ids and msg-41 sub-message numbers (docs/song_load.md section 1).
DEV_SUB = {"mixer": (0, 143), "tb303_1": (1, 39), "tb303_2": (2, 39), "tr808": (3, 56), "tr909": (4, 73),
           "delay": (5, 86), "dist": (6, 94), "pcf": (7, 116), "comp": (8, 104)}

BATCH = 1284                       # file events per msg-37 batch (sub_436B6F, sub_467E70(1284, left))
MAX_TICK = 768000                  # sub_42FD60: tick < 768000 and a multiple of 24


def u32be(v):
    return struct.pack(">I", v & 0xFFFFFFFF)


def song_msg(msg, payload=b""):
    return bytes([78, 0, msg]) + payload


def device_msg(name, payload):
    dev, sub = DEV_SUB[name]
    return song_msg(41, bytes([dev, sub]) + payload)


def version_tuple(song):
    major, minor = (int(x) for x in song.head.version.split("."))
    return major, minor


# ------------------------------------------------------------------ msg 36


def globals_payload(song):
    """23 bytes (docs/song_load.md section 3)."""
    g = song.glob
    graw = g.build()
    vintage = graw[482] if version_tuple(song) >= (4, 2) else 0
    out = bytearray(23)
    out[0] = graw[0]               # song mode
    out[1] = 0                     # play/stop: stopped
    out[2] = 0
    out[3] = graw[1]               # loop on
    out[4:8] = graw[2:6]           # tempo, big-endian in both
    out[8:12] = b"\0\0\0\0"        # locate tick
    out[12:16] = graw[6:10]        # loop start (ticks)
    out[16:20] = graw[10:14]       # loop end
    out[20] = graw[14]             # shuffle
    out[21] = vintage              # sound model, song+0x16D
    out[22] = 1
    return bytes(out)


# ------------------------------------------------------------------ msg 41


def mixer_payload(song):
    """29 bytes (section 4.1); 4.1 files get every channel enabled (0x435708)."""
    raw = song.mixer.build()
    major, minor = version_tuple(song)
    if (major, minor) < (4, 1):
        raise ValueError("file version %s: the 4.0 mixer converter is not implemented" % song.head.version)
    out = bytearray(29)
    out[0], out[1], out[2] = raw[0], raw[1], raw[2]
    for c in range(4):
        o = 16 + 12 * c
        ch = bytearray(raw[o:o + 5])
        if (major, minor) == (4, 1):
            ch[0] = 1
        out[5 + 6 * c:5 + 6 * c + 5] = ch
    return bytes(out)


def simple_payload(params, n, tail):
    raw = params.build()
    return bytes(raw[:n]) + bytes(tail)


def tb303_payload(dev):
    """3148 bytes (section 4.6)."""
    raw = dev.build()
    out = bytearray(3148)
    out[0:9] = raw[0:9]
    for p in range(32):
        src = 9 + 34 * p
        dst = 12 + 98 * p
        out[dst] = raw[src]
        out[dst + 1] = raw[src + 1]
        for s in range(16):
            note, fl = raw[src + 2 + 2 * s], raw[src + 3 + 2 * s]
            o = dst + 2 + 6 * s
            out[o:o + 6] = bytes([note, fl & 1, fl >> 1 & 1, fl >> 2 & 1, fl >> 3 & 1, fl >> 4 & 1])
    return bytes(out)


def tom_tune(v):
    return v // 2 + 32             # sub_432E1B, files before 4.2


def tr808_payload(dev, old):
    """6240 bytes (section 4.7)."""
    raw = dev.build()
    out = bytearray(6240)
    out[0:29] = raw[0:29]
    if old:
        for i in (10, 13, 16):
            out[i] = tom_tune(raw[i])
    for p in range(32):
        src = 30 + 194 * p
        dst = 32 + 194 * p
        out[dst], out[dst + 1] = raw[src], raw[src + 1]
        for k in range(192):
            out[dst + 2 + k] = 1 if raw[src + 2 + k] else 0
    return bytes(out)


def tr909_payload(dev):
    """6241 bytes (section 4.8), with the MidTomDecay quirk (0x433558)."""
    raw = dev.build()
    out = bytearray(6241)
    out[0:30] = raw[0:30]
    out[16] = raw[13]
    for p in range(32):
        src = 31 + 194 * p
        dst = 33 + 194 * p
        out[dst:dst + 194] = raw[src:src + 194]
    return bytes(out)


# ------------------------------------------------------------------ msg 37


def _ranges():
    """{track: {ctrl: (lo, hi)}} (section 5)."""
    r = {}
    mix = {1: (0, 5), 2: (0, 4)}
    for c in range(4):
        b = 6 + 6 * c
        mix[b], mix[b + 1], mix[b + 2], mix[b + 3] = (0, 127), (0, 127), (0, 127), (0, 1)
    r[0] = mix
    t303 = {0: (0, 1), 1: (0, 31), 8: (0, 1)}
    t303.update({i: (0, 127) for i in range(2, 8)})
    r[1] = r[2] = t303
    r[3] = {0: (0, 1), 1: (0, 31), **{i: (0, 127) for i in range(2, 29)}}
    r[4] = {0: (0, 1), 1: (0, 31), **{i: (0, 127) for i in range(2, 30)}}
    r[5] = {0: (0, 1), 1: (1, 32), 2: (0, 1), 3: (0, 127), 4: (0, 127)}
    r[6] = {0: (0, 1), 1: (0, 127), 2: (0, 127)}
    r[7] = {0: (0, 1), 1: (0, 127), 2: (0, 127), 3: (0, 127), 4: (0, 127), 5: (0, 127), 6: (0, 1)}
    r[8] = {0: (0, 1), 1: (0, 127), 2: (0, 127)}
    return r


RANGES = _ranges()


def track_batches(track_no, raw, old808):
    """The msg-37 packets of one TRAK chunk (sub_436B6F; sub_436F5A for an 808 track before 4.2).

    raw is the TRAK body: u32 count, then per event a variable-length delta, ctrl, value."""
    count = struct.unpack(">I", raw[0:4])[0]
    p = 4
    absolute = 0                   # advanced only by events whose tick is valid
    accepted = 0
    done = 0
    out = []
    ranges = RANGES[track_no]
    while done < count:
        n = min(BATCH, count - done)
        first = accepted
        body = bytearray()
        for _ in range(n):
            delta, p = rbs.decode_delta(raw, p)
            ctrl, value = raw[p], raw[p + 1]
            p += 2
            cand = (absolute + delta) & 0xFFFFFFFF
            tick = (24 * cand) & 0x1FFFFFFF
            done += 1
            if not (tick < MAX_TICK and tick % 24 == 0):
                continue
            absolute = cand
            if old808 and track_no == 3 and ctrl in (10, 13, 16):
                value = tom_tune(value)
            rng = ranges.get(ctrl)
            if rng is None or not rng[0] <= value <= rng[1]:
                continue
            body += bytes([99]) + u32be(tick) + bytes([ctrl, value])
            accepted += 1
        if accepted > first:
            out.append(song_msg(37, bytes([track_no]) + u32be(first) + u32be(accepted) + bytes(body)))
    return out


def track_raw(song, name):
    """The TRAK body; rbs.py writes back what it read byte for byte."""
    return song.tracks[name].build()


# ------------------------------------------------------------------ the whole load


def load_messages(song):
    major, minor = version_tuple(song)
    if major != 4:
        raise ValueError("file major version %d" % major)
    old = (major, minor) < (4, 2)
    msgs = [song_msg(36, globals_payload(song)),
            device_msg("mixer", mixer_payload(song)),
            device_msg("delay", simple_payload(song.delay, 5, [0])),
            device_msg("pcf", simple_payload(song.pcf, 7, [0])),
            device_msg("dist", simple_payload(song.dist, 3, [0])),
            device_msg("comp", simple_payload(song.comp, 3, [127, 0])),
            device_msg("tb303_1", tb303_payload(song.tb303[0])),
            device_msg("tb303_2", tb303_payload(song.tb303[1])),
            device_msg("tr808", tr808_payload(song.tr808, old)),
            device_msg("tr909", tr909_payload(song.tr909))]
    for i, name in enumerate(rbs.TRACK_NAMES):
        msgs += track_batches(i, track_raw(song, name), old)
    msgs.append(song_msg(32))
    return msgs


def mod_id(song):
    if song.glob.mod_name != STANDARD_MOD:
        raise ValueError("mod %r: only %r is supported" % (song.glob.mod_name, STANDARD_MOD))
    return STANDARD_MOD_ID


def message_file(song):
    msgs = load_messages(song)
    g = song.glob
    out = [MAGIC, struct.pack("<IHIIII", VERSION, mod_id(song), g.tempo, g.loop_start, g.loop_end, len(msgs))]
    for m in msgs:
        out.append(struct.pack("<I", len(m)) + m)
    return b"".join(out)


def read_message_file(data):
    if data[:4] != MAGIC:
        raise ValueError("not a message file")
    version, mod, tempo, start, end, n = struct.unpack("<IHIIII", data[4:26])
    p = 26
    msgs = []
    for _ in range(n):
        ln = struct.unpack("<I", data[p:p + 4])[0]
        msgs.append(data[p + 4:p + 4 + ln])
        p += 4 + ln
    if p != len(data):
        raise ValueError("trailing bytes")
    return {"version": version, "mod": mod, "tempo": tempo, "loop_start": start, "loop_end": end, "messages": msgs}


def main(argv):
    if len(argv) == 2 and argv[0] == "--dump":
        song = rbs.read_song(argv[1])
        for m in load_messages(song):
            extra = (" dev %d sub %d" % (m[3], m[4])) if m[2] == 41 else (" track %d" % m[3]) if m[2] == 37 else ""
            print("msg %d len %d%s" % (m[2], len(m), extra))
        return 0
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    song = rbs.read_song(argv[0])
    with open(argv[1], "wb") as f:
        f.write(message_file(song))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

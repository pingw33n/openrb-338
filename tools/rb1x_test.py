"""Unit tests of tools/rb1x.py on a synthetic ReBirth 1.x song."""
import os
import struct
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rb1x  # noqa: E402


def vlq(v):
    out = [v & 0x7F]
    v >>= 7
    while v:
        out.append(0x80 | (v & 0x7F))
        v >>= 7
    return bytes(reversed(out))


def encode_block(block):
    out = bytearray([0, 0, 0x66, 0, 0x15, 1, 0])
    for i in range(0, len(block), 7):
        g = block[i:i + 7]
        hi = 0
        for k, b in enumerate(g):
            out.append(b & 0x7F)
            if b & 0x80:
                hi |= 0x40 >> k
        out += bytes(7 - len(g)) + bytes([hi])
    return bytes(out) + b"\xf7"


def track(events):
    body, last = bytearray(), 0
    for t, data in events:
        body += vlq(t - last) + data
        last = t
    body += b"\x00\xff\x2f\x00"
    return b"MTrk" + struct.pack(">I", len(body)) + bytes(body)


def song(block, tracks, version=b"3.1.0"):
    text = b"File version " + version + b" (c)1997"
    t0 = track([(0, b"\xff\x02" + vlq(len(text)) + text)])
    sx = encode_block(block)
    t1 = track([(0, b"\xf0" + vlq(len(sx)) + sx)])
    rest = [track(t) for t in tracks]
    return b"MThd" + struct.pack(">IHHH", 6, 1, 2 + len(rest), 32) + t0 + t1 + b"".join(rest)


def block():
    b = bytearray(rb1x.DECODED)
    b[0:4] = b"\xbc\x03\x01\x00"
    b[5:9] = b"TinY"
    b[9] = 90
    b[10:14] = struct.pack(">I", 2)
    b[14:18] = struct.pack(">I", 4)
    b[0x12] = 1
    b[0x2A:0x2E] = struct.pack(">I", 128000)
    b[0x2F] = 0x83
    b[0x37] = 1
    b[0x40] = 2
    b[0x100 + 7] = 0x3F
    b[352 + 30:352 + 34] = bytes([13, 0, 0b1001, 0])
    b[352 + 8192 + 48] = 0x40
    b[352 + 8192 + 24] = 100           # 808 LT tuning byte
    return bytes(b)


def cc(t, ctrl, value, ch=0):
    return (t, bytes([0xB0 | ch, ctrl, value]))


class Rb1xTest(unittest.TestCase):
    def test_decode_round_trip(self):
        b = bytes(range(256)) * 64
        b = b[:5] + b"TinY" + b[9:]
        self.assertEqual(rb1x.decode_sysex(encode_block(b)), b)
        with self.assertRaises(rb1x.Error):
            rb1x.decode_sysex(b"\x00\x00\x67" + bytes(10))

    def test_globals_and_blocks(self):
        s = rb1x.Song1x(song(block(), []))
        self.assertEqual(s.version, (3, 1))
        self.assertEqual((s.glob.tempo, s.glob.loop_start, s.glob.loop_end), (128000, 1536, 4608))
        m = s.load_messages()
        self.assertEqual([x[2] for x in m[:9]], [36] + [41] * 8)
        self.assertEqual([(x[3], x[4]) for x in m[1:9]],
                         [(0, 143), (5, 86), (6, 94), (7, 116), (1, 39), (2, 39), (3, 56), (4, 74)])
        g = m[0][3:]
        self.assertEqual((g[0], g[20], g[21], g[22]), (1, 64, 1, 1))
        mix = m[1][5:]
        self.assertEqual(mix[:3], bytes([90, 0, 3]))
        # dist on channel 1 only; channel 3 (the 909) at sub_423560's defaults
        self.assertEqual((mix[5 + 4], mix[11 + 4], mix[23], mix[24]), (0, 1, 1, 100))
        self.assertEqual(m[2][5:7], bytes([0, 3]))         # delay off, 3 steps
        self.assertEqual(m[2][7], 1)                         # sixteenths
        t = m[5][5:]
        self.assertEqual((t[13], t[12 + 98 + 1]), (4, 16))  # pattern lengths
        self.assertEqual(t[14:20], bytes([1, 1, 0, 1, 1, 0]))  # note 13: 1, up; slide, down
        d = m[7][5:]
        self.assertEqual(d[10], 100 // 2 + 64)
        self.assertEqual(d[34], 1)
        self.assertEqual(m[8][3:], bytes([4, 74, 0, 0]))
        self.assertEqual(m[9][3:], bytes([4]) + struct.pack(">II", 0, 1) + bytes([99]) + bytes(4) + bytes(2))
        self.assertEqual(m[-1], bytes([78, 0, 32]))

    def test_tracks(self):
        t2 = [cc(i, 14, 1) for i in range(1300)] + [cc(2000, 7, 99, ch=5)]
        t5 = [cc(10, 17, 2), cc(10, 31, 3), cc(11, 20, 77)]
        s = rb1x.Song1x(song(block(), [t2, [], [cc(5, 24, 100)], t5]))
        batches = [x for x in s.load_messages() if x[2] == 37]
        self.assertEqual([(b[3], len(b) // 7 - 1) for b in batches],
                         [(1, 1284), (0, 6), (1, 16), (3, 1), (4, 1), (5, 1)])
        mixer = batches[1][12:]
        ev = [(struct.unpack(">I", mixer[i + 1:i + 5])[0], mixer[i + 5], mixer[i + 6])
              for i in range(0, len(mixer), 7)]
        self.assertEqual(ev, [(48000, 6, 99), (240, 9, 0), (240, 15, 0), (240, 21, 1), (240, 27, 0), (240, 2, 0)])
        self.assertEqual(batches[3][12:], bytes([99]) + struct.pack(">I", 120) + bytes([10, 114]))

    def test_refusals(self):
        with self.assertRaises(rb1x.Error):
            rb1x.Song1x(song(block(), [[cc(0, 99, 1)]]))
        with self.assertRaises(rb1x.Error):
            rb1x.Song1x(song(block(), [], version=b"3.0.0"))


if __name__ == "__main__":
    unittest.main()

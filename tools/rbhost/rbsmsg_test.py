"""Tests of tools/rbhost/rbsmsg.py: /usr/bin/python3 -m unittest tools/rbhost/rbsmsg_test.py"""
import glob
import os
import struct
import sys
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.dirname(HERE))
import rbs  # noqa: E402
import rbsmsg  # noqa: E402

SHIPPED = sorted(glob.glob(os.path.join(rbs.ROOT, "tests", "songs", "shipped", "*.rbs")))
DIAG = os.path.join(rbs.ROOT, "tests", "songs", "diag", "songs")


def trak(events):
    """Raw TRAK body from (delta, ctrl, value) triples."""
    out = [struct.pack(">I", len(events))]
    for d, c, v in events:
        out.append(rbs.encode_delta(d) + bytes([c, v]))
    return b"".join(out)


def events_of(packet):
    """[(tick, ctrl, value)] of a msg-37 packet, with its header."""
    assert packet[:3] == bytes([78, 0, 37])
    track, first, last = packet[3], struct.unpack(">I", packet[4:8])[0], struct.unpack(">I", packet[8:12])[0]
    ev = []
    for p in range(12, len(packet), 7):
        assert packet[p] == 99
        ev.append((struct.unpack(">I", packet[p + 1:p + 5])[0], packet[p + 5], packet[p + 6]))
    return track, first, last, ev


class Shapes(unittest.TestCase):
    def test_every_shipped_song(self):
        sizes = [26, 34, 11, 13, 9, 10, 3153, 3153, 6245, 6246]
        for path in SHIPPED:
            msgs = rbsmsg.load_messages(rbs.read_song(path))
            self.assertEqual([len(m) for m in msgs[:10]], sizes, path)
            self.assertEqual([m[2] for m in msgs[:10]], [36] + [41] * 9)
            self.assertEqual([m[3] for m in msgs[1:10]], [0, 5, 7, 6, 8, 1, 2, 3, 4])
            self.assertEqual([m[4] for m in msgs[1:10]], [143, 86, 116, 94, 104, 39, 39, 56, 73])
            self.assertEqual(msgs[-1], bytes([78, 0, 32]))
            tracks = [m[3] for m in msgs if m[2] == 37]
            self.assertEqual(tracks, sorted(tracks), path)
            for m in msgs:
                self.assertLessEqual(len(m), 9000)   # the exe's buffer (sub_436B6F)

    def test_message_file_round_trip(self):
        song = rbs.read_song(os.path.join(DIAG, "303_accent0.rbs"))
        f = rbsmsg.read_message_file(rbsmsg.message_file(song))
        self.assertEqual(f["mod"], 411)
        self.assertEqual((f["tempo"], f["loop_start"], f["loop_end"]),
                         (song.glob.tempo, song.glob.loop_start, song.glob.loop_end))
        self.assertEqual(f["messages"], rbsmsg.load_messages(song))

    def test_other_mod_refused(self):
        song = rbs.read_song(os.path.join(DIAG, "303_accent0.rbs"))
        song.glob.mod_name = "Orbit 2.0"
        with self.assertRaises(ValueError):
            rbsmsg.message_file(song)

    def test_override_gives_the_override_id(self):
        song = rbs.read_song(os.path.join(DIAG, "303_accent0.rbs"))
        song.glob.mod_name = "Orbit 2.0"
        self.assertEqual(rbsmsg.mod_id(song, 412), 412)
        self.assertEqual(rbsmsg.read_message_file(rbsmsg.message_file(song, 412))["mod"], 412)
        song.glob.mod_name = rbsmsg.STANDARD_MOD
        self.assertEqual(rbsmsg.mod_id(song), 411)       # no override: the built-in mod's id
        self.assertEqual(rbsmsg.mod_id(song, 7), 7)


class Payloads(unittest.TestCase):
    def setUp(self):
        self.song = rbs.build_song({
            "glob": {"song_mode": 1, "loop": 1, "tempo": 133250, "loop_start": 768, "loop_end": 3072,
                     "shuffle": 77, "vintage": 1},
            "mixer": {"master": 90, "comp_id": 3, "pcf_id": 2,
                      "channels": {"1": {"enabled": 0, "level": 50, "pan": 10, "delay_send": 20, "dist": 1}}},
            "comp": {"values": {"enabled": 1, "amount": 5, "threshold": 6}},
            "tb303": {"0": {"patterns": {"3": {"shuffle": 1, "length": 7, "steps": {
                "2": {"note": 12, "slide": 1, "accent": 0, "up": 1, "down": 0, "rest": 0}}}}}},
            "tr909": {"params": {"low_tom_decay": 11, "mid_tom_decay": 99},
                      "patterns": {"1": {"steps": {"5": [2, 3, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0]}}}},
            "tr808": {"patterns": {"0": {"steps": {"0": [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]}}}},
        })

    def test_globals(self):
        g = rbsmsg.globals_payload(self.song)
        self.assertEqual(g, bytes([1, 0, 0, 1]) + struct.pack(">IIII", 133250, 0, 768, 3072) + bytes([77, 1, 1]))

    def test_globals_vintage_before_42(self):
        self.song.head.version = "4.1"
        self.assertEqual(rbsmsg.globals_payload(self.song)[21], 0)

    def test_mixer(self):
        m = rbsmsg.mixer_payload(self.song)
        self.assertEqual(m[:5], bytes([90, 3, 2, 0, 0]))
        self.assertEqual(m[11:17], bytes([0, 50, 10, 20, 1, 0]))
        self.song.head.version = "4.1"
        self.assertEqual(rbsmsg.mixer_payload(self.song)[11], 1)   # 4.1: enabled forced (0x435708)

    def test_comp_tail(self):
        self.assertEqual(rbsmsg.simple_payload(self.song.comp, 3, [127, 0]), bytes([1, 5, 6, 127, 0]))

    def test_303_step(self):
        p = rbsmsg.tb303_payload(self.song.tb303[0])
        self.assertEqual(len(p), 3148)
        o = 12 + 98 * 3
        self.assertEqual(p[o:o + 2], bytes([1, 7]))
        self.assertEqual(p[o + 2 + 6 * 2:o + 2 + 6 * 3], bytes([12, 1, 0, 1, 0, 0]))
        self.assertEqual(p[o + 2:o + 8], bytes([0, 0, 0, 0, 0, 1]))     # template steps are rests

    def test_909_quirk_and_raw_steps(self):
        p = rbsmsg.tr909_payload(self.song.tr909)
        self.assertEqual(len(p), 6241)
        self.assertEqual(p[13], 11)
        self.assertEqual(p[16], 11)    # MidTomDecay takes LowTomDecay (0x433558)
        o = 33 + 194 + 2 + 12 * 5
        self.assertEqual(p[o:o + 12], bytes([2, 3, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0]))

    def test_808_steps_and_old_toms(self):
        p = rbsmsg.tr808_payload(self.song.tr808, False)
        self.assertEqual(len(p), 6240)
        self.assertEqual(p[34:46], bytes([1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]))
        raw = self.song.tr808.build()
        old = rbsmsg.tr808_payload(self.song.tr808, True)
        for i in (10, 13, 16):
            self.assertEqual(old[i], raw[i] // 2 + 32)


class Tracks(unittest.TestCase):
    def test_validation(self):
        raw = trak([(0, 3, 5),      # 303 ctrl 3 = cutoff: kept
                    (2, 1, 32),     # pattern 32: out of range, dropped, position still advances
                    (1, 9, 0),      # ctrl 9: not a 303 controller
                    (40000, 3, 1),  # tick >= 768000: dropped and the position does not advance
                    (5, 8, 1)])     # kept at 0 + 2 + 1 + 5 = 8 -> tick 192
        (pk,) = rbsmsg.track_batches(1, raw, False)
        self.assertEqual(events_of(pk), (1, 0, 2, [(0, 3, 5), (192, 8, 1)]))

    def test_batches(self):
        n = rbsmsg.BATCH + 10
        raw = trak([(1, 6, i % 128) for i in range(n)])
        a, b = rbsmsg.track_batches(0, raw, False)
        ta, fa, la, ea = events_of(a)
        tb, fb, lb, eb = events_of(b)
        self.assertEqual((ta, fa, la, len(ea)), (0, 0, rbsmsg.BATCH, rbsmsg.BATCH))
        self.assertEqual((tb, fb, lb, len(eb)), (0, rbsmsg.BATCH, n, 10))
        self.assertEqual(eb[-1][0], 24 * n)

    def test_empty_batch_not_sent(self):
        self.assertEqual(rbsmsg.track_batches(8, trak([(0, 7, 1)]), False), [])

    def test_mixer_controllers(self):
        ok = [(0, 1, 5), (0, 2, 4), (0, 27, 1), (0, 26, 127)]
        bad = [(0, 1, 6), (0, 3, 0), (0, 5, 0), (0, 27, 2), (0, 28, 0)]
        (pk,) = rbsmsg.track_batches(0, trak(ok + bad), False)
        self.assertEqual([(c, v) for _, c, v in events_of(pk)[3]], [(c, v) for _, c, v in ok])

    def test_old_808_toms(self):
        (pk,) = rbsmsg.track_batches(3, trak([(0, 10, 100), (0, 11, 100)]), True)
        self.assertEqual([(c, v) for _, c, v in events_of(pk)[3]], [(10, 82), (11, 100)])


if __name__ == "__main__":
    unittest.main()

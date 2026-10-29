"""Tests of tools/rbs.py: /usr/bin/python3 -m unittest tools/rbs_test.py"""
import glob
import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rbs  # noqa: E402

SHIPPED = sorted(glob.glob(os.path.join(rbs.ROOT, "tests", "songs", "shipped", "*.rbs")))


class DeltaPosition(unittest.TestCase):
    CASES = {0: b"\x00", 127: b"\x7f", 128: b"\x81\x00", 16383: b"\xff\x7f",
             16384: b"\x81\x80\x00", 31976: b"\x81\xf9\x68"}

    def test_encode(self):
        for v, b in self.CASES.items():
            self.assertEqual(rbs.encode_delta(v), b, v)

    def test_decode(self):
        for v, b in self.CASES.items():
            self.assertEqual(rbs.decode_delta(b + b"\xee", 0), (v, len(b)), v)

    def test_reference_algorithm(self):
        # RBS42.txt WriteDeltaPos, transcribed
        def write(value):
            out = []
            buffer = value & 0x7F
            value >>= 7
            while value > 0:
                buffer <<= 8
                buffer |= 0x80
                buffer += value & 0x7F
                value >>= 7
            while True:
                out.append(buffer & 0xFF)
                if buffer & 0x80:
                    buffer >>= 8
                else:
                    return bytes(out)
        for v in list(range(0, 600)) + [16383, 16384, 31976, 2097151]:
            self.assertEqual(rbs.encode_delta(v), write(v), v)
            self.assertEqual(rbs.decode_delta(write(v), 0)[0], v)

    def test_track_roundtrip(self):
        evs = [rbs.Event(p, 0, 1) for p in (0, 127, 255, 16638, 33022)]
        t = rbs.Track(evs)
        self.assertEqual(rbs.Track.parse(t.build()).events, evs)

    def test_long_deltas_read_as_the_exe(self):
        # sub_4A09D1: no length limit, 32-bit shift (a third-party song has 8f ff fa 94 00)
        self.assertEqual(rbs.decode_delta(b"\x8f\xff\xfa\x94\x00", 0), (0xFFFE8A00, 5))
        self.assertEqual(rbs.encode_delta(0xFFFE8A00), b"\x8f\xff\xfa\x94\x00")
        v = 0
        for c in (1, 1, 1, 1, 1, 2):
            v = ((v << 7) | c) & 0xFFFFFFFF
        self.assertEqual(rbs.decode_delta(b"\x81\x81\x81\x81\x81\x02", 0), (v, 6))
        t = rbs.Track.parse(b"\0\0\0\2\x00\x06\x09\x8f\xff\xfa\x94\x00\x07\x01")
        self.assertEqual([e.pos for e in t.events], [0, 0xFFFE8A00])
        self.assertEqual(t.build(), b"\0\0\0\2\x00\x06\x09\x8f\xff\xfa\x94\x00\x07\x01")

    def test_a_wrapped_track_writes_back_its_deltas(self):
        raw = b"\0\0\0\2\x8f\xff\xff\xff\x70\x06\x09\x20\x07\x01"   # 0xFFFFFFF0, then 0x20
        t = rbs.Track.parse(raw)
        self.assertEqual([e.pos for e in t.events], [0xFFFFFFF0, 0x10])
        self.assertEqual(t.build(), raw)
        # every backward step is a wrap, whatever its delta (1, 0xFFFFFFF0, 2^31, 2^32 - 1)
        for a, b in ((0xFFFFFFFF, 0), (0x10, 0), (0x80000000, 0), (5, 4)):
            t = rbs.Track([rbs.Event(a, 0, 1), rbs.Event(b, 0, 1)])
            self.assertEqual(rbs.Track.parse(t.build()).events, t.events)
        raw = b"\0\0\0\2\x10\x06\x09\x8f\xff\xff\xff\x70\x07\x01"   # 0x10, then 0xFFFFFFF0
        t = rbs.Track.parse(raw)
        self.assertEqual([e.pos for e in t.events], [0x10, 0])
        self.assertEqual(t.build(), raw)


class Shipped(unittest.TestCase):
    def test_count(self):
        self.assertEqual(len(SHIPPED), 18)

    def test_roundtrip_is_byte_identical(self):
        for f in SHIPPED:
            data = rbs.read_bytes(f)
            self.assertEqual(rbs.Song.parse(data).build(), data, f)

    def test_json_model_roundtrip(self):
        # the JSON model alone (plus the hidden raw bytes) rebuilds the file
        for f in SHIPPED:
            data = rbs.read_bytes(f)
            song = rbs.Song.parse(data)
            again = rbs.from_json(json.loads(json.dumps(rbs.to_json(song))))
            rbs.restore_raw(again, song)
            self.assertEqual(again.build(), data, f)

    def test_versions(self):
        v = [rbs.read_song(f).head.version for f in SHIPPED]
        self.assertEqual(v.count("4.1"), 17)
        self.assertEqual(v.count("4.2"), 1)

    def test_corrected_meanings(self):
        silent = rbs.read_song([f for f in SHIPPED if "Silent" in f][0])
        # every 303 step is "00 10": rest set (docs/song_load.md 4.6)
        self.assertTrue(all(s.rest and not s.slide for p in silent.tb303[0].patterns for s in p.steps))
        self.assertEqual(rbs.MIXER_NAMES[0x1A], "tr909_delay_send")
        self.assertEqual(rbs.MIXER_NAMES[0x1B], "tr909_dist")
        self.assertNotIn(0x20, rbs.MIXER_NAMES)

    def test_track_events_valid(self):
        # Every shipped song obeys the format's rules except the 31976 limit, which one song exceeds
        # (Alloy Cowboy Resurrection has events up to position 41024; the engine drops positions >= 32000,
        # tick 768000, docs/song_load.md 5).
        over = []
        for f in SHIPPED:
            song = rbs.read_song(f)
            try:
                rbs.validate(song)
            except ValueError as e:
                over.append((os.path.basename(f), str(e)))
        self.assertTrue(all("beyond 31976" in e for _, e in over), over)


class Build(unittest.TestCase):
    SPEC = {
        "glob": {"tempo": 140000, "song_mode": 1, "loop": 1, "loop_start": 0, "loop_end": 768},
        "tb303": {"0": {"params": {"enabled": 1, "cutoff": 60},
                        "patterns": {"0": {"steps": {"0": {"note": 0, "rest": 0}, "4": {"note": 7, "rest": 0}}}}}},
        "tracks": {"tb303_1": [{"pos": 0, "ctrl": "enabled", "value": 1}, {"pos": 0, "ctrl": "pattern", "value": 0},
                               {"pos": 64, "ctrl": 3, "value": 100}]},
    }

    def build(self, spec):
        return rbs.build_song(spec)

    def test_minimal_spec_gives_template(self):
        out = self.build({}).build()
        song = rbs.Song.parse(out)
        self.assertEqual(song.head.version, "4.2")
        self.assertEqual(rbs.to_json(song)["glob"], rbs.to_json(
            rbs.read_song(rbs.DEFAULT_TEMPLATE))["glob"])

    def test_spec(self):
        song = rbs.Song.parse(self.build(self.SPEC).build())
        self.assertEqual(song.glob.tempo, 140000)
        t = song.tb303[0]
        self.assertEqual(t.params["cutoff"], 60)
        self.assertEqual(t.patterns[0].steps[4].note, 7)
        self.assertEqual(t.patterns[0].steps[4].rest, 0)
        self.assertEqual(t.patterns[0].steps[1].rest, 1)
        self.assertEqual([(e.pos, e.ctrl, e.value) for e in song.tracks["tb303_1"].events],
                         [(0, 0, 1), (0, 1, 0), (64, 3, 100)])
        # other tracks keep the template's events
        self.assertTrue(song.tracks["mixer"].events)

    def test_tom_tuning_conversion(self):
        tmpl = rbs.read_song(rbs.DEFAULT_TEMPLATE)
        s42 = self.build({})
        s41 = self.build({"version": "4.1"})
        self.assertEqual(s41.tr808.params["low_tom_tuning"], tmpl.tr808.params["low_tom_tuning"])
        self.assertEqual(s42.tr808.params["low_tom_tuning"], tmpl.tr808.params["low_tom_tuning"] // 2 + 32)

    def test_validation(self):
        for bad in ({"tracks": {"tr808": [{"pos": 5, "ctrl": "enabled", "value": 1}]}},
                    {"tracks": {"tr808": [{"pos": 0, "ctrl": "enabled", "value": 1},
                                          {"pos": 16, "ctrl": "pattern", "value": 1}]}},
                    {"tracks": {"delay": [{"pos": 0, "ctrl": "enabled", "value": 1},
                                          {"pos": 31977, "ctrl": "enabled", "value": 0}]}},
                    {"tracks": {"mixer": [{"pos": 0, "ctrl": 0x20, "value": 1}]}},
                    # unsorted: the writer would store a wrap, so the spec check refuses it
                    {"tracks": {"delay": [{"pos": 0, "ctrl": "enabled", "value": 1},
                                          {"pos": 8, "ctrl": "enabled", "value": 0},
                                          {"pos": 4, "ctrl": "enabled", "value": 1}]}}):
            with self.assertRaises(ValueError, msg=str(bad)):
                self.build(bad)

    def test_unknown_field(self):
        with self.assertRaises(ValueError):
            self.build({"glob": {"tempoo": 1}})

    def test_cli(self):
        with tempfile.TemporaryDirectory() as d:
            spec, out = os.path.join(d, "s.json"), os.path.join(d, "o.rbs")
            with open(spec, "w") as fh:
                json.dump(self.SPEC, fh)
            self.assertEqual(rbs.main(["rbs.py", "build", spec, out]), 0)
            self.assertEqual(rbs.main(["rbs.py", "check", out]), 0)
            self.assertIn("tempo 140.000", rbs.dump(rbs.read_song(out)))


if __name__ == "__main__":
    unittest.main()

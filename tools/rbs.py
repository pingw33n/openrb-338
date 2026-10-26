#!/usr/bin/python3
"""Reader and writer of the ReBirth RB-338 song format (.rbs, version 4.x).

Format: docs/RBS42.txt, with the corrections of docs/song_load.md section 10
(TRAK position unit is a 32nd note, mixer-track 909 controllers 0x1A/0x1B,
PCF ids 1..4, 303 step bit 4 set = rest, delay mode 0 = triplets, 303 waveform
0 = saw).  Stdlib only.

The model is a tree of dataclasses (Song).  Bytes the model does not name
(reserved bytes, USRI's tail, odd flag bits) are carried in a hidden `_raw`
of each chunk and written back as read, so read -> write is byte-identical;
a field is rewritten only when its value differs from what `_raw` decodes to.

CLI:
    rbs.py dump FILE             human-readable listing
    rbs.py json FILE             the model as JSON
    rbs.py build SPEC.json OUT   write a song; SPEC is merged over a template
                                 song (default: tests/songs/shipped/Silent
                                 Default Song.rbs, every field defaulted)
    rbs.py check FILE...         read, write back, compare bytes

Build spec: a JSON object with the same shape as `rbs.py json` output; dicts
merge recursively, a list replaces the template's list, and an object with
integer-string keys patches those indices of the template's list, e.g.
{"tr808": {"patterns": {"0": {"length": 16, "steps": {"0": [0,1,0,...]}}}}}.
"tracks" maps a track name to a list of {"pos", "ctrl", "value"} events
(ctrl is a name or an id; pos is in 32nd notes, 32 per bar).  "version" may be
"4.1" or "4.2" (default "4.2"); the 808 tom tunings of a 4.1 template are
converted (v/2+32, sub_432E1B) when the output is 4.2.
"""
import json
import os
import struct
import sys
from dataclasses import dataclass, field, fields, is_dataclass
from typing import Dict, List

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEFAULT_TEMPLATE = os.path.join(ROOT, "tests", "songs", "shipped", "Silent Default Song.rbs")

# RBS42.txt's limit. Deliberately stricter than the exe, which accepts positions below 32000
# (tick 768000, docs/song_load.md 5): build only writes songs the format allows.
MAX_POS = 31976
TRACK_NAMES = ["mixer", "tb303_1", "tb303_2", "tr808", "tr909", "delay", "dist", "pcf", "comp"]
CHANNELS = ["tb303_1", "tb303_2", "tr808", "tr909"]

# ---------------------------------------------------------------- IFF helpers


def read_chunks(data, pos, end):
    """Yield (tag, body_start, body_size) of the chunks in data[pos:end]."""
    while pos < end:
        if pos + 8 > end:
            raise ValueError("truncated chunk header at %d" % pos)
        tag = data[pos:pos + 4]
        size = struct.unpack(">I", data[pos + 4:pos + 8])[0]
        if pos + 8 + size > end:
            raise ValueError("chunk %r at %d overruns its parent" % (tag, pos))
        yield tag, pos + 8, size
        pos += 8 + size + (size & 1)


def chunk(tag, body):
    assert len(tag) == 4
    return tag + struct.pack(">I", len(body)) + body + (b"\0" if len(body) & 1 else b"")


def cat(kind, children):
    return chunk(b"CAT ", kind + b"".join(children))


# ------------------------------------------------------------ field handling


def get_u8(raw, off):
    return raw[off]


def get_u32(raw, off):
    return struct.unpack(">I", raw[off:off + 4])[0]


def get_z(raw, off, size):
    b = bytes(raw[off:off + size])
    return b.split(b"\0", 1)[0].decode("latin-1")


def put_z(raw, off, size, s):
    b = s.encode("latin-1")
    if len(b) > size - 1:
        raise ValueError("string %r longer than %d" % (s, size - 1))
    raw[off:off + size] = b + b"\0" * (size - len(b))


# (name, offset, kind, size): kind is u8, u32 or z (zero terminated string of size)
def apply_fields(cls_fields, obj, raw):
    for name, off, kind, size in cls_fields:
        v = getattr(obj, name)
        if kind == "u8":
            if get_u8(raw, off) != v:
                if not 0 <= v <= 255:
                    raise ValueError("%s: %r is not a byte" % (name, v))
                raw[off] = v
        elif kind == "u32":
            if get_u32(raw, off) != v:
                raw[off:off + 4] = struct.pack(">I", v)
        else:
            if get_z(raw, off, size) != v:
                put_z(raw, off, size, v)


def load_fields(cls_fields, raw):
    out = {}
    for name, off, kind, size in cls_fields:
        out[name] = get_u8(raw, off) if kind == "u8" else get_u32(raw, off) if kind == "u32" else get_z(raw, off, size)
    return out


def raw_field():
    return field(default=b"", repr=False, compare=False)


# -------------------------------------------------------------------- chunks


@dataclass
class Head:
    version: str = "4.2"           # from bytes 5 (major) and 6 (minor) of the version signature
    copyright: str = "(c)1997 Propellerhead Software, all rights reserved"
    _raw: bytes = raw_field()

    SIZE = 256
    FIELDS = [("copyright", 9, "z", 129)]

    @classmethod
    def parse(cls, raw):
        return cls(version="%d.%d" % (raw[5], raw[6]), _raw=bytes(raw), **load_fields(cls.FIELDS, raw))

    def build(self):
        raw = bytearray(self._raw or bytes([0x5b, 0x54, 0x5b, 0x54, 0xbc, 4, 2, 0, 0]) + bytes(self.SIZE - 9))
        major, minor = (int(x) for x in self.version.split("."))
        raw[5], raw[6] = major, minor
        apply_fields(self.FIELDS, self, raw)
        return bytes(raw)


@dataclass
class Glob:
    song_mode: int = 0             # 0 pattern mode, 1 song mode
    loop: int = 0
    tempo: int = 120000            # BPM * 1000
    loop_start: int = 0            # bar * 768 (a bar is 768 engine ticks)
    loop_end: int = 0
    shuffle: int = 0               # 0..127
    mod_name: str = "Standard ReBirth"
    mod_ftp: str = "ftp.propellerheads.se"
    mod_www: str = "www.propellerheads.se"
    vintage: int = 0               # 0 ReBirth 2.0 sound, 1 vintage sound
    _raw: bytes = raw_field()

    SIZE = 512
    FIELDS = [("song_mode", 0, "u8", 1), ("loop", 1, "u8", 1), ("tempo", 2, "u32", 4),
              ("loop_start", 6, "u32", 4), ("loop_end", 10, "u32", 4), ("shuffle", 14, "u8", 1),
              ("mod_name", 15, "z", 65), ("mod_ftp", 80, "z", 201), ("mod_www", 281, "z", 201),
              ("vintage", 482, "u8", 1)]

    @classmethod
    def parse(cls, raw):
        return cls(_raw=bytes(raw), **load_fields(cls.FIELDS, raw))

    def build(self):
        raw = bytearray(self._raw or bytes(self.SIZE))
        apply_fields(self.FIELDS, self, raw)
        return bytes(raw)


@dataclass
class Usri:
    window_title: str = ""
    text: str = ""                 # 0x0d is a newline
    web_page: str = ""
    show_on_open: int = 0
    _raw: bytes = raw_field()      # 712 bytes in real files (RBS42.txt says 512); the tail holds junk

    SIZE = 712
    # RBS42.txt puts web_page at 242 and show_on_open at 343 for a 512 byte chunk; real files hold 712
    # bytes with the text field 200 bytes longer (derived from the shipped songs, docs/song_load.md 10).
    FIELDS = [("window_title", 0, "z", 41), ("text", 41, "z", 401), ("web_page", 442, "z", 101),
              ("show_on_open", 543, "u8", 1)]

    @classmethod
    def parse(cls, raw):
        return cls(_raw=bytes(raw), **load_fields(cls.FIELDS, raw))

    def build(self):
        raw = bytearray(self._raw or bytes(self.SIZE))
        apply_fields(self.FIELDS, self, raw)
        return bytes(raw)


@dataclass
class MixChannel:
    enabled: int = 1
    level: int = 100
    pan: int = 64
    delay_send: int = 0
    dist: int = 0                  # distortion insert on/off


@dataclass
class Mixer:
    master: int = 100
    comp_id: int = 0               # 0 off, 1 master, 2..5 = channel 0..3 (303 #1, 303 #2, 808, 909)
    pcf_id: int = 0                # 0 off, 1..4 = channel 0..3 (docs/song_load.md 4.1)
    channels: List[MixChannel] = field(default_factory=lambda: [MixChannel() for _ in range(4)])
    _raw: bytes = raw_field()

    SIZE = 64

    @classmethod
    def parse(cls, raw):
        chans = []
        for c in range(4):
            o = 16 + 12 * c
            chans.append(MixChannel(raw[o], raw[o + 1], raw[o + 2], raw[o + 3], raw[o + 4]))
        return cls(raw[0], raw[1], raw[2], chans, bytes(raw))

    def build(self):
        raw = bytearray(self._raw or bytes(self.SIZE))
        raw[0], raw[1], raw[2] = self.master, self.comp_id, self.pcf_id
        for c, ch in enumerate(self.channels):
            o = 16 + 12 * c
            raw[o:o + 5] = bytes([ch.enabled, ch.level, ch.pan, ch.delay_send, ch.dist])
        return bytes(raw)


# Parameter names by controller id == chunk byte offset (RBS42.txt, docs/song_load.md 4.x).
PARAMS = {
    "delay": ["enabled", "steps", "mode", "feedback", "pan"],          # mode 0 = triplets, 1 = sixteenths
    "pcf": ["enabled", "frequency", "resonance", "amount", "wave", "decay", "mode"],  # mode 0 BP, 1 LP
    "dist": ["enabled", "amount", "shape"],
    "comp": ["enabled", "amount", "threshold"],
    "tb303": ["enabled", "pattern", "tune", "cutoff", "resonance", "env_mod", "decay", "accent", "waveform"],
    "tr808": ["enabled", "pattern", "accent_level", "bass_level", "bass_tone", "bass_decay", "snare_level",
              "snare_tone", "snare_snappy", "low_tom_level", "low_tom_tuning", "low_tom_selector",
              "mid_tom_level", "mid_tom_tuning", "mid_tom_selector", "hi_tom_level", "hi_tom_tuning",
              "hi_tom_selector", "rim_shot_level", "rim_shot_selector", "clap_level", "clap_selector",
              "cow_bell_level", "cymbal_level", "cymbal_tone", "cymbal_decay", "open_hihat_level",
              "open_hihat_decay", "closed_hihat_level"],
    "tr909": ["enabled", "pattern", "accent_level", "bass_level", "bass_tune", "bass_attack", "bass_decay",
              "snare_level", "snare_tune", "snare_tone", "snare_snappy", "low_tom_level", "low_tom_tune",
              "low_tom_decay", "mid_tom_level", "mid_tom_tune", "mid_tom_decay", "hi_tom_level", "hi_tom_tune",
              "hi_tom_decay", "rim_shot_level", "clap_level", "hihat_level", "closed_hihat_decay",
              "open_hihat_decay", "crash_level", "crash_tune", "ride_level", "ride_tune", "flam_interval"],
}
# Step columns of the drum patterns, in file order.
DRUM_COLUMNS = {
    "tr808": ["accent", "bass", "snare", "low_tom", "mid_tom", "hi_tom", "rim_shot", "clap", "cow_bell",
              "cymbal", "open_hihat", "closed_hihat"],            # 0 off, 1 on
    "tr909": ["accent", "bass", "snare", "low_tom", "mid_tom", "hi_tom", "rim_shot", "clap", "closed_hihat",
              "open_hihat", "crash", "ride"],                      # 0 off, 1 on, 2 accent, 3 flam
}
MIXER_NAMES = {1: "comp_id", 2: "pcf_id"}
for _c, _ch in enumerate(CHANNELS):
    for _k, _n in enumerate(["level", "pan", "delay_send", "dist"]):
        MIXER_NAMES[6 + 6 * _c + _k] = "%s_%s" % (_ch, _n)

CHUNK_TAGS = {"delay": b"DELY", "pcf": b"PCF ", "dist": b"DIST", "comp": b"COMP"}
SIMPLE_SIZES = {"delay": 8, "pcf": 12, "dist": 8, "comp": 8}


@dataclass
class Params:
    """Plain parameter chunk: DELY, PCF, DIST, COMP."""
    kind: str = ""
    values: Dict[str, int] = field(default_factory=dict)
    _raw: bytes = raw_field()

    @classmethod
    def parse(cls, kind, raw):
        names = PARAMS[kind]
        return cls(kind, {n: raw[i] for i, n in enumerate(names)}, bytes(raw))

    def build(self):
        raw = bytearray(self._raw or bytes(SIMPLE_SIZES[self.kind]))
        for i, n in enumerate(PARAMS[self.kind]):
            raw[i] = self.values[n]
        return bytes(raw)


@dataclass
class Step303:
    note: int = 0                  # 0..12
    slide: int = 0                 # flags bit 0
    accent: int = 0                # bit 1
    up: int = 0                    # bit 2 (transpose up)
    down: int = 0                  # bit 3 (transpose down)
    rest: int = 1                  # bit 4: SET = rest (docs/song_load.md 4.6)
    flags_hi: int = 0              # bits 5..7, unused by the original (kept for round trip)


@dataclass
class Pattern303:
    shuffle: int = 0
    length: int = 16
    steps: List[Step303] = field(default_factory=lambda: [Step303() for _ in range(16)])


@dataclass
class Pattern808:
    """steps[s][i], i over DRUM_COLUMNS[kind]."""
    shuffle: int = 0
    length: int = 16
    steps: List[List[int]] = field(default_factory=lambda: [[0] * 12 for _ in range(16)])


@dataclass
class Tb303:
    params: Dict[str, int] = field(default_factory=dict)
    patterns: List[Pattern303] = field(default_factory=list)
    _raw: bytes = raw_field()

    SIZE = 1097

    @classmethod
    def parse(cls, raw):
        params = {n: raw[i] for i, n in enumerate(PARAMS["tb303"])}
        pats = []
        for p in range(32):
            o = 9 + 34 * p
            steps = []
            for s in range(16):
                note, fl = raw[o + 2 + 2 * s], raw[o + 3 + 2 * s]
                steps.append(Step303(note, fl & 1, fl >> 1 & 1, fl >> 2 & 1, fl >> 3 & 1, fl >> 4 & 1, fl >> 5))
            pats.append(Pattern303(raw[o], raw[o + 1], steps))
        return cls(params, pats, bytes(raw))

    def build(self):
        raw = bytearray(self._raw or bytes(self.SIZE))
        for i, n in enumerate(PARAMS["tb303"]):
            raw[i] = self.params[n]
        for p, pat in enumerate(self.patterns):
            o = 9 + 34 * p
            raw[o], raw[o + 1] = pat.shuffle, pat.length
            for s, st in enumerate(pat.steps):
                fl = st.slide | st.accent << 1 | st.up << 2 | st.down << 3 | st.rest << 4 | st.flags_hi << 5
                raw[o + 2 + 2 * s], raw[o + 3 + 2 * s] = st.note, fl
        return bytes(raw)


@dataclass
class Drums:
    """TR-808 (kind tr808, 6238 bytes) or TR-909 (kind tr909, 6239 bytes)."""
    kind: str = "tr808"
    params: Dict[str, int] = field(default_factory=dict)
    patterns: List[Pattern808] = field(default_factory=list)
    _raw: bytes = raw_field()

    SIZES = {"tr808": 6238, "tr909": 6239}

    @classmethod
    def parse(cls, kind, raw):
        base = 30 if kind == "tr808" else 31
        params = {n: raw[i] for i, n in enumerate(PARAMS[kind])}
        pats = []
        for p in range(32):
            o = base + 194 * p
            steps = [list(raw[o + 2 + 12 * s:o + 14 + 12 * s]) for s in range(16)]
            pats.append(Pattern808(raw[o], raw[o + 1], steps))
        return cls(kind, params, pats, bytes(raw))

    def build(self):
        base = 30 if self.kind == "tr808" else 31
        raw = bytearray(self._raw or bytes(self.SIZES[self.kind]))
        for i, n in enumerate(PARAMS[self.kind]):
            raw[i] = self.params[n]
        for p, pat in enumerate(self.patterns):
            o = base + 194 * p
            raw[o], raw[o + 1] = pat.shuffle, pat.length
            for s, st in enumerate(pat.steps):
                raw[o + 2 + 12 * s:o + 14 + 12 * s] = bytes(st)
        return bytes(raw)


@dataclass
class Event:
    pos: int                       # absolute, in 32nd notes (32 per bar); engine tick = pos * 24
    ctrl: int
    value: int


def encode_delta(v):
    """MIDI-style variable length quantity (RBS42.txt WriteDeltaPos)."""
    if v < 0 or v >= 1 << 28:
        raise ValueError("delta %d out of range" % v)
    out = [v & 0x7F]
    v >>= 7
    while v:
        out.append(0x80 | (v & 0x7F))
        v >>= 7
    return bytes(reversed(out))


def decode_delta(data, pos):
    """Return (value, next_pos) (RBS42.txt ReadDeltaPos, at most 4 bytes)."""
    c = data[pos]
    pos += 1
    v = c
    if c & 0x80:
        v &= 0x7F
        n = 1
        while True:
            c = data[pos]
            pos += 1
            n += 1
            v = (v << 7) | (c & 0x7F)
            if not c & 0x80:
                break
            if n >= 4:
                raise ValueError("delta position longer than 4 bytes")
    return v, pos


@dataclass
class Track:
    events: List[Event] = field(default_factory=list)
    _raw: bytes = raw_field()      # bytes after the last event, if any

    @classmethod
    def parse(cls, raw):
        n = get_u32(raw, 0)
        pos, p = 0, 4
        events = []
        for _ in range(n):
            d, p = decode_delta(raw, p)
            pos += d
            events.append(Event(pos, raw[p], raw[p + 1]))
            p += 2
        return cls(events, bytes(raw[p:]))

    def build(self):
        out = [struct.pack(">I", len(self.events))]
        prev = 0
        for e in self.events:
            if e.pos < prev:
                raise ValueError("events not sorted by position")
            out.append(encode_delta(e.pos - prev) + bytes([e.ctrl, e.value]))
            prev = e.pos
        out.append(self._raw)
        return b"".join(out)


@dataclass
class Song:
    head: Head = field(default_factory=Head)
    glob: Glob = field(default_factory=Glob)
    usri: Usri = field(default_factory=Usri)
    mixer: Mixer = field(default_factory=Mixer)
    delay: Params = None
    pcf: Params = None
    dist: Params = None
    comp: Params = None
    tb303: List[Tb303] = field(default_factory=list)   # two
    tr808: Drums = None
    tr909: Drums = None
    tracks: Dict[str, Track] = field(default_factory=dict)   # by TRACK_NAMES

    @classmethod
    def parse(cls, data):
        top = list(read_chunks(data, 0, len(data)))
        if len(top) != 1 or data[:4] != b"CAT " or data[8:12] != b"RB40":
            raise ValueError("not an RB40 song")
        _, start, size = top[0]
        items = list(read_chunks(data, start + 4, start + size))
        song = cls()
        cats = []
        for tag, s, n in items:
            body = data[s:s + n]
            if tag == b"HEAD":
                song.head = Head.parse(body)
            elif tag == b"GLOB":
                song.glob = Glob.parse(body)
            elif tag == b"USRI":
                song.usri = Usri.parse(body)
            elif tag == b"CAT ":
                cats.append((body[:4], s + 4, s + n))
            else:
                raise ValueError("unexpected chunk %r" % tag)
        kinds = {k: (a, b) for k, a, b in cats}
        devl = [(t, data[s:s + n]) for t, s, n in read_chunks(data, *kinds[b"DEVL"])]
        tags = [t for t, _ in devl]
        want = [b"MIXR", b"DELY", b"PCF ", b"DIST", b"COMP", b"303 ", b"303 ", b"808 ", b"909 "]
        if tags != want:
            raise ValueError("DEVL chunk order %r" % tags)
        b = [x for _, x in devl]
        song.mixer = Mixer.parse(b[0])
        song.delay = Params.parse("delay", b[1])
        song.pcf = Params.parse("pcf", b[2])
        song.dist = Params.parse("dist", b[3])
        song.comp = Params.parse("comp", b[4])
        song.tb303 = [Tb303.parse(b[5]), Tb303.parse(b[6])]
        song.tr808 = Drums.parse("tr808", b[7])
        song.tr909 = Drums.parse("tr909", b[8])
        traks = list(read_chunks(data, *kinds[b"TRKL"]))
        if len(traks) != 9 or any(t != b"TRAK" for t, _, _ in traks):
            raise ValueError("TRKL must hold 9 TRAK chunks")
        for name, (_, s, n) in zip(TRACK_NAMES, traks):
            song.tracks[name] = Track.parse(data[s:s + n])
        return song

    def build(self):
        devl = cat(b"DEVL", [
            chunk(b"MIXR", self.mixer.build()),
            chunk(b"DELY", self.delay.build()), chunk(b"PCF ", self.pcf.build()),
            chunk(b"DIST", self.dist.build()), chunk(b"COMP", self.comp.build()),
            chunk(b"303 ", self.tb303[0].build()), chunk(b"303 ", self.tb303[1].build()),
            chunk(b"808 ", self.tr808.build()), chunk(b"909 ", self.tr909.build())])
        trkl = cat(b"TRKL", [chunk(b"TRAK", self.tracks[n].build()) for n in TRACK_NAMES])
        return cat(b"RB40", [chunk(b"HEAD", self.head.build()), chunk(b"GLOB", self.glob.build()),
                             chunk(b"USRI", self.usri.build()), devl, trkl])


# ----------------------------------------------------------- controller names


def controller_names(track):
    """{id: name} of the controllers of a track."""
    if track == "mixer":
        return MIXER_NAMES
    kind = {"tb303_1": "tb303", "tb303_2": "tb303"}.get(track, track)
    return dict(enumerate(PARAMS[kind]))


def controller_id(track, ctrl):
    if isinstance(ctrl, int):
        return ctrl
    for k, v in controller_names(track).items():
        if v == ctrl:
            return k
    raise ValueError("track %s has no controller %r" % (track, ctrl))


def validate(song):
    """Check what RBS42.txt requires of a song; raise ValueError."""
    for name, tr in song.tracks.items():
        if not tr.events or tr.events[0].pos != 0:
            raise ValueError("track %s: the first event must exist at position 0" % name)
        names = controller_names(name)
        for e in tr.events:
            if e.pos > MAX_POS:
                raise ValueError("track %s: position %d beyond %d" % (name, e.pos, MAX_POS))
            if e.ctrl not in names:
                raise ValueError("track %s: unknown controller %d" % (name, e.ctrl))
            if not 0 <= e.value <= 127:
                raise ValueError("track %s: value %d" % (name, e.value))
            if e.ctrl == 1 and name in ("tb303_1", "tb303_2", "tr808", "tr909"):
                if e.pos % 32:
                    raise ValueError("track %s: pattern change at %d is not on a bar" % (name, e.pos))
                if e.value > 31:
                    raise ValueError("track %s: pattern %d (the app rejects 32)" % (name, e.value))


# ----------------------------------------------------------------------- JSON


def to_json(obj, track=None):
    if isinstance(obj, Song):
        d = {"version": obj.head.version}
        for f in fields(obj):
            if f.name == "tracks":
                d["tracks"] = {n: [{"pos": e.pos, "ctrl": controller_names(n).get(e.ctrl, e.ctrl),
                                    "value": e.value} for e in t.events] for n, t in obj.tracks.items()}
            elif f.name == "head":
                d["head"] = {"copyright": obj.head.copyright}
            else:
                d[f.name] = to_json(getattr(obj, f.name))
        return d
    if is_dataclass(obj):
        return {f.name: to_json(getattr(obj, f.name)) for f in fields(obj)
                if not f.name.startswith("_") and f.name != "kind"}
    if isinstance(obj, list):
        return [to_json(x) for x in obj]
    if isinstance(obj, dict):
        return {k: to_json(v) for k, v in obj.items()}
    return obj


def merge(base, spec, path=""):
    """Merge a spec over a JSON-shaped template, returning the result."""
    if isinstance(base, dict) and isinstance(spec, dict):
        out = dict(base)
        for k, v in spec.items():
            if k not in base and path != "tracks":
                raise ValueError("unknown field %s.%s" % (path, k))
            out[k] = merge(base.get(k), v, k if not path else path + "." + k) if k in base else v
        return out
    if isinstance(base, list) and isinstance(spec, dict) and spec and all(k.isdigit() for k in spec):
        out = list(base)
        for k, v in spec.items():
            out[int(k)] = merge(base[int(k)], v, "%s[%s]" % (path, k))
        return out
    return spec


def from_json(d):
    """Build a Song from a full JSON model (the output of to_json)."""
    song = Song()
    song.head = Head(version=d["version"], copyright=d["head"]["copyright"])
    song.glob = Glob(**d["glob"])
    song.usri = Usri(**d["usri"])
    m = d["mixer"]
    song.mixer = Mixer(m["master"], m["comp_id"], m["pcf_id"], [MixChannel(**c) for c in m["channels"]])
    for k in ("delay", "pcf", "dist", "comp"):
        setattr(song, k, Params(k, dict(d[k]["values"])))
    song.tb303 = [Tb303(dict(t["params"]), [
        Pattern303(p["shuffle"], p["length"], [Step303(**s) for s in p["steps"]]) for p in t["patterns"]])
        for t in d["tb303"]]
    for k in ("tr808", "tr909"):
        setattr(song, k, Drums(k, dict(d[k]["params"]), [
            Pattern808(p["shuffle"], p["length"], [list(s) for s in p["steps"]]) for p in d[k]["patterns"]]))
    for n in TRACK_NAMES:
        song.tracks[n] = Track([Event(e["pos"], controller_id(n, e["ctrl"]), e["value"]) for e in d["tracks"][n]])
    return song


def restore_raw(song, tmpl):
    """Carry the template's hidden raw bytes into a song built from JSON."""
    for name in ("head", "glob", "usri", "mixer", "delay", "pcf", "dist", "comp", "tr808", "tr909"):
        getattr(song, name)._raw = getattr(tmpl, name)._raw
    for a, b in zip(song.tb303, tmpl.tb303):
        a._raw = b._raw
    for n in TRACK_NAMES:
        song.tracks[n]._raw = tmpl.tracks[n]._raw


def convert_template_version(tmpl, version):
    """808 tom tunings changed meaning at 4.2: v/2+32 when loading a file older than 4.2 (sub_432E1B)."""
    if tmpl.head.version == version:
        return
    if (tmpl.head.version, version) != ("4.1", "4.2"):
        raise ValueError("only a 4.1 -> 4.2 template conversion is known")
    for n in ("low_tom_tuning", "mid_tom_tuning", "hi_tom_tuning"):
        tmpl.tr808.params[n] = tmpl.tr808.params[n] // 2 + 32


def read_bytes(path):
    with open(path, "rb") as f:
        return f.read()


def read_song(path):
    return Song.parse(read_bytes(path))


def build_song(spec, template_path=None):
    tmpl = read_song(template_path or DEFAULT_TEMPLATE)
    spec = dict(spec)
    version = spec.pop("version", "4.2")
    convert_template_version(tmpl, version)
    tmpl.head.version = version
    merged = merge(to_json(tmpl), spec)
    song = from_json(merged)
    restore_raw(song, tmpl)
    validate(song)
    return song


# ----------------------------------------------------------------------- dump


def dump(song):
    L = []
    g = song.glob
    L.append("version %s  %r" % (song.head.version, song.head.copyright))
    L.append("mode %s  loop %s  tempo %.3f  loop %d..%d (bars %.2f..%.2f)  shuffle %d  vintage %d" % (
        "song" if g.song_mode else "pattern", g.loop, g.tempo / 1000, g.loop_start, g.loop_end,
        g.loop_start / 768, g.loop_end / 768, g.shuffle, g.vintage))
    L.append("mod %r  ftp %r  www %r" % (g.mod_name, g.mod_ftp, g.mod_www))
    u = song.usri
    L.append("usri title %r web %r show %d text %r" % (u.window_title, u.web_page, u.show_on_open, u.text))
    m = song.mixer
    L.append("mixer master %d comp_id %d pcf_id %d" % (m.master, m.comp_id, m.pcf_id))
    for n, c in zip(CHANNELS, m.channels):
        L.append("  %-8s enabled %d level %3d pan %3d delay_send %3d dist %d" % (
            n, c.enabled, c.level, c.pan, c.delay_send, c.dist))
    for k in ("delay", "pcf", "dist", "comp"):
        L.append("%s %s" % (k, " ".join("%s=%d" % kv for kv in getattr(song, k).values.items())))
    for i, t in enumerate(song.tb303):
        L.append("tb303_%d %s" % (i + 1, " ".join("%s=%d" % kv for kv in t.params.items())))
        for p, pat in enumerate(t.patterns):
            if all(s.rest for s in pat.steps):
                continue
            notes = " ".join(("-" if s.rest else "%d%s%s%s%s" % (
                s.note, "s" if s.slide else "", "a" if s.accent else "", "^" if s.up else "",
                "v" if s.down else "")) for s in pat.steps[:pat.length])
            L.append("  pattern %2d shuffle %d len %2d: %s" % (p, pat.shuffle, pat.length, notes))
    for k in ("tr808", "tr909"):
        d = getattr(song, k)
        L.append("%s %s" % (k, " ".join("%s=%d" % kv for kv in d.params.items())))
        for p, pat in enumerate(d.patterns):
            if not any(any(s) for s in pat.steps):
                continue
            L.append("  pattern %2d shuffle %d len %2d (%s)" % (p, pat.shuffle, pat.length, ",".join(DRUM_COLUMNS[k])))
            for s in range(pat.length):
                L.append("    %2d %s" % (s, "".join(".123456789"[v] if v < 10 else "?" for v in pat.steps[s])))
    for n in TRACK_NAMES:
        t = song.tracks[n]
        names = controller_names(n)
        L.append("track %s: %d events" % (n, len(t.events)))
        for e in t.events:
            L.append("  pos %5d (bar %3d.%02d) %-22s %d" % (
                e.pos, e.pos // 32 + 1, e.pos % 32, names.get(e.ctrl, "0x%02x" % e.ctrl), e.value))
    return "\n".join(L)


def main(argv):
    if len(argv) >= 3 and argv[1] == "dump":
        print(dump(read_song(argv[2])))
    elif len(argv) >= 3 and argv[1] == "json":
        print(json.dumps(to_json(read_song(argv[2])), indent=1))
    elif len(argv) == 4 and argv[1] == "build":
        with open(argv[2]) as f:
            spec = json.load(f)
        data = build_song(spec).build()
        with open(argv[3], "wb") as f:
            f.write(data)
    elif len(argv) >= 3 and argv[1] == "check":
        bad = 0
        for f in argv[2:]:
            data = read_bytes(f)
            ok = Song.parse(data).build() == data
            bad += not ok
            print("%s %s" % ("ok " if ok else "BAD", f))
        return 1 if bad else 0
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))

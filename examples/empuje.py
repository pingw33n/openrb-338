#!/usr/bin/python3
"""Micro texture under a dance beat that comes and goes.

Two corrections to the previous piece:

1. Its intro was fifteen seconds of pulse, which is indefensible. This one is
   built out of four-bar blocks, so nothing holds a shape for longer than eight
   seconds, and the first block already has the beat in it.
2. Its level curve was flat. The beat here enters, drops out, returns harder,
   and leaves before the end; the micro texture stays underneath throughout, so
   the piece has something to hold on to while the drums change.

The micro layer is what the last piece found: a figure varied by one step
across copies, a rim on a different clock, and a global shuffle of 96, which is
a sixteen-tick delay of the off-16ths rather than a swing.

Usage:  python3 /tmp/make-driven.py OUT.json
"""
import json
import sys

BAR = 32
TEMPO = 118
BARS = 64
SHUFFLE = 96

C909 = {"accent": 0, "bass": 1, "snare": 2, "low_tom": 3, "mid_tom": 4,
        "hi_tom": 5, "rim": 6, "clap": 7, "chh": 8, "ohh": 9,
        "crash": 10, "ride": 11}
C808 = {"accent": 0, "bass": 1, "snare": 2, "low_tom": 3, "mid_tom": 4,
        "hi_tom": 5, "rim": 6, "clap": 7, "cow": 8, "cymbal": 9,
        "ohh": 10, "chh": 11}


def rows(machine, pattern):
    c = C909 if machine == "909" else C808
    out = {}
    for step, hits in enumerate(pattern):
        if hits:
            r = [0] * 12
            for col, val in hits.items():
                r[c[col]] = val
            out[str(step)] = r
    return out


def note(p, accent=0):
    return {"note": p, "rest": 0, "accent": accent, "slide": 0}


R = None


def steps303(p):
    return {str(i): s for i, s in enumerate(p) if s is not None}


def at(bar, step=0):
    return bar * BAR + step


def knob(ctrl, value, bar=0, step=0):
    return {"pos": at(bar, step), "ctrl": ctrl, "value": value}


def frames(semitones):
    return max(0, 18 + int(round(semitones * 4.5)))

# -- the dance beat: three depths, so it can come and go -------------------
# A: kick and a rim only. The beat is present but thin.
# B: four on the floor with hats, the body of the piece.
# C: the same with an open hat and a clap, the peak.
# D: nothing but a kick on one, for the gaps.

def dance(kick, snare, hats, extra=()):
    p = [None] * 16
    for i in hats:
        p[i] = {"chh": 1}
    for i in kick:
        p[i] = dict(p[i] or {}, bass=1)
    for i in snare:
        p[i] = dict(p[i] or {}, snare=1)
    for i, k in extra:
        p[i] = dict(p[i] or {}, **{k: 1})
    return p


BEAT_THIN = dance([0, 8], [], [4, 12], [(6, "rim")])
BEAT_BODY = dance([0, 4, 8, 12], [4, 12], [2, 6, 10, 14])
BEAT_PEAK = dance([0, 4, 8, 12], [4, 12], [2, 6, 10],
                  [(14, "ohh"), (12, "clap")])
BEAT_GAP = dance([0], [], [], [])

# -- the micro layer: one figure, varied by one step ----------------------
# The toms carry a two-hit cell whose second hit drifts by one step each
# variant, and a rim that sits on its own clock.

def kit(toms, rims=(), accents=()):
    p = [None] * 16
    for i in rims:
        p[i] = {"rim": 1}
    for i, t in toms:
        p[i] = dict(p[i] or {}, **{t: 1})
    for i in accents:
        if p[i]:
            p[i] = dict(p[i], accent=1)
    return p


MICRO_0 = kit([(2, "low_tom"), (11, "mid_tom")], rims=[7], accents=[11])
MICRO_1 = kit([(2, "low_tom"), (12, "mid_tom")], rims=[7], accents=[12])
MICRO_2 = kit([(3, "low_tom"), (12, "mid_tom")], rims=[8], accents=[12])
MICRO_3 = kit([(3, "low_tom"), (10, "hi_tom")], rims=[8], accents=[10])


# -- the 303: one note that drifts, as before -----------------------------

def one(step, p):
    s = [R] * 16
    s[step] = note(p)
    return s


DRIFT_0 = one(6, 0)
DRIFT_1 = one(7, 7)
DRIFT_2 = one(5, 4)


# The James Brown half: the bass does not leave, it takes over. From a pulse
# on one, to a slap figure that fills the bar, and it keeps going to the last
# beat. Accent on the one, as Brown's bassists did.
def slap(pairs):
    s = [R] * 16
    for i, p in pairs:
        s[i] = note(p, accent=1 if i == 0 else 0)
    return s


# The corpus' own vocabulary, from G-Spot, Calm and Transfusion: 0 (the root),
# 7 (the fifth) and 4 (the third) carry most of the notes, with 3 and 9 turning
# up as colour. Root, fifth and third is a minor outline that still moves, and
# the 3 next to the 4 makes the mode ambiguous, which is darker without being
# sad.
SLAP_1 = slap([(0, 0), (4, 7), (8, 0), (12, 4)])
SLAP_2 = slap([(0, 0), (2, 7), (4, 0), (6, 4), (8, 7), (10, 0), (12, 3), (14, 7)])


spec = {
    "version": "4.2",
    "usri": {
        "window_title": "Empuje",
        "text": "A dance beat that enters and leaves over a micro layer.",
        "web_page": "docs/beats.md",
    },
    "glob": {
        "tempo": TEMPO * 1000, "song_mode": 1, "loop": 1, "loop_start": 0,
        "loop_end": BARS * 768, "shuffle": SHUFFLE, "vintage": 0,
    },
    "mixer": {
        "master": 112, "comp_id": 0, "pcf_id": 0,
        "channels": [
            {"enabled": 1, "level": 86, "pan": 72, "delay_send": 96, "dist": 0},
            {"enabled": 1, "level": 82, "pan": 52, "delay_send": 26, "dist": 0},
            {"enabled": 1, "level": 96, "pan": 60, "delay_send": 22, "dist": 0},
            {"enabled": 1, "level": 84, "pan": 44, "delay_send": 0, "dist": 0},
        ],
    },
    "delay": {"values": {"enabled": 1, "steps": 4, "mode": 1,
                         "feedback": 56, "pan": 46}},
    "pcf": {"values": {"enabled": 1, "frequency": 22, "resonance": 44,
                       "amount": 20, "wave": 4, "decay": 60, "mode": 0}},
    "dist": {"values": {"enabled": 1, "amount": 58, "shape": 60}},
    "comp": {"values": {"enabled": 1, "amount": 110, "threshold": 40}},

    # the 909 carries the dance beat
    "tr909": {
        "params": {"enabled": 1, "pattern": 0, "accent_level": 40,
                   "bass_level": 108, "bass_tune": 54, "bass_attack": 64,
                   "bass_decay": 96, "snare_level": 92, "snare_tune": 60,
                   "snare_tone": 56, "snare_snappy": 70,
                   "closed_hihat_decay": 24, "open_hihat_decay": 80,
                   "hihat_level": 70, "rim_shot_level": 64, "clap_level": 80},
        "patterns": {
            "0": {"length": 16, "steps": rows("909", BEAT_GAP)},
            "1": {"length": 16, "steps": rows("909", BEAT_THIN)},
            "2": {"length": 16, "steps": rows("909", BEAT_BODY)},
            "3": {"length": 16, "steps": rows("909", BEAT_PEAK)},
        },
    },
    # the 808 is the micro layer: toms and rims, tuned apart
    "tr808": {
        "params": {"enabled": 1, "pattern": 0,
                   "low_tom_level": 108, "low_tom_tuning": frames(-5),
                   "low_tom_selector": 0,
                   "mid_tom_level": 100, "mid_tom_tuning": frames(0),
                   "mid_tom_selector": 0,
                   "hi_tom_level": 92, "hi_tom_tuning": frames(6),
                   "hi_tom_selector": 0,
                   "bass_level": 60, "bass_tone": 40, "bass_decay": 100,
                   "snare_level": 40, "snare_tone": 40, "snare_snappy": 22,
                   "closed_hihat_level": 30, "rim_shot_level": 58,
                   "cow_bell_level": 24},
        "patterns": {
            "0": {"length": 16, "steps": rows("808", MICRO_0)},
            "1": {"length": 16, "steps": rows("808", MICRO_1)},
            "2": {"length": 16, "steps": rows("808", MICRO_2)},
            "3": {"length": 16, "steps": rows("808", MICRO_3)},
        },
    },
    "tb303": {
        "0": {
            "params": {"enabled": 1, "pattern": 0, "tune": 68, "waveform": 1,
                       "cutoff": 84, "resonance": 84, "env_mod": 30,
                       "decay": 62, "accent": 118},
            "patterns": {
                "0": {"length": 16, "steps": steps303(DRIFT_0)},
                "1": {"length": 16, "steps": steps303(DRIFT_1)},
                "2": {"length": 16, "steps": steps303(DRIFT_2)},
                "3": {"length": 16, "steps": steps303(SLAP_1)},
                "4": {"length": 16, "steps": steps303(SLAP_2)},
            },
        },
    },
    "tracks": {},
}



# -- the arrangement: four-bar blocks -------------------------------------
# Bar 0 already has the beat. Then it thins, returns, peaks, drops and returns.
# Nothing holds a shape longer than eight bars.

tr909 = [
    # the beat is there from the first bar: no counted-in intro
    knob("pattern", 1), knob("bass_level", 108, 0),
    knob("pattern", 2, 4),        # body
    knob("pattern", 3, 12),       # peak
    knob("pattern", 1, 20),       # drop to thin
    knob("pattern", 0, 24),       # a gap of four bars
    knob("pattern", 2, 28),       # back, harder
    knob("pattern", 3, 36),
    knob("pattern", 2, 44),
    knob("pattern", 1, 52),       # thin out
    knob("pattern", 0, 60),       # gone at the end
    knob("bass_level", 108), knob("bass_tune", 54),
    knob("bass_attack", 64), knob("bass_decay", 96),
    knob("snare_level", 92), knob("snare_tune", 60),
    knob("snare_tone", 56), knob("snare_snappy", 70),
    knob("closed_hihat_decay", 24), knob("open_hihat_decay", 80),
    knob("hihat_level", 70), knob("rim_shot_level", 64),
    knob("clap_level", 80), knob("accent_level", 40),
    knob("hihat_level", 84, 12), knob("hihat_level", 56, 44),
]

# The micro layer never stops: it is what holds the piece while the beat moves.
tr808 = [knob("pattern", 0)]
for i, bar in enumerate(range(4, 64, 8)):
    tr808.append(knob("pattern", (i + 1) % 4, bar))
tr808 += [
    knob("low_tom_level", 108), knob("low_tom_tuning", frames(-5)),
    knob("mid_tom_level", 100), knob("mid_tom_tuning", frames(0)),
    knob("hi_tom_level", 92), knob("hi_tom_tuning", frames(6)),
    knob("bass_level", 60), knob("bass_tone", 40), knob("bass_decay", 100),
    knob("snare_level", 40), knob("snare_tone", 40), knob("snare_snappy", 22),
    knob("closed_hihat_level", 30), knob("rim_shot_level", 58),
    # when the beat leaves, the micro layer comes forward
    
]

bass = [
    knob("pattern", 0), knob("pattern", 1, 16),
    knob("pattern", 2, 32),
    # from bar 40 the bass takes over and never leaves: 3, then 4, then 5
    knob("pattern", 3, 52), knob("pattern", 4, 58),
    knob("waveform", 1), knob("tune", 68),
    knob("resonance", 84), knob("env_mod", 30), knob("decay", 62),
    knob("accent", 118),
    knob("cutoff", 28), knob("cutoff", 36, 12), knob("cutoff", 24, 28),
    knob("cutoff", 48, 52), knob("cutoff", 76, 58),
]

delay = [
    knob("enabled", 1), knob("steps", 4), knob("mode", 1),
    knob("feedback", 56), knob("pan", 46),
    knob("steps", 3, 24), knob("steps", 5, 48),
]

mixer = [
    knob("tb303_1_level", 86), knob("tb303_1_pan", 72),
    knob("tb303_1_delay_send", 124),
    knob("tr808_level", 82), knob("tr808_pan", 52),
    knob("tr808_delay_send", 26),
    # the micro layer comes forward in the gaps and falls back under the beat
    knob("tr808_level", 112, 22),
    knob("tr808_level", 112, 56),
    knob("tr909_level", 96), knob("tr909_pan", 60),
    # the beat drops out and returns, which no pattern alone achieves
    knob("tr909_level", 34, 20), knob("tr909_level", 0, 24),
    knob("tr909_level", 110, 28), knob("tr909_level", 104, 36),
    knob("tr909_level", 90, 44), knob("tr909_level", 40, 52),
    knob("tr909_level", 0, 60),
    knob("tr909_level", 84, 44),
]

for ev in (tr909, tr808, bass, delay, mixer):
    ev.sort(key=lambda e: e["pos"])

spec["tracks"] = {
    "tr909": tr909, "tr808": tr808, "tb303_1": bass,
    "delay": delay, "mixer": mixer,
}

out = sys.argv[1] if len(sys.argv) > 1 else "driven.json"
with open(out, "w") as f:
    json.dump(spec, f, indent=1)
print("%s: %d bars at %d BPM" % (out, BARS, TEMPO))


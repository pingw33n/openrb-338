#!/usr/bin/python3
"""Generate the diagnostic songs' JSON specs (specs/*.json) and the README.

    gen_specs.py            write specs/NAME.json for every song and README.md

Every song is a few bars in song mode (Export is greyed in pattern mode, docs/test_machine.md
section 8) with the loop set, USRI show_on_open 0 (no info box).  In song mode the TRAK events at the
loop start set every automatable value (docs/song_load.md section 6), so each song carries an event
at position 0 for every controller of every track, computed from the same parameter values that fill
the device chunks.  Position unit of events: a 32nd note, 32 per bar.

A song is described by a Song object: parameter dicts per device, patterns, extra events.  Starting
point of every song: silence (all patterns empty), 120 BPM, a 2-bar loop from bar 1.
"""
import json
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(HERE)))
sys.path.insert(0, os.path.join(ROOT, "tools"))
import rbs  # noqa: E402

BAR = 768          # engine ticks per bar (loop start/end in GLOB)
B32 = 32           # TRAK positions per bar
TRACKS = rbs.TRACK_NAMES
CHANNELS = ["tb303_1", "tb303_2", "tr808", "tr909"]
COL808 = rbs.DRUM_COLUMNS["tr808"]
COL909 = rbs.DRUM_COLUMNS["tr909"]

# ---------------------------------------------------------------- base values
BASE = {
    "tb303": {"enabled": 1, "pattern": 0, "tune": 64, "cutoff": 64, "resonance": 0, "env_mod": 64,
              "decay": 64, "accent": 64, "waveform": 0},
    "delay": {"enabled": 1, "steps": 3, "mode": 1, "feedback": 0, "pan": 64},
    "pcf": {"enabled": 1, "frequency": 64, "resonance": 64, "amount": 64, "wave": 64, "decay": 64, "mode": 1},
    "dist": {"enabled": 1, "amount": 64, "shape": 0},
    "comp": {"enabled": 1, "amount": 64, "threshold": 64},
}
# drum knobs start from the template song's values
_TEMPLATE = json.loads(subprocess.check_output(
    [sys.executable, os.path.join(ROOT, "tools", "rbs.py"), "json",
     os.path.join(ROOT, "tests", "songs", "shipped", "Silent Default Song.rbs")]))
DRUM_BASE = {k: dict(_TEMPLATE[k]["params"]) for k in ("tr808", "tr909")}


class Song:
    def __init__(self, name, what, bars=2, tempo=120000, loop_start=0):
        self.name, self.what = name, what
        self.glob = {"song_mode": 1, "loop": 1, "tempo": tempo, "loop_start": loop_start * BAR,
                     "loop_end": (loop_start + bars) * BAR, "shuffle": 0, "vintage": 0}
        self.mixer = {"master": 100, "comp_id": 0, "pcf_id": 0,
                      "channels": [{"enabled": 1, "level": 100, "pan": 64, "delay_send": 0, "dist": 0}
                                   for _ in range(4)]}
        self.dev = {k: dict(v) for k, v in BASE.items() if k != "tb303"}
        self.t303 = [dict(BASE["tb303"]), dict(BASE["tb303"])]
        self.drum = {k: dict(v) for k, v in DRUM_BASE.items()}
        self.pat303 = [{}, {}]            # index -> pattern dict
        self.patdrum = {"tr808": {}, "tr909": {}}
        self.events = {n: [] for n in TRACKS}   # extra (pos, ctrl name, value)

    def p303(self, steps, n=0, idx=0, shuffle=0):
        """steps: 16 entries, None = rest or (note, flags) with flags from 's' slide, 'a' accent, 'u' up, 'd' down."""
        out = []
        for s in steps:
            if s is None:
                out.append({"note": 0, "slide": 0, "accent": 0, "up": 0, "down": 0, "rest": 1})
                continue
            note, fl = s
            out.append({"note": note, "slide": int("s" in fl), "accent": int("a" in fl), "up": int("u" in fl),
                        "down": int("d" in fl), "rest": 0})
        self.pat303[n][idx] = {"shuffle": shuffle, "length": len(out), "steps": out}

    def pdrum(self, kind, cols, hits, idx=0, shuffle=0, value=1, accent=()):
        """The columns in cols play on the steps in hits (value: 1, or a {step: value} dict);
        accent: steps with the AC column set (2 on the 909)."""
        names = rbs.DRUM_COLUMNS[kind]
        steps = [[0] * 12 for _ in range(16)]
        for h in hits:
            for c in cols:
                steps[h][names.index(c)] = value[h] if isinstance(value, dict) else value
        for h in accent:
            steps[h][0] = 2 if kind == "tr909" else 1
        self.patdrum[kind][idx] = {"shuffle": shuffle, "length": 16, "steps": steps}

    def pdrum_cells(self, kind, cells, idx=0):
        """cells: (step, instrument, value)."""
        names = rbs.DRUM_COLUMNS[kind]
        steps = [[0] * 12 for _ in range(16)]
        for h, ins, v in cells:
            steps[h][names.index(ins)] = v
        self.patdrum[kind][idx] = {"shuffle": 0, "length": 16, "steps": steps}

    def ev(self, track, pos32, ctrl, value):
        self.events[track].append((pos32, ctrl, value))

    def initial(self, track):
        if track == "mixer":
            d = {"comp_id": self.mixer["comp_id"], "pcf_id": self.mixer["pcf_id"]}
            for c, ch in zip(CHANNELS, self.mixer["channels"]):
                for k in ("level", "pan", "delay_send", "dist"):
                    d["%s_%s" % (c, k)] = ch[k]
            return d
        if track in ("tb303_1", "tb303_2"):
            return self.t303[int(track[-1]) - 1]
        if track in ("tr808", "tr909"):
            return self.drum[track]
        return self.dev[track]

    def spec(self):
        s = {"version": "4.2", "glob": self.glob, "usri": {"show_on_open": 0}, "mixer": self.mixer}
        for k in ("delay", "pcf", "dist", "comp"):
            s[k] = {"values": self.dev[k]}
        s["tb303"] = {str(i): {"params": self.t303[i], "patterns": {str(k): v for k, v in self.pat303[i].items()}}
                      for i in range(2)}
        for k in ("tr808", "tr909"):
            s[k] = {"params": self.drum[k], "patterns": {str(i): v for i, v in self.patdrum[k].items()}}
        s["tracks"] = {}
        for tr in TRACKS:
            evs = [{"pos": 0, "ctrl": c, "value": v} for c, v in self.initial(tr).items()]
            evs += [{"pos": p, "ctrl": c, "value": v} for p, c, v in sorted(self.events[tr], key=lambda e: e[0])]
            s["tracks"][tr] = evs
        return s


SONGS = []


def add(song):
    SONGS.append(song)
    return song


# ---------------------------------------------------------------- 303 material
NOTES = [0, 0, 12, 0, 3, 0, 7, 10, 0, 0, 12, 0, 5, 3, 0, 7]
NOTES2 = [7, 7, 3, 7, 0, 12, 7, 5, 7, 7, 3, 7, 10, 7, 0, 3]
NOTES3 = [12, 12, 7, 12, 5, 12, 3, 0, 12, 12, 7, 12, 5, 12, 3, 0]
RESTS = {3, 7, 11, 15}
SLIDES = (1, 2, 5, 6, 9, 10, 13, 14)


def plain303(s, n=0, idx=0, accents=(), slides=(), shuffle=0, notes=NOTES):
    steps = []
    for i in range(16):
        if i in RESTS and i not in slides:
            steps.append(None)
            continue
        fl = ("a" if i in accents else "") + ("s" if i in slides else "") + ("u" if i % 8 == 6 else "")
        steps.append((notes[i], fl))
    s.p303(steps, n, idx, shuffle)


def s303(name, what, wave=0, cutoff=64, res=0, env=64, decay=64, acc=64, tune=64, vint=0, accents=(),
         slides=(), bars=2, tempo=120000):
    s = Song(name, what, bars, tempo)
    s.glob["vintage"] = vint
    s.t303[0].update(waveform=wave, cutoff=cutoff, resonance=res, env_mod=env, decay=decay, accent=acc, tune=tune)
    plain303(s, accents=accents, slides=slides)
    return add(s)


add(Song("silence", "Nothing plays: every pattern empty. Expect all-zero samples (the 1e-6 offsets truncate to 0)."))

for wave, wn in ((0, "saw"), (1, "square")):
    for co, cn in ((20, "lowcut"), (110, "highcut")):
        s303("303_%s_%s" % (wn, cn), "TB-303 #1 plain 16-step line (rests on steps 3,7,11,15), %s wave, cutoff %d, "
             "resonance 0." % (wn, co), wave=wave, cutoff=co)
for r in (0, 64, 127):
    s303("303_res%d" % r, "TB-303 #1 saw line, cutoff 50, envmod 90, resonance %d." % r, cutoff=50, env=90, res=r)
s303("303_env0", "TB-303 #1 saw line, cutoff 40, envmod 0 (no envelope sweep), decay 64.", cutoff=40, env=0)
s303("303_env127_decay0", "TB-303 #1 saw line, cutoff 40, envmod 127, decay 0 (shortest).", cutoff=40, env=127,
     decay=0)
s303("303_env127_decay127", "TB-303 #1 saw line, cutoff 40, envmod 127, decay 127 (longest).", cutoff=40,
     env=127, decay=127)
s303("303_accent0", "TB-303 #1 saw line with accented steps 0,4,8,12, accent knob 0.", cutoff=50, env=90, res=64,
     acc=0, accents=(0, 4, 8, 12))
s303("303_accent127", "TB-303 #1 saw line with accented steps 0,4,8,12, accent knob 127.", cutoff=50, env=90,
     res=64, acc=127, accents=(0, 4, 8, 12))
s303("303_slide", "TB-303 #1 saw line with slides on steps 1,2,5,6,9,10,13,14 (the rests on 3,7,11,15 are "
     "kept), resonance 64.", cutoff=60, env=80, res=64, slides=SLIDES)
s303("303_slide_accent", "TB-303 #1 slides (as 303_slide) and accents on steps 0,2,8,10 together.", cutoff=60,
     env=80, res=64, acc=100, accents=(0, 2, 8, 10), slides=SLIDES)
s303("303_tune_lo", "TB-303 #1 saw line, tune knob 0.", tune=0)
s303("303_tune_hi", "TB-303 #1 saw line, tune knob 127.", tune=127)
for vn, kw in (("saw", {}), ("square", {"wave": 1}), ("res127", {"cutoff": 50, "env": 90, "res": 127}),
               ("accent", {"cutoff": 50, "env": 90, "res": 64, "acc": 100, "accents": (0, 4, 8, 12)}),
               ("slide", {"cutoff": 60, "env": 80, "res": 64, "slides": SLIDES}),
               ("lowcut", {"cutoff": 20}), ("highcut", {"cutoff": 110})):
    s303("303_vintage_" + vn, "Vintage ReBirth synth sound (GLOB byte 482 = 1): TB-303 #1 line, %s variant "
         "of the 2.0-sound song of the same kind (inline engine 0x10023F10 instead of 0x100248C4)." % vn,
         vint=1, **kw)

s = add(Song("303_two_voices", "Both TB-303s at once: #1 saw line panned left (20), #2 square line panned right "
             "(107); the second device and the pan law."))
s.t303[1].update(waveform=1, cutoff=70, resonance=40)
plain303(s)
plain303(s, n=1, notes=NOTES2)
s.mixer["channels"][0]["pan"] = 20
s.mixer["channels"][1]["pan"] = 107

# ---------------------------------------------------------------- drums
D808 = ["bass", "snare", "low_tom", "mid_tom", "hi_tom", "rim_shot", "clap", "cow_bell", "cymbal", "open_hihat",
        "closed_hihat"]
D909 = ["bass", "snare", "low_tom", "mid_tom", "hi_tom", "rim_shot", "clap", "closed_hihat", "open_hihat",
        "crash", "ride"]
HITS = (0, 4, 8, 12)

for ins in D808:
    s = add(Song("808_%s" % ins, "TR-808 %s alone at the template's knobs, hits on steps 0,4,8,12 of a 16-step "
                 "pattern." % ins.replace("_", " ")))
    s.pdrum("tr808", [ins], HITS)
for ins in D909:
    s = add(Song("909_%s" % ins, "TR-909 %s alone at the template's knobs, hits on steps 0,4,8,12 of a 16-step "
                 "pattern." % ins.replace("_", " ")))
    s.pdrum("tr909", [ins], HITS)


def knobs(kind, name, what, ins, **kw):
    s = add(Song(name, what))
    s.pdrum(kind, ins if isinstance(ins, list) else [ins], HITS)
    s.drum[kind].update(kw)
    return s


for v in (0, 127):
    t = "lo" if v == 0 else "hi"
    knobs("tr808", "808_bass_knobs_" + t, "TR-808 bass with tone and decay at %d." % v, "bass", bass_tone=v,
          bass_decay=v)
    knobs("tr808", "808_snare_knobs_" + t, "TR-808 snare with tone and snappy at %d." % v, "snare", snare_tone=v,
          snare_snappy=v)
    knobs("tr808", "808_lowtom_knobs_" + t, "TR-808 low tom with tuning %d and selector %d." % (v, v // 127),
          "low_tom", low_tom_tuning=v, low_tom_selector=v // 127)
    knobs("tr808", "808_cymbal_knobs_" + t, "TR-808 cymbal with tone and decay at %d." % v, "cymbal",
          cymbal_tone=v, cymbal_decay=v)
    knobs("tr808", "808_openhat_knobs_" + t, "TR-808 open hi-hat with decay %d." % v, "open_hihat",
          open_hihat_decay=v)
    knobs("tr808", "808_clap_knobs_" + t, "TR-808 clap with level %d and selector %d." % (max(v, 20), v // 127),
          "clap", clap_level=max(v, 20), clap_selector=v // 127)
    knobs("tr909", "909_bass_knobs_" + t, "TR-909 bass with tune, attack and decay at %d." % v, "bass",
          bass_tune=v, bass_attack=v, bass_decay=v)
    knobs("tr909", "909_snare_knobs_" + t, "TR-909 snare with tune, tone and snappy at %d." % v, "snare",
          snare_tune=v, snare_tone=v, snare_snappy=v)
    knobs("tr909", "909_hightom_knobs_" + t, "TR-909 high tom with tune and decay at %d." % v, "hi_tom",
          hi_tom_tune=v, hi_tom_decay=v)
    knobs("tr909", "909_closedhat_knobs_" + t, "TR-909 closed hi-hat with decay %d." % v, "closed_hihat",
          closed_hihat_decay=v)
    knobs("tr909", "909_openhat_knobs_" + t, "TR-909 open hi-hat with decay %d." % v, "open_hihat",
          open_hihat_decay=v)
    s = add(Song("909_crash_ride_knobs_" + t, "TR-909 crash (steps 0,8) and ride (steps 4,12) with both tunes "
                 "at %d." % v))
    s.pdrum_cells("tr909", [(0, "crash", 1), (8, "crash", 1), (4, "ride", 1), (12, "ride", 1)])
    s.drum["tr909"].update(crash_tune=v, ride_tune=v)
for v in (0, 127):
    s = add(Song("808_accent_level%d" % v, "TR-808 bass and snare on steps 0,4,8,12, the AC column set on steps "
                 "0 and 8, accent knob %d." % v))
    s.pdrum("tr808", ["bass", "snare"], HITS, accent=(0, 8))
    s.drum["tr808"]["accent_level"] = v
    s = add(Song("909_accent_level%d" % v, "TR-909 bass and snare on steps 0,4,8,12, AC column 2 on steps 0 and "
                 "8, steps 4 and 12 with value 2 (accented step); accent knob %d." % v))
    s.pdrum("tr909", ["bass", "snare"], HITS, accent=(0, 8), value={0: 1, 4: 2, 8: 1, 12: 2})
    s.drum["tr909"]["accent_level"] = v
for v in (0, 64, 127):
    s = add(Song("909_flam_interval%d" % v, "TR-909 snare and clap with flam (step value 3) on steps 0,4,8,12, "
                 "flam interval %d." % v))
    s.pdrum("tr909", ["snare", "clap"], HITS, value=3)
    s.drum["tr909"]["flam_interval"] = v

s = add(Song("drums_kit", "Both drum machines with many instruments in dense 16-step patterns: voice summing, "
             "the 808 and 909 channel strips, 909 values 1/2/3."))
s.pdrum_cells("tr808", [(0, "bass", 1), (8, "bass", 1), (4, "snare", 1), (12, "snare", 1), (4, "clap", 1),
                        (12, "clap", 1), (2, "closed_hihat", 1), (6, "closed_hihat", 1), (10, "closed_hihat", 1),
                        (14, "open_hihat", 1), (3, "cow_bell", 1), (11, "cow_bell", 1), (13, "low_tom", 1),
                        (14, "mid_tom", 1), (15, "hi_tom", 1), (0, "cymbal", 1), (7, "rim_shot", 1),
                        (0, "accent", 1)])
s.pdrum_cells("tr909", [(0, "bass", 1), (8, "bass", 2), (4, "snare", 1), (12, "snare", 3), (4, "clap", 1),
                        (2, "closed_hihat", 1), (6, "closed_hihat", 2), (10, "open_hihat", 1),
                        (14, "closed_hihat", 1), (0, "crash", 1), (8, "ride", 1), (11, "rim_shot", 1),
                        (13, "low_tom", 1), (14, "mid_tom", 1), (15, "hi_tom", 1)])


# ---------------------------------------------------------------- effects
def src303(s, **kw):
    s.t303[0].update(waveform=0, cutoff=70, resonance=30, env_mod=70)
    s.t303[0].update(kw)
    plain303(s, accents=(0, 8))


def src808(s):
    s.pdrum_cells("tr808", [(0, "bass", 1), (4, "snare", 1), (8, "bass", 1), (10, "bass", 1), (12, "snare", 1),
                            (2, "closed_hihat", 1), (6, "closed_hihat", 1), (14, "open_hihat", 1),
                            (0, "cymbal", 1), (0, "accent", 1)])


def hit808(s, ins="clap"):
    """One short hit at the start of bar 1; pattern 1 (empty) from bar 2: for delay tails."""
    s.pdrum_cells("tr808", [(0, ins, 1)])
    s.ev("tr808", B32, "pattern", 1)


for amt, sh in ((20, 0), (100, 0), (127, 0), (100, 64), (100, 127), (0, 0)):
    s = add(Song("dist_a%d_s%d" % (amt, sh), "Distortion insert on the 303 #1 channel (mixer dist on): amount %d, "
                 "shape %d%s." % (amt, sh, "; amount 0 is a corner case" if amt == 0 else
                                  "; shape 0 is the corner case" if sh == 0 else "")))
    src303(s)
    s.mixer["channels"][0]["dist"] = 1
    s.dev["dist"].update(amount=amt, shape=sh)
s = add(Song("dist_drums", "Distortion insert on the 808 channel (dist on), amount 90, shape 40."))
src808(s)
s.mixer["channels"][2]["dist"] = 1
s.dev["dist"].update(amount=90, shape=40)
s = add(Song("dist_disabled", "Distortion device enabled=0 while the channel's dist switch is on (does a disabled "
             "device pass the signal?); amount 100, shape 64."))
src303(s)
s.mixer["channels"][0]["dist"] = 1
s.dev["dist"].update(enabled=0, amount=100, shape=64)

for mode, mn in ((1, "lp"), (0, "bp")):
    for wave in (0, 64, 127):
        s = add(Song("pcf_%s_wave%d" % (mn, wave), "PCF (pattern controlled filter) on the 808 channel, %s mode, "
                     "wave 'pattern' %d, frequency 64, resonance 64, amount 100, decay 64." %
                     ("low-pass" if mode else "band-pass", wave) +
                     (" The original exports silence: wave 127 has pattern length 0, so the PCF divides by zero "
                      "(0x10016811) and the sound callback leaves the output unwritten; see docs/original_bugs.md OB-6."
                      if wave == 127 else "")))
        src808(s)
        s.mixer["pcf_id"] = 3
        s.dev["pcf"].update(mode=mode, wave=wave, amount=100)
    for fr, rs in ((10, 100), (120, 20)):
        s = add(Song("pcf_%s_freq%d_res%d" % (mn, fr, rs), "PCF on the 303 #1 channel, %s mode, frequency %d, "
                     "resonance %d, wave 64, amount 127." % ("low-pass" if mode else "band-pass", fr, rs)))
        src303(s)
        s.mixer["pcf_id"] = 1
        s.dev["pcf"].update(mode=mode, frequency=fr, resonance=rs, amount=127)
for amt, dec in ((0, 64), (127, 0), (127, 127)):
    s = add(Song("pcf_lp_amount%d_decay%d" % (amt, dec), "PCF low-pass on the 808 channel, amount %d, decay %d." %
                 (amt, dec)))
    src808(s)
    s.mixer["pcf_id"] = 3
    s.dev["pcf"].update(mode=1, amount=amt, decay=dec)
s = add(Song("pcf_on_909", "PCF band-pass on the 909 channel (pcf id 4), amount 100."))
s.pdrum_cells("tr909", [(h, i, 1) for h in HITS for i in ("bass", "closed_hihat")] + [(4, "snare", 1),
                                                                                        (12, "snare", 1)])
s.mixer["pcf_id"] = 4
s.dev["pcf"].update(mode=0, amount=100)

for amt, th in ((100, 10), (127, 0), (30, 64)):
    s = add(Song("comp_ch_a%d_t%d" % (amt, th), "Compressor on the 303 #1 channel (mono, comp id 2), amount %d, "
                 "threshold %d, accent knob 127 with accents on steps 0 and 8." % (amt, th)))
    src303(s, accent=127)
    s.mixer["comp_id"] = 2
    s.dev["comp"].update(amount=amt, threshold=th)
s = add(Song("comp_ch_808", "Compressor on the 808 channel (comp id 4), amount 100, threshold 20."))
src808(s)
s.mixer["comp_id"] = 4
s.dev["comp"].update(amount=100, threshold=20)
for amt, th in ((100, 10), (127, 0)):
    s = add(Song("comp_master_a%d_t%d" % (amt, th), "Compressor on the master (stereo, comp id 1), amount %d, "
                 "threshold %d; 808 pattern plus 303 line." % (amt, th)))
    src808(s)
    src303(s)
    s.mixer["comp_id"] = 1
    s.dev["comp"].update(amount=amt, threshold=th)

for mode, mn in ((1, "straight"), (0, "triplet")):
    for fb in (0, 64, 127):
        s = add(Song("delay_%s_fb%d" % (mn, fb), "Delay send 127 on the 808 channel from a single clap at the "
                     "start; %s steps (mode %d), 3 steps, feedback %d; 4 bars to hear the tail." % (mn, mode, fb),
                     bars=4))
        hit808(s)
        s.mixer["channels"][2]["delay_send"] = 127
        s.dev["delay"].update(mode=mode, steps=3, feedback=fb)
for st_ in (1, 8):
    s = add(Song("delay_straight_steps%d" % st_, "Delay, straight, %d step(s), feedback 64, send 100; one clap." %
                 st_, bars=4))
    hit808(s)
    s.mixer["channels"][2]["delay_send"] = 100
    s.dev["delay"].update(mode=1, steps=st_, feedback=64)
for pan in (0, 127):
    s = add(Song("delay_pan%d" % pan, "Delay pan %d, straight 3 steps, feedback 64, send 127; one clap." % pan,
                 bars=4))
    hit808(s)
    s.mixer["channels"][2]["delay_send"] = 127
    s.dev["delay"].update(mode=1, steps=3, feedback=64, pan=pan)
s = add(Song("delay_303", "Delay send 80 on the 303 #1 line, triplet 2 steps, feedback 80."))
src303(s)
s.mixer["channels"][0]["delay_send"] = 80
s.dev["delay"].update(mode=0, steps=2, feedback=80)
s = add(Song("delay_automated", "Delay steps and feedback changed by TRAK events during the loop (steps 3 -> 5 at "
             "bar 2, feedback 20 -> 100 at bar 3); one clap per bar.", bars=4))
s.pdrum_cells("tr808", [(0, "clap", 1)])
s.mixer["channels"][2]["delay_send"] = 127
s.dev["delay"].update(mode=1, steps=3, feedback=20)
s.ev("delay", B32, "steps", 5)
s.ev("delay", 2 * B32, "feedback", 100)

# ---------------------------------------------------------------- mixer
for nm, ch, what in (("303", 0, "303 #1"), ("808", 2, "808"), ("909", 3, "909")):
    s = add(Song("mixer_level_sweep_" + nm, "Level of the %s channel swept from 127 down to 0 in 16 steps over the "
                 "2-bar loop by song automation (mixer track events)." % what))
    if ch == 0:
        src303(s)
    elif ch == 2:
        src808(s)
    else:
        s.pdrum("tr909", ["bass", "snare", "closed_hihat"], (0, 2, 4, 6, 8, 10, 12, 14))
    s.mixer["channels"][ch]["level"] = 127
    for i in range(1, 16):
        s.ev("mixer", i * 4, CHANNELS[ch] + "_level", 127 - i * 8)
for nm, ch, what in (("303", 0, "303 #1"), ("808", 2, "808"), ("909", 3, "909")):
    s = add(Song("mixer_pan_sweep_" + nm, "Pan of the %s channel swept from 0 to 120 in 16 steps over the 2-bar "
                 "loop by song automation." % what))
    if ch == 0:
        src303(s)
    elif ch == 2:
        src808(s)
    else:
        s.pdrum("tr909", ["bass", "snare", "closed_hihat"], (0, 2, 4, 6, 8, 10, 12, 14))
    s.mixer["channels"][ch]["pan"] = 0
    for i in range(1, 16):
        s.ev("mixer", i * 4, CHANNELS[ch] + "_pan", i * 8)
s = add(Song("mixer_delay_send_sweep", "808 channel delay send swept 0 to 120 in 16 steps over 4 bars (feedback "
             "60, straight 3 steps); claps on steps 0 and 8.", bars=4))
s.pdrum_cells("tr808", [(0, "clap", 1), (8, "clap", 1)])
s.dev["delay"].update(mode=1, steps=3, feedback=60)
for i in range(1, 16):
    s.ev("mixer", i * 8, "tr808_delay_send", i * 8)
s = add(Song("mixer_dist_switch", "303 #1 channel dist switch toggled by automation every half bar (on, off, "
             "on, off); distortion amount 100."))
src303(s)
s.dev["dist"].update(amount=100, shape=0)
s.mixer["channels"][0]["dist"] = 1
s.ev("mixer", 16, "tb303_1_dist", 0)
s.ev("mixer", 32, "tb303_1_dist", 1)
s.ev("mixer", 48, "tb303_1_dist", 0)
s = add(Song("mixer_master_low", "Mixer master level 30 (not automatable) with the 303 line."))
src303(s)
s.mixer["master"] = 30
s = add(Song("mixer_master_full", "Mixer master 127, channel levels 127, 303 line plus 808 pattern: clipping probe."))
src303(s)
src808(s)
s.mixer["master"] = 127
for ch in s.mixer["channels"]:
    ch["level"] = 127
s = add(Song("mixer_pcf_switch", "PCF routing changed by automation (pcf id 0, 1 at bar 2, 3 at bar 3, 0 at bar "
             "4) on a 303 line plus 808 pattern.", bars=4))
src303(s)
src808(s)
s.dev["pcf"].update(mode=1, amount=127, frequency=30, resonance=100)
s.ev("mixer", B32, "pcf_id", 1)
s.ev("mixer", 2 * B32, "pcf_id", 3)
s.ev("mixer", 3 * B32, "pcf_id", 0)

# ---------------------------------------------------------------- shuffle
for g in (0, 64, 127):
    s = add(Song("shuffle_303_g%d" % g, "GLOB shuffle %d with the 303 pattern's shuffle flag on." % g))
    s.glob["shuffle"] = g
    plain303(s, shuffle=1)
for g in (64, 127):
    s = add(Song("shuffle_drums_g%d" % g, "GLOB shuffle %d with the shuffle flag on in the 808 (even 16ths) and "
                 "909 (odd 16ths) patterns." % g))
    s.glob["shuffle"] = g
    s.pdrum("tr808", ["closed_hihat", "bass"], tuple(range(0, 16, 2)), shuffle=1)
    s.pdrum("tr909", ["closed_hihat", "snare"], tuple(range(1, 16, 2)), shuffle=1)
s = add(Song("shuffle_flag_off", "GLOB shuffle 127 but the patterns' shuffle flag off: expected identical to "
             "shuffle_303_g0."))
s.glob["shuffle"] = 127
plain303(s, shuffle=0)

# ---------------------------------------------------------------- loop and chase
s = add(Song("loop_bar1", "Loop starts at bar 1 (tick 0), 2 bars; 303 pattern 0 in bar 1, pattern 1 in bar 2 "
             "(pattern change event at bar 2)."))
plain303(s)
plain303(s, idx=1, notes=NOTES3)
s.ev("tb303_1", B32, "pattern", 1)
s = add(Song("loop_bar2", "Same material as loop_bar1, but the loop is bars 2..3 (loop start tick 768): the "
             "chase at the loop start must pick pattern 1 (set at bar 2) and the cutoff 30 set at bar 1.",
             loop_start=1))
plain303(s)
plain303(s, idx=1, notes=NOTES3)
s.t303[0].update(cutoff=100)
s.ev("tb303_1", B32, "pattern", 1)
s.ev("tb303_1", 4, "cutoff", 30)
s = add(Song("loop_chase", "Loop is bars 4..5 (loop start tick 2304); events before the loop set the 303 cutoff, "
             "resonance and waveform, a 303 pan, the 808 pattern and delay send: all must be chased.",
             loop_start=3))
src303(s)
s.pdrum("tr808", ["bass", "snare"], HITS)
s.pdrum("tr808", ["closed_hihat", "bass"], (0, 2, 4, 6, 8, 10, 12, 14), idx=1)
s.t303[0].update(cutoff=90, resonance=0, waveform=0)
s.ev("tb303_1", 8, "cutoff", 25)
s.ev("tb303_1", 40, "resonance", 110)
s.ev("tb303_1", 60, "waveform", 1)
s.ev("tr808", 2 * B32, "pattern", 1)
s.ev("mixer", 70, "tb303_1_pan", 10)
s.ev("mixer", 80, "tr808_delay_send", 100)
s.dev["delay"].update(mode=0, steps=2, feedback=60)
s = add(Song("loop_events_inside", "Loop bars 1..2 with events inside the loop on the 303: cutoff ramp every 4 "
             "steps, waveform flips to square at bar 2."))
src303(s)
for i in range(1, 16):
    s.ev("tb303_1", i * 4, "cutoff", 20 + i * 6)
s.ev("tb303_1", B32, "waveform", 1)

# ---------------------------------------------------------------- tempo
for tempo in (97300, 143210, 66667, 200003):
    s = add(Song("tempo_%s" % ("%g" % (tempo / 1000)).replace(".", "_"), "Tempo %.3f BPM, 2 bars of 303 line and "
                 "808 kick/snare: odd exported length (final partial 2048-frame chunk) and 64-frame block "
                 "quantization." % (tempo / 1000), tempo=tempo))
    src303(s)
    s.pdrum("tr808", ["bass", "snare"], HITS)
s = add(Song("full_mix", "Everything at once for 4 bars at 130 BPM: both 303s, 808, 909, distortion on 303 #1, "
             "PCF on the 909, triplet delay on 303 #1, master compressor, pan spread.", bars=4, tempo=130000))
src303(s)
s.t303[1].update(waveform=1, cutoff=60, resonance=60)
plain303(s, n=1, notes=NOTES2)
s.pdrum("tr808", ["bass", "closed_hihat"], HITS)
s.pdrum("tr909", ["snare", "open_hihat", "clap"], (4, 12))
s.mixer["channels"][0].update(dist=1, delay_send=40, pan=40)
s.mixer["channels"][1]["pan"] = 90
s.mixer["pcf_id"] = 4
s.mixer["comp_id"] = 1
s.dev["dist"].update(amount=70, shape=30)
s.dev["delay"].update(mode=0, steps=2, feedback=50)
s.dev["pcf"].update(mode=1, amount=90)


# ---------------------------------------------------------------- output
README_HEAD = """# Diagnostic songs

Short songs (2 to 4 bars) that each isolate one engine feature, for the reference corpus
(task-014) and the engine's tests.  Generated: do not edit this file or `specs/` by hand.

- `gen_specs.py` writes `specs/NAME.json` and this README from the song descriptions in the script.
- `build.py` builds `songs/NAME.rbs` from every spec with `tools/rbs.py build` (spec merged over the
  Silent Default Song template, written as RBS 4.2).  `build.py NAME...` builds only those.
- Every song is in **song mode** with the loop set (Export Loop as Audio File is greyed in pattern
  mode, `docs/test_machine.md` section 8) and has USRI show-on-open off (no info box).  In song mode
  the TRAK events at the loop start set every automatable value (`docs/song_load.md` section 6), so each
  song carries an event at position 0 for every controller of every track, taken from the same values
  as the device chunks.  Event positions are in 32nd notes (32 per bar).
- Base state of every song: silence (all patterns empty), 120 BPM, master 100, channel levels 100,
  pan centered, no sends, no inserts, GLOB shuffle 0, the 2.0 sound model, loop of 2 bars from bar 1.
  A song changes only what its row says.  Mixer channel numbers: 0 = TB-303 #1, 1 = TB-303 #2,
  2 = TR-808, 3 = TR-909.  The 303 line: 16 steps, notes `0 0 12 0 3 0 7 10 0 0 12 0 5 3 0 7`,
  rests on steps 3, 7, 11, 15, octave-up on steps 6 and 14.
- The WAV a song exports is the loop only (44.1 kHz, 16-bit, stereo), of the length that the formula in
  `docs/initial_investigation.md` section 3 gives.  `silence` is all zeros.

"""


def write_specs():
    os.makedirs(os.path.join(HERE, "specs"), exist_ok=True)
    names = set()
    for s in SONGS:
        assert s.name not in names, s.name
        names.add(s.name)
        with open(os.path.join(HERE, "specs", s.name + ".json"), "w") as f:
            json.dump(s.spec(), f, indent=1)
            f.write("\n")
    with open(os.path.join(HERE, "README.md"), "w") as f:
        f.write(README_HEAD)
        f.write("| Song | Isolates | Loop |\n|---|---|---|\n")
        for s in SONGS:
            g = s.glob
            f.write("| `%s` | %s | %g bars at %g BPM |\n" % (
                s.name, s.what, (g["loop_end"] - g["loop_start"]) / BAR, g["tempo"] / 1000))


if __name__ == "__main__":
    write_specs()
    print("%d songs" % len(SONGS))

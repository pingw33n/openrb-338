# Writing a song: the beat cookbook

How to make a song from scratch and hear it, without a DAW: `tools/rbs.py build` writes an `.rbs`
from a JSON spec, and `orb render` turns it into audio. Everything here was checked against the
repository's own code and songs, not written from memory.

The 1997 machine has three instruments and no sampler:

| Machine | In the song | What it is | Good for |
|---|---|---|---|
| TR-808 | `tr808` | the boomy drum machine | hip-hop, trap: the kick and the cowbell |
| TR-909 | `tr909` | the 80s drum machine | house, techno, boom-bap: a tighter kick and snare |
| TB-303 | `tb303` | one monophonic bass line with a resonant filter | bass lines; acid house by nature |

No piano, no strings, no samples, no long arrangement. It is patterns of up to 16 steps, and a
song that chains patterns.

## The short way

```sh
cd ~/Proyectos/openrb-338

cat > /tmp/beat.json <<'JSON'
{
  "glob": { "tempo": 90000, "song_mode": 1, "loop": 1, "loop_start": 0, "loop_end": 3072 },
  "tr909": { "patterns": { "0": { "length": 16, "steps": {
      "0": [0,1,0,0,0,0,0,0,0,0,0,0],
      "4": [0,0,0,0,0,0,0,0,0,1,0,0],
      "8": [0,1,0,0,0,0,0,0,0,0,0,0],
      "12":[0,0,1,0,0,0,0,0,0,1,0,0]
  } } } },
  "tracks": { "tr909": [ {"pos": 0, "ctrl": "pattern", "value": 0} ] }
}
JSON

python3 tools/rbs.py build /tmp/beat.json /tmp/beat.rbs
python3 tools/rbs.py check /tmp/beat.rbs          # reads back byte for byte
./target/release/orb render /tmp/beat.rbs -o /tmp/beat.wav
xdg-open /tmp/beat.wav
```

`build` merges the spec over the template song (`tests/songs/shipped/Silent Default Song.rbs`):
every field you leave out keeps the template's value, so a spec names only what it changes.

## The three levels of a spec

```
glob        tempo (0.001 BPM units), song_mode, loop, loop_start, loop_end (ticks)
tr808       params (levels and tunings) and patterns
tr909       params and patterns
tb303       a LIST of two: index 0 is the first 303, index 1 the second
mixer       master, and one channel per machine: level, pan, delay_send, dist
delay, pcf, dist, comp    the effect settings
tracks      the events: which pattern plays when, and every knob change
```

`tempo` is in **thousandths of a BPM**: `90000` is 90 BPM, `140000` is 140 BPM.

### The drum patterns

`steps` maps a step index (`"0"`…`"15"`) to a row of 12 numbers, one per column. The column
order is fixed and differs between the two machines (`DRUM_COLUMNS` in `tools/rbs.py`):

```
tr808: accent, bass, snare, low_tom, mid_tom, hi_tom, rim_shot, clap, cow_bell,
       cymbal, open_hihat, closed_hihat
tr909: accent, bass, snare, low_tom, mid_tom, hi_tom, rim_shot, clap,
       closed_hihat, open_hihat, crash, ride
```

> The last four are **swapped** between the two machines. Check the list before you write a row.

A value means, for the 808: `0` off, `1` on. For the 909: `0` off, `1` on, `2` accent, `3` flam.

`length` is `1` to `16` (the pattern's steps; the rest of the 16 are silent).

### Accent is not a sound

The first column of both machines is `accent`, and it is not a drum. A step with accent makes
**every** sound of that step louder, by the same amount, and the machine's `accent_level` knob sets
that difference. It is how a flat pattern gets life: a closed hi-hat on all sixteenths with accents
on the odd steps is the classic example (manual, page 35).

### Open and closed hi-hat exclude each other

Open hi-hat and closed hi-hat cannot sound together (manual, page 35):

- A closed hi-hat cuts off an open one that is still ringing.
- On the **808**, both on the same step give a very short open hi-hat, which sounds like a closed
  one.
- On the **909**, both on the same step give the open hi-hat.

So an open hi-hat followed by a closed one is a two-step gesture, not two independent sounds.

### The 303 patterns

A step is an object, not a bare number (`Step303` in `tools/rbs.py`):

```json
"0": {"note": 1, "rest": 0, "accent": 1, "slide": 0}
```

| Field | Range | Meaning |
|---|---|---|
| `note` | 0 to 12 | tone/pitch, a semitone ladder (`docs/RBS42.txt`: `0x00` to `0x0c`) |
| `rest` | 0 or 1 | **set = rest** (silence), the opposite of what you would guess |
| `accent`, `slide` | 0 or 1 | accent, and a slide into the next note |
| `up`, `down` | 0 or 1 | transpose up / down |

A step left out of the spec keeps the template's, which is `rest: 1` — silence. So you write only
the notes, and the holes stay silent.

**Slide goes on the note you slide *from*, not the one you slide to** (manual, page 45). The step
with `slide` ties into the next one and the pitch glides continuously. A slide from a rest still
glides: the glide starts at the silent step's pitch and lands on the next note's.

`note` is a scale position, not a MIDI note, so what you get depends on the 303's `tune` and the
song's `vintage` setting. Write the pattern, render, and listen: there is no table that turns
`note: 5` into a named pitch without running the engine.

### The 303's knobs

A knob has two homes in the file, and only one of them is heard:

- The **chunk** (`tb303[0].params`) is what the panel shows when the song opens.
- The **track** (`tracks.tb303_1`) is a list of knob events, and **the automation wins**: in song
  mode the engine follows the events, not the chunk.

Setting `cutoff` in the chunk and nothing in the track gives you silence for that knob. This is
the easiest way to lose an afternoon: the file looks right, the value is there, and the bass does
not change when you edit it. Write the knobs as track events:

```json
"tracks": {
  "tb303_1": [
    {"pos": 0, "ctrl": "cutoff",    "value": 52},
    {"pos": 0, "ctrl": "resonance", "value": 35},
    {"pos": 0, "ctrl": "env_mod",   "value": 45},
    {"pos": 0, "ctrl": "decay",     "value": 62},
    {"pos": 0, "ctrl": "accent",    "value": 58},
    {"pos": 0, "ctrl": "tune",      "value": 64},
    {"pos": 0, "ctrl": "waveform",  "value": 0},
    {"pos": 0, "ctrl": "pattern",   "value": 0}
  ]
}
```

Check it: change `cutoff` to 20 and to 90 and render both. If the two WAVs are byte-identical, the
knob is not reaching the engine. (`md5sum` on the two files answers it at once.)

A second trap, which caught this document's own first beat: the knobs went into `tb303_2`'s track
while the notes were in `tb303_1`'s. Both machines are in the song, `tb303_1` plays, and the knob
events sit on the one that does not. Keep every event of an instrument on that instrument's track.

| Parameter | Range | What it does |
|---|---|---|
| `cutoff` | 0 to 127 | the filter: low is dark and round, high is bright and thin |
| `resonance` | 0 to 127 | routes the filter's output back to its input: a hollow, pronounced character. With envelope modulation, slide and accent, it makes the 303 what it is |
| `env_mod` | 0 to 127 | how far the envelope opens the filter. At its lowest it barely acts; higher makes each note brighter and then darker as it decays |
| `decay` | 0 to 127 | the length of that envelope: fast or slow dulling of each note |
| `accent` | 0 to 127 | how different an accented step sounds from the others, in volume and in character (the manual, page 49) |
| `tune` | ±1 octave in semitones | the base pitch, for putting patterns in a key |
| `waveform` | 0 or 1 | `0` is the sawtooth (all harmonics, full), `1` the square (odd harmonics only, hollow) |

For a low hip-hop bass line: low `cutoff` (30 to 45), low `accent`, short `decay`. The 303 is a
monophonic acid synth by design: a high `resonance` with a loud `env_mod` is its signature sound,
and it is not what a plain bass line usually wants.

> The `Envelope Modulation` and `Decay` descriptions are the original manual's (page 48 to 49,
> `docs/propellerhead_rebirth_rb338_v2.01_manual.pdf`), which is the authority on what the knobs
> are meant to do; the code only says what they compute.

## Shuffle (swing)

Shuffle has two parts, and they live in different places:

- **Amount**, one for the whole song: `glob.shuffle`, `0` to `127` (`docs/song_load.md`, GLOB[14]).
  It delays the sixteenth notes that fall between the eighths, which is the swing feel.
- **On or off, per section**: each pattern carries a shuffle flag, so the 808 can swing while the
  909 stays straight (manual, page 21: "activate Shuffle for the desired sections").

The manual's pointer for the amount: **around "two o'clock" you get a classic triplet feel.** Its
knob is drawn as a dial, so the 0 to 127 value at two o'clock is about 85; the template's 64 is a
milder swing. `0` is straight.

> This is an estimate from the dial's geometry, not a measured value: the manual names a clock
> position, not a number. Check it by ear.

## Making it a song, not a one-bar loop

A template is in **pattern mode** (`song_mode: 0`) with a one-bar loop, so it plays one pattern
and stops. To get four bars:

```json
"glob": { "song_mode": 1, "loop": 1, "loop_start": 0, "loop_end": 3072 }
```

`loop_end` is in ticks, and one bar is **768 ticks** (`TICKS_PER_BAR`), four beats of 192. So
`0` to `3072` is four bars; `0` to `768` is one.

Then `tracks` says which pattern plays when, and every knob change, in 32nd notes (32 per bar):

```json
"tracks": {
  "tb303_1": [ {"pos": 0, "ctrl": "pattern", "value": 0} ],
  "tr909":   [ {"pos": 0, "ctrl": "pattern", "value": 0} ],
  "mixer":   [ {"pos": 0, "ctrl": "tr909_level", "value": 110} ]
}
```

The names are `mixer`, `tb303_1`, `tb303_2`, `tr808`, `tr909`, `delay`, `dist`, `pcf`, `comp`.

## Checking it is not silence

The template song really is silent, so an empty build renders zeroes and looks like success.
Check the level:

```sh
python3 - /tmp/beat.wav <<'PY'
import wave, audioop, sys
w = wave.open(sys.argv[1])
d = w.readframes(w.getnframes())
print("frames", w.getnframes(), "seconds", round(w.getnframes() / 44100, 2),
      "RMS", audioop.rms(d, 2), "peak", audioop.max(d, 2))
PY
```

`RMS 0` and `peak 0` mean silence: the spec named no steps, or the pattern is not playing (a
machine's `enabled` is 0, or `tracks` names no pattern for it). Any non-zero RMS is sound.

## Rendering

| Want | Command |
|---|---|
| the whole loop | `orb render SONG.rbs -o out.wav` |
| bars 0 to 4 | `orb render SONG.rbs -o out.wav --range 0:4` |
| FLAC or Ogg, for smaller files | `-o out.flac` or `-o out.ogg` |
| listen from the shell | `orb play SONG.rbs --seconds 11` |

`--range` counts bars from 0 and END is exclusive; `orb` refuses an END past bar 1000 and a START
not below END. `.wav` is 16-bit stereo 44100 Hz, the bytes the engine exports.

> A long range writes a big file: bar 1000 is over 400 MB of WAV. Use `.flac` or `.ogg` for
> anything long.

`orb play` opens the sound card and plays with **no window and no player**: it is audio only, and
it ends at the song's end or at `--seconds`. Nothing on screen says it is running; `pgrep -af 'orb
play'` finds it. Do not leave one playing by accident: it is heard, and nothing shows it.

## From an MCP client

With the `orb` server loaded (`docs/mcp.md`), the same steps are tools:

```
orb_render  {song, output, range?, cwd}
orb_info    {song, cwd}
orb_compare {song, reference, cwd}
```

`cwd` is the repository root, because the server is a separate process and its own working
directory is not the client's. No tool writes a song: `tools/rbs.py build` is a Python tool, run
from the shell.

## A worked example

`~/Musica/rebirth/hiphop-90bpm.rbs` is a beat of four bars at 90 BPM: a 909 with a kick on 1 and
3, a snare on 2 and 4, eighth-note hats, an open hat and a syncopated kick; and a 303 line with a
low cutoff (38) and a high resonance (82) for a dark bass. It renders to 10.7 s. Its level is RMS
2653, against RMS 0 for the silent template it was built from.

## More songs to learn from

`examples/` holds one song written for this repository (`examples/empuje.rbs`, generated by
`examples/empuje.py`): 64 bars of dance built with the techniques in this document. It is not part
of the reference corpus and nothing compares it; it is there to be read, changed and played.

`tests/songs/` holds the corpus: `diag/` has one song per engine feature, and `shipped/` the songs
that came with the original. Read any of them with:

```sh
python3 tools/rbs.py dump SONG.rbs     # human readable
python3 tools/rbs.py json SONG.rbs     # the model, the same shape `build` takes
```

Copying a `diag` song and changing one field is the fastest way to learn what each knob does.

## Tricks from the people who used it

Two decades of community knowledge, and none of it is in the code. The sources: Peff's Keyboard
Magazine article of July 2000 ("ReBirth of the Cool", Kurt Kurasaki, six pages, with example songs
in `examples/`) and the 320 mods of the Computer Controlled Underground archive
(`github.com/peffre/ReBirth-RB-338`). Neither is in git; both are read locally.

### 32-step patterns out of 16-step ones

The 909 and the 808 cannot hold a 32-step pattern. Use two bars of a song loop:

1. Loop locators over two bars, bar set to the loop start.
2. Record, select pattern A1, advance one bar, select A2, stop.
3. Program the loop as if it were pattern mode.

Knob settings reset each cycle, so they must be recorded into the loop. For 32nd-note rolls, double
the tempo and double the delay steps with it: at 133 BPM, 266 BPM and 6 delay steps give each step
the length of a 32nd note. The article notes the app's ceiling of 500 BPM, which caps a 32-step
pattern at 250; the file format itself stores the tempo as an integer of thousandths, so the limit
is the program's, not the format's.

### Three beats to the bar

Song mode does not really handle patterns shorter than 16 steps: a 12-step pattern plays and
cycles, but a pattern change replays its first four steps first. Two ways round it: change the
12-step pattern every three bars, or use three patterns (48 steps) for four bars of 3/4. Using 12
steps for a whole bar also means the real tempo is 75% of the displayed one; 160 shown is 120
played.

### Toms in tune

The tunable 808 and 909 toms take musical samples in many mods, so they can play melodies. The
808's tuning knob has 36 frames and the 909's 41, and the article gives the cents of each. On the
808: frame 18 is the sample's own pitch, frame 22 is +100 cents, frame 14 is -100 cents, so every
four or five frames is a semitone. On the 909 those land on frames 20, 24 and 17, and its first and
last positions are not chromatic.

That makes the kit an instrument. Three toms set 18 frames apart (0, 18, 40 — the low one clamped
at the knob's floor) are an octave and a fifth, and a pattern written in the tom columns is then a
phrase:

```json
"steps": {
  "0":  [0,1,0,0,0,0,0,0,0,0,0,0],
  "3":  [0,0,0,0,1,0,0,0,0,0,0,0],
  "6":  [0,1,0,0,0,0,0,0,0,0,0,0],
  "8":  [0,0,0,0,0,1,0,0,0,0,0,0]
}
```

Read the tom columns down the steps and it is a tune: low, mid, low, high. The kick
(`column 1`) and the snare (`column 2`) keep time underneath it. The drums carry the melody and
whatever else is playing carries the pulse — the inverse of the obvious arrangement.

The knob takes 0 to 63, so a frame that would fall below 0 is clamped: transposing a tom down by
five semitones from frame 18 lands on frame 0, and going further does nothing.

### A held note with a rhythm

The 303 plays one note and holds it; the PCF gives it the rhythm. In `mode 1` (low pass) with a
high `resonance`, a large `amount` and a short `decay`, sweeping the PCF's `frequency` chops the
sustained pitch into a pattern. The instrument keeps time without playing more than one note a bar.

That is the other half of the inversion: the drums have the tune and the synth has the rhythm,
where most pieces do the opposite.

### The delay is an instrument, not just an effect

This is the trick that turns one 303 into a chord:

- **Plucked 303**: the COMP with its ratio at maximum and its threshold near nine o'clock gives the
  attack its pop.
- **Polyphonic 303**: a short eight-step arpeggio with slide, mix delay at maximum, delay steps 2,
  feedback near two o'clock. The delay sustains the notes over each other and they become a chord,
  leaving the second 303 free for a bass line.
- **Reverb**: delay steps at the shortest (a sixteenth triplet), pan centred, feedback at 90%. A
  mono reverb whose length is the feedback.
- **Loop sampler**: feedback at maximum loops for ever, and delay steps at 16 holds one bar of
  everything. Feedback to zero stops it.

The article's example songs use exactly these: `chords01.rbs` has delay steps 2 and feedback 77,
which is the polyphonic 303 setting.

### Wah and filter sweeps

Set the PCF to band pass, amount 30%, decay 75%, Q 80%, then record PCF frequency changes in song
mode. Opening and closing it over a 303 line is the wah effect.

### The mixer's level is not a fade

Setting a channel's level to 0 and raising it later does not fade the channel in. The engine reads
the level when the song starts, and an event at position 0 with value 0 leaves the channel silent
for the whole render, whatever comes after it. Two events at the same position are worse: which one
wins is not defined.

The 303's own `enabled` switch behaves the same way: `enabled: 0` at position 0 and `enabled: 1`
later leaves the instrument silent throughout. Measured on one piece: 219 RMS with the switch, 655
without it.

The way to make an instrument enter is to leave both switches alone and shape its **sound**:

- start with the 303's `cutoff` nearly shut (3 to 10) and open it where the part should arrive.
  A closed filter darkens the sound rather than silencing it, so this shapes the entry without
  touching the level;
- change the level or `enabled` once, at the top, and let the pattern change do the work of marking
  a new section.

This was found the hard way: a piece with `level 0` at bar 0 and `level 127` at bar 5 rendered
almost silent from bar 0 to 15, while the same piece with no mixer events at all played fine.

### Measuring a filter with a number

Counting the RMS does not show a filter moving: closing the cutoff darkens a sound without making
it quieter (measured: cutoff 0, 3, 20 and 60 gave 718, 726, 765 and 825 RMS, a difference too small
to call movement). What does show it is the mean difference between neighbouring samples, which
tracks high-frequency content: the same four settings give a spread of more than two to one.

That number is also the trap: a loud hi-hat raises it as much as an open filter does. Compare two
renders that differ **only** in the filter, or the measurement says nothing.

### Micro-rhythm: variation too small to hear as a change

A loop stops sounding mechanical when it varies by amounts below the level of a new idea. The
engine gives two handles for that:

- **Shuffle is a micro-delay, not a swing.** `shuffled()` (0x1001F3D0) moves only the second 16th
  of each 8th, by `shuffle / 6` ticks, up to 21 of them at value 127. At 104 BPM a tick is under
  two milliseconds, so shuffle 96 is a displacement of about 16 ticks, not a triplet feel. Small
  values do almost nothing; the useful range is high.
- **Variation by one step.** Write one figure, then each pattern that follows is a copy with a
  single hit moved by one step (a tom from 9 to 10, a rim from 13 to 14). The figure stays
  recognisable and its position never settles. Rotating four or five of those variants on one clock
  while another instrument rotates on a different clock means the two never line up twice.

What this does not give: the *amount* is global (GLOB[14], one value for the song), so the
micro-delay cannot drift bar by bar. What each pattern does carry is your own `shuffle` switch
(`song.rs`: 0 means the step function uses the unshuffled tick), so one drum machine can swing while
another plays straight — which is the other half of a microrhythm, since two grids that disagree by
a few ticks sound alive where one grid sounds programmed.

Measured on a piece built this way, comparing bars by the mean difference of their samples:

```
same pattern, adjacent bars   1409
different patterns, 8 bars apart   387
```

The bars of one pattern differed more than the bars of two different patterns — the delay's tail
carried across the bar line and moved the waveform more than the pattern change did. Organic, but
for a different reason than intended: it is the echo that varies, not the playing.

### The mode is in the corpus, not in a theory book

The 303's `note` is a scale position, so "minor" means choosing positions, not picking a key. The
songs in `tests/songs/shipped` answer it directly. Counting every sounding note of the bass across
three of them:

```
G-Spot       0 (26%)  7 (20%)  4 (11%)  9 (8%)   3 (8%)
Calm         0 (28%)  7 (18%)  4 (10%)  3 (9%)  10 (7%)
Transfusion  0 (26%)  5 (17%)  7 (11%)  4 (10%)   9 (8%)
```

Four positions carry most of the music in all three: **0, 7, 4 and 3**. Read as intervals from the
root, that is the root, the fifth, the third and the minor third. Root and fifth alone are
ambiguous; adding the third makes it a chord, and having both `4` and `3` keeps the mode open — the
music is dark without being mournful, which is exactly the balance a dance track wants.

A piece written on those four positions, mostly 0 and 7 with 4 and 3 as colour, sits in the same
harmonic world as the shipped songs. A piece written on 1, 3, 5, 6 does not, whatever its rhythm
does.

### Slap bass, from a mirror in the corpus

A bright, short bass with a hard attack does not come from turning the bass up. The settings that
do it are in one of the shipped songs ("G-Spot", by Dreadzone, programmed by Tim Bran):

| Knob | Value | Why |
|---|---|---|
| `cutoff` | 84 to 88 | the filter open, so the attack has brightness to cut through |
| `resonance` | 84 to 90 | the peak at the cutoff adds the "pop" |
| `accent` | 118 to 127 | the whole point: the accented step hits much harder than the rest |
| `decay` | 62 to 100 | short, so the note stops instead of swelling |
| `dist.amount` / `dist.shape` | 58 to 70 / 60 to 127 | the grit that makes it read as slap |

The manual's compressor recipe helps too: ratio at maximum, threshold low (the article calls this
the "plucked" attack). What it does *not* need is level. In the piece this was tried on, the bass
at level 104 with those settings took the low-frequency energy of the last section from 318 to 983
without the master clipping.

### Carry it, do not fade it

The bass in that music does not leave when the drums do: it takes over and keeps going. Since a
level change is not heard here, "keeps going" has to be **more notes**, not more volume:

```
bars 0-39    a single note a bar, or two
bars 40-47   four notes, accents on the one
bars 48-55   eight notes
bars 56-63   twelve notes, the figure filling the bar, to the last beat
```

The same pattern change that makes an arrangement audible also makes an ending arrive: the piece
gets denser instead of quieter, and nothing has to fade.

### The level that works, and the one that does not

Eleven pieces, each measured, each judged by ear. The two sets do not overlap:

| RMS | Pieces | Verdict |
|---|---|---|
| 2450 to 3620 | sin palabras, envolvente | the bass smothers everything; "a wall" |
| **1390 to 1600** | al reves, espacio, micro | "works", "leave space", "attractive" |
| **785 to 975** | empuje, perfil | "a good base for a DJ" |

Above about 2400 the mix collapses: every element is loud enough that none can be heard
separately. Between 800 and 1600, the parts stay legible. That is a far narrower window than the
headroom suggests, and the reason is the same one three failed pieces share: a loud, continuous
element sets the ceiling for everything else.

The rule to write down: **master at 112 and keep the parts under 130, not 127 with a master at
100.** Leave the volume for the end, and mix for separation instead.

### When the beat leaves, something has to be there

A drop-out only works if what remains can carry the piece. Measured on one arrangement: with the
beat gone at bar 52 and a micro layer at level 40, the low-frequency energy fell from 584 to 159 —
a third of the level — and the section read as a hole rather than a rest. Raising the layer to 82
brought it to 259, which is a drop to about three quarters and reads as a change of weight instead
of a failure.

The instrument that stays is the one that decides whether a drop-out works. It has to be quiet
enough not to bury the beat, and present enough to hold the room when the beat goes.

### Fifteen seconds of pulse is an apology

The previous pieces opened with a long stretch where only a pulse played, on the theory that
something has to establish the grid before anything else can be heard. It is not true: the shipped
songs all start **at bar 0 with the beat already running**, and use the chunk's `enabled` to leave
one machine out rather than starting from silence. A listener gives a track a few seconds, not
fifteen, and a pulse with nothing over it spends them on nothing.

What works instead: the first four bars already carry the beat, and the interest comes from parts
entering and leaving **above** it. Four-bar blocks, so no shape lasts more than eight seconds.

### Dynamics that are heard, and dynamics that are not

A level change in a track does not move the render (see "the mixer's level is not a fade" above), so
an arrangement cannot be built out of fades. The variation that *is* heard comes from the patterns:
a one-hit bar against an eight-hit bar is a real drop, because the instruments stop playing rather
than turning down.

But a constant layer buries all of it. Measured on one attempt: the micro layer at level 72 against
the beat at 96 produced the same RMS (668) in every four-bar block, including the block where the
beat played a single kick; with the layer at 40 the blocks still read flat, and the only step in the
whole curve was where the 303 changed pattern. **The instrument that never rests decides the level
of the piece**, whatever the others do.

So: if a layer is meant to hold the texture, it has to be quiet enough that the parts above it can
be heard stopping. Otherwise its constancy becomes the piece's shape, and the arrangement inside it
disappears.

### Space, not volume

A quiet, dark bass with the rest of the mix below it does not sound "atmospheric": it sounds like
a wall with nothing behind it. The ear separates sounds by level *and* by rhythm, and a bass that
plays continuously removes the second cue for everything else. Measured on one attempt: the bass at
level 118 against the drums at 58 left no room, and the piece read as one loud object.

Two rules that came out of it:

- **Fewer notes, wider gaps.** The shipped song "Calm" puts 9 notes of 16 in its 303 pattern; the
  dense ones use all 16. An ambient piece wants the low end of that, and silence between hits.
- **Do not outline a chord in order.** A figure that walks root, root, third, root, fifth is the
  shape of several well-known tunes, and a listener will hear the quote whether it was meant or
  not. Wide jumps that do not resolve, or a single repeated pitch, avoid it.

The piece that worked best so far does the opposite of the obvious in three ways: the toms carry
the melody (tuned apart, one hit at a time), the 303 plays single notes far apart, and the delay is
the only thing filling the gaps. Its ratio of real silence is about one sample in eight.

### Check that a knob moved

A change that reaches the engine changes the audio. Render twice and compare with `md5sum`: equal
files mean the knob never reached the engine. That test is what exposed the track-versus-chunk trap
above, and it is the fastest way to tell a real parameter from a decorative one.

### One case this check has not settled

The compressor is the one parameter this document could not move. Setting its amount and
threshold, its `enabled`, and the mixer's `comp_id` from 0 to 5 all leave the render byte-identical
(`md5sum` equal), on this beat and on the article's own `Plucked 303-01.rbs`. The engine does
implement it (`crates/orb-engine/src/comp.rs`, `CCompInsert`), and `comp_id` selects where it sits
(1 the master, 2..5 a channel, `crates/orb-engine/src/song.rs`); so either the routing needs
something else that these specs do not set, or the effect is subtle enough not to reach these
programme materials.

Not a claim that it is broken: a note that the "the md5 tells you" check says no here, and nobody has
yet found the input that makes it say yes.



## Where this comes from

Three sources, and each knows something the others do not:

- **The code and the songs** (`tools/rbs.py`, `crates/`, `tests/songs/`) say what a value computes
  and what a field is called.
- **The original manual**, `docs/propellerhead_rebirth_rb338_v2.01_manual.pdf` (232 pages, Ernst
  Nathorst-Böös, Propellerhead 1998), says what a knob is for. The code cannot tell you that a
  closed hi-hat cuts off an open one, or that slide goes on the step you slide from.
- **The people who used it**, the Keyboard Magazine article of July 2000 and the 320 mods of the
  Computer Controlled Underground archive, say how it was actually played: 32-step patterns out of
  two bars, a delay that makes chords, toms tuned to play melodies. That knowledge exists nowhere
  in the program.

All three are copyrighted and stay out of git. What is written here is in our own words, with the
page or the file it comes from, so a reader can check it.

Where they disagree, the manual and the article win on intent and the code wins on behaviour: this
project matches what the original *did*, which the manual describes, the community explored, and
the disassembly proves.

To hear the article's example songs and the archive's mods, keep them outside the repository:

```sh
./target/release/orb render "chords01.rbs" -o /tmp/chords.wav      # the polyphonic 303
python3 tools/rbs.py dump "Plucked 303-01.rbs" | head -20          # its knobs, to learn from
```


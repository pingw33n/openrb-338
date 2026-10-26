# ReBirth RB-338 2.01: initial investigation for a byte-matching Rust audio engine

Goal: a Rust engine that renders `.rbs` (+ optional `.rbm`) to a WAV that matches ReBirth 2.01's
"Export Loop as Audio File" output sample for sample, allowing small floating-point tolerances.
Reference WAVs are produced by the original app on a Windows test machine. The UI comes later and
only needs a similar look and feel. MIDI, ReWire and the realtime audio drivers are out of scope.

Sources:
- `/Volumes/downloads/rebirth_2.01_manual.pdf`, the user manual. It describes behaviour but has no DSP numbers.
- `docs/RBS42.txt`, the official RBS 4.2 file format.
- An IDA survey of `original/Rebirth.exe` (GUI database) and `original/ReBirth.dll`. The DLL has a
  headless database at `ida/ReBirth.dll.i64`.

Every address below is a virtual address: `0x10xxxxxx` is in ReBirth.dll (image base 0x10000000) and
`0x4xxxxx` is in Rebirth.exe. **[C]** means confirmed in the disassembly. **[G]** means guessed or
inferred and still to be verified. Function and field names are descriptive; the databases have not
been renamed.

---

## 1. Verdict

A byte-exact port of the DSP is feasible:
- The engine is small: about 187 KB of code in the DLL.
- It is entirely float32/x87 and block based, with no hidden randomness.
- It has no oversampling, no dithering and no hardware dependence.
- Every algorithm is plain recurrences, lookup tables and linear interpolation.

The hard part is not the DSP. It is the state of the original app when an export starts:
- The export does not reset the engine.
- Oscillator phases, filter states, the delay line, the mixer's 16-block update phase, the leftover
  sub-block buffer and several global counters all carry over from whatever the app did before the
  export.

So the reference-capture protocol (§9) matters as much as the code. Until that protocol is settled,
expect byte-exactness after a short settling period rather than from sample 0.

---

## 2. Binary layout

| Module | Role |
|---|---|
| `Rebirth.exe` (UI "panel", 3672 functions, imports only KERNEL32; the rest is loaded dynamically) | UI, song (.rbs) load/save, mod (.rbm) open/parse, **export driver and WAV/AIFF writers**, realtime audio (DirectSound/MME). Talks to the engine through a shared-memory message port ("ReBirth Panel->DLL" / "DLL->Panel"). |
| `ReBirth.dll` (engine, 945 functions, **full RTTI**) | All DSP and sequencing. Exports `EngineOpen/Close/Idle`, `SafeSoundCallback`, `SyncSoundCallback`, MIDI sync and the TopHat (ReWire precursor) procs. `.rsrc` holds the 46 built-in drum AIFFs. |

RTTI classes in the DLL: `CMixerDevice`, `CTB303Device`, `CTR808Device`, `CTR909Device`,
`CDistInsert`, `CPCFInsert`, `CCompInsert`, `CDelayEffect`, the voice classes `SIMPLE_VOICE`,
`FILTER_VOICE`, `DUAL_SAMPLE_VOICE` and `PITCHED_VOICE`, `CWaveSample`, and the IFF/AIFF readers and
writers.

Device vtables in the DLL (slot = index):

| Class | vtable | Notable slots |
|---|---|---|
| CTB303Device | 0x10033FD4 | [1] step 0x100253A0, [5] reset 0x10026CC0, [6] note-off 0x10025630, [10] host msg 0x100256E0, [11] setParam 0x10026180, [14] **audio 0x10023F10** (inline: Vintage sound) → 2.0 sound 0x100248C4 (§6.3) |
| CTR808Device | 0x10034154 | [14] **audio 0x100280B0**, step 0x10027690, [11] setParam 0x10029050 |
| CTR909Device | 0x10034294 | [14] **audio 0x1002B6F0**, step 0x1002AAC0, [11] setParam 0x1002C6D0 |
| CDistInsert | 0x10033144 | [15] process 0x10010A50, update 0x10010894 |
| CPCFInsert | 0x10033AD4 | [15] process 0x100166D0 |
| CCompInsert | 0x10032B24 | [14] stereo 0x1000EDE0, [15] mono 0x1000EC70, update 0x1000EB44 |
| CDelayEffect | 0x10032CB4 | [14] process 0x1000F940, update 0x1000F754 |
| CMixerDevice | 0x10033214 | mixer block 0x10013444, 16-block update 0x10012C14 |

---

## 3. Export path (exactly what the reference WAV is)

**Export driver in the EXE: `sub_427988` → `sub_4280D5` (WAV) / `sub_427EA7` (AIFF) [C]:**
1. Sends msg 35 `{1,0}` (stop).
2. Queries tempo (id 4), loop start (id 12) and loop end (id 16).
3. Sends msg 35 `{8,start}` (locate to loop start), then msg 35 `{1,1}` (play).
4. Render loop:
   ```
   pos = (double)loopStart
   loop:
     n = (int64)((loopEnd - pos) / (tempo * (192.0/2646000000.0)))   // tempo = BPM*1000; ticks are 192 PPQ
     if n < 2048: last = true else n = 2048
     SafeSoundCallback(buf, n, 0)                                     // fills n frames of int16 stereo
     write 4*n bytes
     pos += tempo * n * (192.0/2646000000.0)
   until last
   ```
   The file's length is therefore set by this double-precision accumulation, with the final partial
   chunk truncated. Replicate it exactly.
5. Afterwards sends msg 35 `{1,0}`, then msg 33 (full engine reset), then `SafeSoundCallback(buf,0,0)`.

**Output format [C]:**
- WAV: RIFF/WAVE, PCM, 2 channels, 16-bit, 44100 Hz (`sub_4A1E69(2,16,44100)`).
- AIFF uses the same sample data.
- The exported range is exactly the song loop. There is no pre-roll and no tail.

**SafeSoundCallback → `sub_100139E0` [C]:**
1. Drains the message port first (`sub_1001CC40`, time-budgeted).
2. Renders in **64-frame blocks**. A partial block is rendered whole and its remainder is kept in a
   leftover buffer (`this[130]`) for the next call.
3. Per sample, L then R:
   `i16 = trunc_toward_zero(clamp((double)f32 * 32768.0, -32768.0, 32767.0))`.
   There is **no dithering**. The clamp constants are f32 32767.0 and -32768.0.
4. Then mixes in a "preview sample" player (field +1064) with saturation. It is only active while a
   UI preview or startup sound plays.

---

## 4. Engine frame (mixer block, `0x10013444`, once per 64 frames) [C]

```
every block:   if (blk & 15) == 0: mixer_update()    // 0x10012C14; blk starts at 10 (ctor), never reset
               blk++
               sequencer_tick()                       // 0x1001EF20, advances position BEFORE rendering
               masterL[0..64] = masterR[0..64] = 1e-6f  // denormal guard, also on send buses
               for ch in [303-1, 303-2, 808, 909]:
                   dev.process(bus)                   // mono float32[64]
                   for ins in [dist, pcf, comp] if routed to ch: ins.process(bus)   // in place
                   send[i] += bus[i] * sendGain       // sendGain constant per block
                   (gL,gR) smoothed: n = old*0.93 + tgt*0.07; stored f32
                   ramp from old to n: step = (n_unrounded - old_f32) * (1/32), advanced every 2 samples
                   masterL += bus*gL(t); masterR += bus*gR(t)
               delay.process(send, masterL, masterR)
               if comp routed to master: comp.process_stereo(masterL, masterR)
               m = mOld*0.93 + mTgt*0.07 (stored f32; the unrounded value is used); master *= m (constant over the block)
```

**Shared tables, built once in the mixer ctor by `0x100046B0` [C]:**
- `T[128]`: `T[127] = 1`. For i = 126 down to 1, `T[i] = f32(T[i+1] - f32(1/24)*T[i+1])`, i.e. about
  (23/24)^(127-i). `T[0] = 0`. This curve is used for every level, send and amount knob.
- `PAN[a][b]` gives `{L,R}` with `d = b-64` and `q = (d*d)>>5`:
  ```
  L = T[d>0 ? clamp(a-q,0,127) : a]
  R = T[d<0 ? clamp(a-q,0,127) : a]
  ```
  The law is asymmetric: b = 0 gives R = 0, while b = 127 leaves L = T[a-124].

**Mixer update (every 16 blocks) [C]:**
- `lev = on ? level : 0`.
- Pan targets: `tgtL = 1e-6f + PAN[lev][pan].L`, and likewise for R.
- Send target: `T[lev]*T[send] + 1e-6f`. An alternative pan-dependent send formula is selected by
  flag `+0x16D`, the sound-model byte (GLOB[482]): zero (2.0 sound) gives the plain send, non-zero
  (Vintage) the pan-dependent one (`docs/song_load.md` §3, §7).
- Master target: `2*T[master] + 1e-6f`.
- Initial values: channel gains 1e-6; master current and target 1.0.

**Routing [C]:**
- Dist is per channel, with one shared parameter set.
- PCF can sit on one channel only.
- Comp sits on one channel (mono) or on the master (stereo, linked).
- The insert order is Dist → PCF → Comp.
- Delay is the only send effect.

**Parameter-change latency [C]:**
- Mixer, dist, comp and delay targets are recomputed only every 16 blocks (1024 samples), then smoothed.
- PCF, 303 and drum parameters are read per block or per trigger.

### Numerics rules (apply everywhere)

- **FPU state [C].** The EXE CRT sets 53-bit precision control. The DLL never changes the precision.
  All 94 `fldcw` sites are a temporary switch to truncation for `fistp`.
  So x87 arithmetic behaves like IEEE f64. The only difference is the extended exponent range, which
  should not matter here.
- **Rust rule:** compute in f64 and round to f32 exactly where the original stores a float to memory.
  - Many loops keep state in x87 registers for the whole block and store f32 only at block end. The
    agents' notes below record where.
  - Never use `mul_add`/FMA, and keep the operand order as written.
- **Float→int conversions** truncate toward zero (`as i32` / `as i64` semantics on in-range values).
- **Transcendentals** (`fcos`, `fsin`, `f2xm1`+`fscale`) and the CRT `pow` (`0x1002DDF0`) run at
  x87 extended precision. They are used only to build tables at init, and every table is stored as f32.
  Plan:
  1. Generate the tables in Rust with f64 maths.
  2. Dump the real tables from a running ReBirth.exe on the test machine and diff them.
  3. If any f32 entries differ, embed the dumped tables as binary blobs.

  All of these tables live in BSS (zero in the file) and are listed in §8.

---

## 5. Sequencer and timing (`0x1001EF20`) [C]

- **Position.** A double that advances by `tempo_mBPM * 4.6439909297052154e-06` per block
  (= 64·192/(60·44100·1000)).
  - That is 192 ticks per quarter note; a 16th step is 48 ticks and a bar is 768 ticks.
  - `tick = trunc(pos)`. Work happens only when `tick > lastTick`.
  - The position is advanced **before** the block renders.
- **Song events (TRAK).**
  - Each event is applied in the first block whose `trunc(pos) >= eventPos`.
  - The loop-end wrap is handled inside this function. Wrapping chases every track to the loop start
    (§5.1) and relocates the devices.
  - At `pos >= 767231` the song stops (999 bars).
- **Shuffle.** `sp = 96*(t/96) + max(0, t%96 - shuffle/6)`. Only the second 16th of each 8th is
  delayed, by up to 21 ticks. Each device step function receives `(tick, shuffledTick, tempo)` and
  uses the shuffled tick when the pattern's shuffle flag is set.
- **Quantisation.** Every note trigger, event and PCF step is quantised to a 64-frame block boundary.
  - A note that starts within block k renders from sample 0 of block k.
  - Voices sample their parameters at trigger time.
- **Pattern changes and per-device enable** take effect only on absolute bar lines (`(tick/48)&15 == 0`).
- **`patStart` reset.** Reset only at a pattern change, at start and at relocate. A pattern shorter
  than 16 steps is therefore not realigned at every bar; it keeps cycling with its own length.
- **909 flam [C].** On a step change, `flamTick = tick + (tempo*flam)/700000 + 1` (int64). When
  `tick > flamTick`, every instrument with step value 3 or 4 retriggers at gain base×1.5.

### 5.1 Chase, locate, start and stop [C]

- **Chase (`0x100151F0`).** Builds `state[ctrl]` from all events with `pos <= P`, then applies ctrl
  0..127 for each track in order.
- **Locate (`0x1001F984`).** Chases in song mode, sets `pos = P` and `lastTick = P`, and relocates the
  devices.
  - Nothing triggers until `trunc(pos)` reaches P+1, which at 120 BPM is the second block.
  - The first step at the loop start fires one block late relative to sample 0.
- **Start / stop.**
  - Start: devices' `vfunc[3]` (303: patStart = bar of the tick, force retrigger).
  - Stop: note-off on all devices, then a chase.

### 5.2 Host message protocol

Packets have the form `[78, song, msg, payload…]`, multi-byte values big-endian. The DLL dispatcher
is `0x1001CDF0`, which calls the song object's handler `0x10011ED0`; `song` is the slot the engine
returned for the song (top-level msg 52), not a device.

| msg | Meaning |
|---|---|
| 35 `{id,val}` | Set global parameter. Ids: 0 song mode, 1 play/stop, 3 loop on, 4 tempo (BPM×1000), 8 locate, 12 loop start, 16 loop end, 20 shuffle, 21 sound model flag (§6.3). Ids 4, 8, 12, 16 carry a u32 |
| 36 | Bulk-set the global struct (23 bytes) |
| 37 | Track events: `[track, u32, u32]`, then 7 bytes each `[99, u32 tick, ctrl, val]` |
| 33 | Full reset |
| 41 | Device data `[dev, sub, payload]` (parameters and patterns) |
| 55 / 56 | Load a mod sample bank / select the built-in bank |

The EXE's conversion of an `.rbs` into these messages is in `docs/song_load.md` [C].

---

## 6. TB-303 ×2 (`CTB303Device`)

### 6.1 Control logic [C]

**Parameters (setParam 0x10026180, applied immediately).** Ctrl 0..8 = enabled, pattern, tune,
cutoff, reso, envmod, decay, accent, waveform (0 = saw, 1 = square).
- Cutoff, reso, envmod and waveform are read every block.
- Tune, decay and accent are sampled at note-on.

**Internal pattern format.** 98 bytes: `[shuffle, len, 16 × {note 0..12, slide, accent, up, down, rest}]`.
- `rest == 0` means play.
- [C] RBS flags map as: rest = bit4 (0x10 set = rest), slide = bit0, accent = bit1, up = bit2,
  down = bit3, each as 0/1 (`sub_434309`; `docs/song_load.md` §4.6). The guess rest = !bit4 was wrong.

**Step function (0x100253A0).**
- `v = (tick' - patStart) % (48*len)`, `step = v/48`.
- Gate off at `v >= trigger+24` ticks, i.e. half a step.
- A wrap is detected when `lastPos - v > 24`.
- On a bar line, the pending pattern and enable flag are latched.

**Note-on (0x10023CD4).** `^k` below means repeated squaring of a float value in double precision,
with the result stored as f32. k = 64 for the 2.0 sound (flag +21 zero), k = 4 for Vintage
(non-zero). These labels were swapped in the survey; `docs/song_load.md` §3 has the evidence.
```
accEnvCoef = f32(((reso/12700 + 0.99) * 0.999)^k)
if !accent: accAmt = 0;  envDecay = f32((0.99999 - (1 - decay*f32(1/127))^2 * 0.0003)^k)
else:       envDecay = f32(0.99969f^k);  accAmt = f32(accentKnob * 3/127)
if play: gate = 1; ampIn,ampFb = 2.0:(1.0,0) vintage:(0.1,0.9)
         if !prevSlide: E = 1.0; retrig = 1
pitchTarget = ((tuneTab[tune>>2] + note + 12*up - 12*down)*64 + 3072) * 16
              // tuneTab @0x10033F90 = -12,-12,-11..-1,0×6,1..12,12
if !prevSlide: pitchCur = pitchTarget
prevSlide = slide
```

**Note-off (0x10023C74).** Skipped while the current step slides; this is how a slide ties notes.
Sets gate = 0 and `ampIn,ampFb` = 2.0:(0.063,0.937), vintage:(0.1,0.9).

**Glide, per block.** `pitchCur = (11*pitchCur + pitchTarget)/12` (u32 division; it can stall short
of the target). Then `inc = pitchTab[pitchCur>>4]`.

**Pitch table (ctor 0x10023920, 6144 f32).**
```
k = (2^(3/12)*110 / 44100) / 2^(3072/768)
tab[i] = f32(2^(i/768) * k)
```
The phase runs from -1 to 1, so frequency = tab·22050; index 3072 is 65.4 Hz.

### 6.2 DSP (Vintage engine, inline in 0x10023F10; per 64 samples, control values every 4 samples) [C]

The survey called this the 2.0 engine. It runs when the sound-model flag +21 is non-zero, which is
the RBS "Vintage ReBirth sound" (`docs/song_load.md` §3). No shipped song uses it.

```
R  = reso*f32(1/127)
cs = (A*cs1 + (cut*0.0022834645669291337 + 0.46)*g) - b2*cs2; cs2 = cs1; cs1 = f32(cs)   // cutoff smoother
     // A = f32 0x3FF797CF, b2 = 0x3F7068F3, g = 0x3B9CA9F1 (static init 0x10026BB0)
amp = (double)amp + 1e-6f;  E = (double)E + 1e-6f
kA = 0.10166666666666667 - R*0.066666666666666666
em = envmod*0.0029921259842519685 + 0.08
accIn = (1 - accEnvCoef)*accAmt
16×: E' = E*envDecay
     accEnv = f32(accEnvCoef*accEnv + E*accIn)                  // rounded every 4 samples
     amp = amp*ampFb + gate*ampIn
     C = clamp(((E'-0.4)*em + cs) + accEnv*kA, 0, 1)
     c0,c1,c2,c3 = FILT[(reso>>4)*1024 + trunc(C*1023f)]
     G = ((accEnv+1)*amp)*c2;  E = E'
     4×: ph += inc; if ph > 1: ph -= 2
         x = saw ? ph : (ph <= 0.0457763672f ? -0.549316406f : 0.549316406f)
         s = c3*s + x                                           // one-pole pre-filter
         y = (s*G + y1*c0) - y2*c1                              // 2-pole resonant LP
         o = (y - y1) + o*0.985853374f                          // DC blocker; out = f32(o)
         y2 = y1; y1 = y
block end: store f32 of y1, y2, s, o, ph, E, amp
```

**Filter table FILT (0x1012B364; 8 reso × 1024 cutoff × {c0,c1,c2,c3}; built by 0x100234A0) [C]:**
```
K = 1602^(1/1023); base = 0.4/K^1023
for r in 0..8:
  t = r*f32(1/7); aW = t*0.4+0.6; m94 = r/6+0.5; m8C = 1.7799999713897705^r * 0.52
  for i in 0..1024:
    W = K^i*base*aW
    q = clamp((i/8)^2*0.000125 - 0.3, 0, 1)
    D = (1-q)*m94 + q*m8C
    c0 = exp(-πW/D)*2cos(2πW); c1 = exp(-2πW/D); c3 = exp((t*0.85+0.15)*(-2πW))
    D' = D>4 ? 3 : D
    c2 = ((1-c3)*(c1+(1-c0))) / ((D'+1)*0.8)
// π = 3.141592654, 2π = 6.283185308 (decimal literals); exp via f2xm1/fscale; pow = CRT
```

### 6.3 ReBirth 2.0 engine (0x100248C4) [C]

The survey called this the vintage engine. It runs when flag +21 is zero: the RBS "ReBirth 2.0
sound", every file before version 4.2 and every shipped song.

- Control values are computed once per 64 samples with `^64` coefficients.
- Resonance is continuous: rows l and l+1 of FILT are interpolated, with `rf = reso*0.0078732288f`
  and `l = trunc(rf*7)`.
- Coefficients ramp linearly every 2 samples from the stored start values (+0xCF4..+0xD00).
- On retrigger, the start c2 is 0, giving a click-free attack.
- The sample loop is identical to the Vintage engine (§6.2).

**Selection.** The engine is chosen by global flag +21 (song object +0x16D), tested at 0x10023F2D:
zero selects this engine (2.0 sound), non-zero the inline Vintage engine of §6.2.
[C] +21 = GLOB[482] unchanged (0 for files before version 4.2); the exe does not invert it, and the
menu item "Vintage ReBirth Synth Sound" is checked when it is 1 (`docs/song_load.md` §3). The guess
+21 = !GLOB[482] was wrong; it was the engine labels that were swapped.

---

## 7. Drums (808, 909)

### 7.1 Voices (all mono; they add into the device's float bus, in a fixed order) [C]

Samples are raw int16 values (`fild`). The 1/32768 factor is folded into the gain:
`g = f32(gain*2^-15)`. Each voice keeps `hold` (in blocks, multiplier 1.0 while held) and a per-pair
multiplier `m`. The process loops run 32 pairs; the gain is constant within a pair and `g *= m`
after each pair.

| Voice | Algorithm | Ends when |
|---|---|---|
| SIMPLE (0x1001E300) | `out += g*s` | g < 3e-8f or the sample ends |
| FILTER (0x1001E4D0) | One-pole LP `y = c*y + g*s; out += y` | Same as SIMPLE |
| DUAL (0x1001E700) | `out += g1*(A + g2*B)`, with g2 decaying | min(lenA, lenB) |
| PITCHED (0x1001E9A0) | 24.8 fixed-point phase with integer linear interpolation `s = a + ((f*(b-a))>>8)` | phase > len<<8 |

- The voice gains and states (g, y) are stored as f32 only at block end.
- Sample buffers carry 512 zero samples of padding. Voices always render the full 64 samples.
- Each device zeroes its bus, then calls voices in a fixed order. This order affects f32 summation:
  - **808:** BD-A, BD-B, SD, LT, MT, HT, RS/CL, CP/MA, CB, OH, CH, CY.
  - **909:** BD-A, BD-B, SD, LT, MT, HT, RS, CP, CH, OH, Crash, Ride.
  - [G] For export the per-instrument ReWire outputs are null, so everything sums into bus 0.

### 7.2 Triggering and accent [C]

- **808 (step 0x10027690).**
  - `accent = f32(acc*AC*0.03125f + 1)`.
  - Instruments trigger in ascending order. As a result, OH and CH on the same step leaves OH cut.
- **909 (step 0x1002AAC0).**
  - `base = acc*AC*0.03125 + 1`.
  - Step value 1 or 3 gives base×0.8; value 2 or 4 gives base×2.0. Values 3 and 4 also flam (§5).
  - Instruments trigger in **descending** order.
  - [C] RBS 909 values 0..3 reach the step function unchanged (no shift to 1..4): 1 = ×0.8, 2 = ×2.0,
    3 = ×0.8 with flam; 4 never occurs. The AC column holds 0 or 2 (`docs/song_load.md` §4.8).
- **Bass drum.** Two voices alternate, driven by a global flag. On each new hit, the other voice's m
  is set to 0.99 (a fade). All other instruments are monophonic and hard-restart on retrigger.
- **Snare noise order.** Not random. It follows fixed 32-entry sequences, SEQ808 at 0x1003409C and
  SEQ909 at 0x1003421C, indexed by **global counters that are never reset** (pre-incremented).
  The BD alternation flags are global and never reset either. §9 explains why that matters.

### 7.3 Per-instrument formulas

`k = f32(1/127)`. All tables are static f32 data in the file unless noted. The full parameter list
and voice offsets are in the agent notes; the formulas are summarised here.

**808 [C]:**

| Instrument | Formula |
|---|---|
| BD | FILTER, `coef = TONE8[tone>>4]`, `m = DEC16[decay>>3]`, `gain = ((L*k)*L)*0.005f*acc*(1-coef)` |
| SD | DUAL. `A = SD[(tune*5)>>7]` (switched, not crossfaded, despite the manual). `B = N[SEQ808[ctr]]`. `g1 = ((L*k)*L)*0.025f*acc`. `g2 = f32((snappy*k)^2)` |
| Toms / congas | PITCHED, `step = T808[tune]` with `T808[i] = trunc(256*2^((i-64)/192))` (built at init). `gain = (((L*k)*L)*k)*acc`. The selector picks the tom or conga sample |
| RS/CL, CP/MA, CB | SIMPLE, `gain = ((L*k)*L)*0.005f*acc`. Selector 0 picks CL / MA |
| CY | FILTER, like BD but without the `(1-coef)` factor |
| OH | SIMPLE, `L = lvl>>1`, `m = 0.998f`, `hold = 4*decay` blocks |
| CH | SIMPLE, `L = (2*lvl)/3`, then sets hold = 0 on all five SIMPLE voices, so OH starts decaying (the choke) |

**909 [C]:**

| Instrument | Formula |
|---|---|
| BD | FILTER, 4 samples switched by `tune>>5`, `coef = C8[attack>>4]`, `m = DT[decay>>3]`, `gain = (((L*k)*L)*k)*acc9*(1-coef)` |
| SD | DUAL, `A = SD[(tune*5)>>7]`, `B = SDN[SEQ909[ctr]]`, `g2 = f32(snappy*0.01f)`, `m = 0.998` on g2, `hold = tone + 30` blocks |
| Toms | PITCHED, `T909[i] = trunc(256*2^((i-64)/144))`, `m = DT[decay>>3]`, `gain = ((L*(L*6.2e-5f))*kv)*acc9`, with kv = 1.2 / 1.1 / 1.4 |
| Crash / Ride | PITCHED, kv = 0.6, m = 1 |
| RS, CP, CH, OH | SIMPLE, `gain = ((6.2e-5f*L)*L)*c*acc9`, with c = 1.1 (RS), 2.3 (CP), 0.8 (CH), 1.1 (OH) |
| CH decay | `m = f32(DT[d>>3]^8)` |
| OH decay | `m = DT[d>>3]` |
| CH choke | CH sets OH's m to 0.996f |

`DT[j]`: start at 1.0 and subtract 2.5e-5 per index going down, stored f32 (computed at init).

### 7.4 Samples and RBM mods [C, except where noted]

- **Built-in samples.** DLL resources of type `"AIFF"`:
  - 808: IDs 1000..1023 (BD, SD1-5, N1-4, CB, CH, CL, CP, CY, HC, HT, LC, LT, MA, MC, MT, OH, RS).
  - Startup sound: ID 1024.
  - 909: IDs 1025..1045 (bd1-4, sd1-5, sdn1-3, lt, mt, ht, rs, hc, chh, ohh, ccy, rcy).
  - These can be extracted from `.rsrc` once and shipped as data.
- **AIFF parsing.** COMM and SSND chunks, 16-bit only. Big-endian int16 values are used as they are.
  The sample rate is ignored, and only channel 0 of multichannel files is used.
- **RBM container (EXE 0x41813E / 0x414D18).** IFF-like: `RBM `, `INFO` (1280 bytes, includes the mod
  name), `HEAD`, then `EMBF` chunks. Each EMBF chunk holds a NUL-terminated file name followed by the AIFF.
  - Samples are matched by file name, e.g. `TR808BD.aif`; the list is at EXE 0x4BB388.
  - A missing sample falls back to the built-in one, per sample.
- **RBM sample scrambling.** For chunks larger than 1024 bytes, bytes from offset 1024 onward are
  descrambled before AIFF parsing (DLL 0x1001A500 → 0x100163D0):
  ```
  P = (17*key % 1925) + 123
  base = (key&1)*2048
  buf[n] = buf[n] + table[base + (n % P)]   // u8 wrapping add
  ```
  `key` = low byte of the chunk's ordinal.
- **Open: where the 4096-byte key `table` comes from.** The EXE derives it in obfuscated code
  (0x4158A9 / 0x4149EC). It sums the first 1024 bytes of each EMBF chunk, using a stride that depends
  on anti-tamper globals, and reads 2048-byte halves from some file (app field `this[325]`) at offset
  `256 + sum % (0x8000000-2048)`. This is the main blocker for RBM support.
  - Alternative: capture the descrambled samples from a running ReBirth on the test machine and
    work the derivation out from them.

---

## 8. Effects [C]

All effects run their parameter update when `(cnt & 15) == 0`, then increment the counter
(`cnt` starts at 0). PCF reads its parameters live instead.

### 8.1 Distortion

**Update (every 16 blocks):**
- `S = shape*f32(1/127)*3.999` and `A = amount*f32(1/127)*3.999`.
- Bilinear interpolation in a 5×5 grid (0x10151AB8) gives `r`.
- `inGain = f32((T[amount]+0.0625)*2048)` and `outGain = f32(r*5*0.00390625)`.
- Waveshaper table index = `shape>>2`.

**Per block.** Gains are smoothed with `0.9*old + 0.1*tgt` (both start at 0).

**Normal mode.**
```
v = inGain*x + 128
y = linear interpolation of W[idx][0..255] at v, clamped to the ends
out = f32(y*outGain)
```

**Shape == 0 ("1.5 mode").**
```
g = 16*T[amount] + 1
th = 1/(16*T[min(amount,90)] + 1)
out = f32(clamp(g*x, -th, th))
```

**W[32][256]** (BSS, 0x10011210). Uses `fsin`, with phase increment `(t²/961)*(π/8) + π/1024`:
- `W[t][128±k] = ±f32((sin(ph)*256*(1-y) + 256*y*y)*0.7)` with `y = k/128`.
- `W[t][0] = -W[t][255]`.

### 8.2 PCF

**Pattern data** (static in the file):
- Lengths: u16[55] at 0x10033310.
- Resolutions: u16[55] at 0x10033380 (8, 16 or 32).
- Steps: 55×32 bytes at 0x100333F0. 0 = no trigger, otherwise the velocity. Odd values give a slow
  attack (r = 0.995), even values an instant attack.
- Decay table: f32[16] at 0x100332D0.

**Step.** Once per block:
`step = (trunc(seqPos) / (res==32 ? 24 : res==16 ? 48 : 96)) % len`.
This uses the absolute song position, so the pattern stays phase-locked to the song.

**Cutoff base.** A 2-pole smoother towards `freq*4`, with coefficients from `exp`/`cos` at
w = 0.0628 (static init 0x10017840). It starts at 0.

**Envelope, per 4 samples.**
- `E *= dec`
- `env = env*r + (E+base)*(1-r)`
- On a trigger: `E = (v*amount)>>5`, limited so that `E + base <= 511`.

**Filter.**
- `idx = trunc(clamp(env,0,511))`, `res8 = res>>4`.
- `(c0,c1,c2) = Tab[mode][res8][idx]`; tables at 0x1010C270 (LP) and 0x10100270 (BP), built at init
  by 0x100172C4 / 0x10017574.
- `y = (c2*x + c0*y1) - c1*y2`.
- LP outputs `y`; BP outputs `y - y1`.
- State stays in double through the block and is stored as f32 at block end.

**Tables.** Same construction style as the 303 table: `K = 1.0145453214645386`, 512 entries,
8 resonance rows.
- LP: `c2 = 2(1-c0+c1)/((Qg+1)*0.8)`.
- BP: `c2 = 3*cos(πF)*sqrt(1-c0+c1)/((Qg+1)*0.8)`.
- `Qg = Q>4 ? 3 : Q`.

### 8.3 Compressor (mono 0x1000EC70, stereo 0x1000EDE0)

**Update.**
```
af = amount*f32(1/127)
off = 1 - 0.99*af
X = af*(Th/32 + 1)*0.3662109375
floor = Th*f32(1/127)*80000/32768 + 0.01220703125
```
`attack = 0.01f` and `release = 0.9984f`. The hold reload is 300 groups.

**Per 4-sample group.**
- Detector: `s = |x1+x3|`; in stereo, `|L1+L3+R1+R3|` (linked).
- If `s > env`: `env += (s-env)*atk; hold = 300`.
- Else if `hold == 0`: `env = env*rel + 1e-6f`. Otherwise `hold--`.
- `env = max(env, floor)`.
- Gain G ramps linearly over the 4 samples towards `X/env + off`.
- G starts at 0, so the compressor fades in at creation.

### 8.4 Delay (send; global buffer of 131072 f32, mask 0x1FFFF, prefilled with 1e-6)

**Update.**
```
q = mode==0 ? 3*T/100 : 4*T/100     // T = milli-BPM, integer arithmetic
D = min((u32)(26460000*steps)/(u32)q, 0x1FFFF)
fb = f32(fbByte/127)                // 127 = infinite repeats
```
- Pan gains: `f32(1e-6 + PAN[127][pan])`, smoothed with 0.9/0.1 per block.
- [C] Mode 0 = triplet steps (1/3 beat), mode ≠ 0 = 16ths (test at 0x1000F7E5). The exe passes the
  RBS byte unchanged, so RBS42.txt has the polarity backwards (`docs/song_load.md` §4.2).

**Per sample.**
```
d = buf[(w-D) & mask]
L += gL*d; R += gR*d
buf[w] = f32(fb*d + send)
w++
```
- No interpolation, filtering or saturation.
- The write pointer and buffer are **never reset**.

---

## 9. State at export start: the main byte-exactness risk

Nothing below is reset by stop, locate or play. Msg 33 (full reset) is sent only *after* an export
(and after the benchmark) [C]. It is **not** sent on song load: opening a song makes a new song in the
engine (top-level msg 52) and sends its data (`docs/song_load.md` §2) [C]. [G] Whether the new song
gets new device objects, and so fresh 303 and effect state, is not traced.

| State | Effect on the export |
|---|---|
| 303 oscillator phase, filter (y1, y2, s), DC blocker, amp, E, accent env, cutoff smoother | They keep running while stopped. The amp decays to about 1e-6, but the phase alignment changes waveform rounding for the whole song |
| Leftover sub-block buffer in `sub_100139E0` | Up to 63 samples rendered **before** the locate can be the first samples of the export |
| Mixer 16-block counter (starts at 10), effect counters | The phase of gain and parameter updates relative to the export start is arbitrary |
| Mixer and effect smoothed gains | They start from their current values, not from the targets |
| Delay buffer and write pointer (global) | Repeats from earlier playback can leak into the export |
| Compressor env and G, PCF filter and envelope | Carry over |
| Snare noise counters and BD alternation flags (global) | Shift by every snare or bass drum hit since the DLL loaded |

**Proposed reference-capture protocol** (to be confirmed experimentally on the test machine):
1. Start ReBirth fresh with the startup sound and demo song disabled if possible.
2. Open the song. This creates a new song in the engine (msg 52); it does not send msg 33.
3. Export immediately without pressing Play.
4. Repeat each export twice, and once after a restart, to measure how deterministic it is.

Then model "reset, then N idle blocks, then export". N matters only for the 303 phase, which keeps
running in the idle callback while stopped.

**Diagnostic songs worth building:**
- Silence: expect all zeros, since the 1e-6 offsets truncate to 0.
- Single 303 notes (saw and square, at several cutoff and reso settings).
- Each drum instrument alone.
- Each effect alone.
- Loop starts at bar 1 and bar 2.
- Odd tempos, to exercise the final partial chunk and block quantisation.

**If fresh-process determinism fails,** instrument the test machine with a debugger (x64dbg or
similar) to dump the engine state at export start. The engine objects come from `operator new`,
reached through `dword_1011D0B0`.

---

## 10. Work items and open questions

1. **RBS → engine message conversion in the EXE**: traced in `docs/song_load.md` (task-003) [C].
   - Track position scaling: tick = RBS position × 24, not ×8; the RBS unit is a 32nd note (§5 there).
   - 303 step flags: rest = bit4, the others as listed in §6.1. The scrambler in `sub_420FB7` and its
     siblings only fire without the genuine `Rb20ful.dat` pages, which a normal load always has (§9
     there).
   - 909 step values pass unchanged (§7.2).
   - Delay mode 0 = triplets (§8.4); PCF id 1..4 = channel 0..3, comp id 1 master, 2..5 channel 0..3;
     +21 = GLOB[482] unchanged, and the 303 engine labels of §6.2 / §6.3 were swapped.
   - Pattern mode: the device chunks decide. Song mode: the TRAK chase decides every automatable
     value; the chunks give patterns, mixer master and channel enables (§6 there).
2. **Mixer flag `+0x16D`**: it is the sound-model byte, written by msg 36 byte 21 and msg 35 id 21
   (the "host msg 21" case at 0x10012696 is msg 35's id 21). During an export it holds the song's
   GLOB[482], 0 for every shipped song (`docs/song_load.md` §7) [C].
3. **RBM key table** source file and the stride in the obfuscated EXE code (§7.4).
4. **Table generation fidelity.** Dump the BSS tables from a live process and diff them against Rust:
   - 303 FILT (0x1012B364), pitch table (0x10125364)
   - PCF LP/BP tables
   - Dist W
   - T and PAN (0x10054550 / 0x10034550)
   - T808 / T909 / DT
5. **Export start state** (§9): settle by experiment.
6. **Loop-end wrap inside the final pre-rendered block.** The last block can wrap to the loop start
   and chase; this only affects the leftover buffer.

## 11. Suggested Rust architecture

```
rbs        – IFF parser for RBS 4.2 (docs/RBS42.txt) → Song model
rbm        – RBM container parser and descrambler (needs the key table, §7.4)
engine     – block engine (64 frames), f64 math with explicit f32 store points
  tables   – generated or embedded tables (§4, §6, §7, §8)
  seq      – clock, chase, shuffle, per-device step logic
  tb303, tr808, tr909, voices
  fx       – dist, pcf, comp, delay
  mixer
render     – export driver replicating §3 (chunking, int16 conversion), WAV writer
tools      – golden-WAV comparator (exact / ±1 LSB / first-diff report)
```

Keep the engine single-threaded and deterministic. Unit-test each block function against values
captured from the original where possible. Inject the "unreset" state of §9 explicitly, so that the
renderer can reproduce a given capture protocol.

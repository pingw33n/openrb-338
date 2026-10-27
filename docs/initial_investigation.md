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
- Oscillator phases, filter states, the delay line and several global counters carry over from
  whatever the app did before the export. [C] The leftover sub-block buffer is always empty at export
  start, and with the MME driver the mixer's 16-block update phase is fixed (`docs/export_state.md`
  §5, §7.1).

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
   Then it stops the realtime audio channel (its vtable slot 2, 0x427BD9); no realtime callback runs
   during the export (`docs/export_state.md` §4).
3. Sends msg 35 `{8,start}` (locate to loop start), then msg 35 `{1,1}` (play). Both are applied by
   the drain of the first `SafeSoundCallback` of the render loop.
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
   [C] task-021, from the disassembly of 0x428139..0x4281DB: start, end and tempo are 32-bit
   `fild`s; `192.0/2646000000.0` is divided at run time from `dbl_4BBE08` (192.0) and `dbl_4BBE10`
   (2646000000.0); `n = __ftol((end - pos) / (tempo * (192/2646e6)))` (truncation); the step is
   `(tempo * n) * (192/2646e6)` (`fild tempo; fimul n`), stored as a double; the chunk is rendered
   and written before `pos += step`. A final `n` of 0 still makes a 0-frame call. Rust:
   `crates/rb-engine/src/frame/export.rs`, which matches the frame count of all 153 diagnostic and
   shipped references of `tests/refs`; for "Fast Forward" (140 BPM, 250 bars) it is 18899999,
   one less than the exact 18900000 ([Q-1](original_bugs.md)). 152 of the 153 end on a chunk that is not a multiple of 64.
5. Afterwards sends msg 35 `{1,0}`, then msg 33 (full engine reset), then
   `SafeSoundCallback(NULL,0,0)` (0x427DD6), then restarts the realtime channel (slot 1, 0x427DE7).
   What msg 33 resets: `docs/export_state.md` §6.4.

**Output format [C]:**
- WAV: RIFF/WAVE, PCM, 2 channels, 16-bit, 44100 Hz (`sub_4A1E69(2,16,44100)`).
- AIFF uses the same sample data.
- The exported range is exactly the song loop. There is no pre-roll and no tail.

**SafeSoundCallback → `sub_100139E0` [C]:**
1. Drains the message port first (`sub_1001CC40`, time-budgeted: at least one message, more while
   `timeGetTime() <= start + (n*1000/44100)/3` ms; `docs/export_state.md` §2.3).
2. Renders in **64-frame blocks**. A partial block is rendered whole and its remainder is kept in a
   leftover buffer (`this[130]`) for the next call. [C] In the app this buffer is empty at every
   export start (`docs/export_state.md` §5).
3. Per sample, L then R:
   `i16 = trunc_toward_zero(clamp((double)f32 * 32768.0, -32768.0, 32767.0))`.
   There is **no dithering**. The clamp constants are f32 32767.0 and -32768.0.
   [C] task-021 (0x10013C3C..0x10013D24): the scale is `dbl_10151F70` = 32768.0, the bounds
   `flt_10151F78` = 32767.0 and `flt_10151F7C` = -32768.0; the product is tested against the upper
   bound first (`fcom`, C0|C3), so a NaN falls through to the lower test and becomes -32768; the
   conversion is `fistp qword` under a truncating control word. It converts the call's float
   output buffers `unk_100E012C` (L) and `unk_100F012C` (R); `n` is clamped to 0x4000 first
   (0x100139ED). Only output channels 0 and 1 are enabled (`byte_100DF754[0..1]` = 1, 2..17 = 0).
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

[C] task-021, details from the disassembly of 0x10013444 (Rust: `crates/rb-engine/src/frame`):
- The argument is the 18-entry output pointer array of `sub_100139E0`. The master buses are
  entries 0 (L) and 1 (R); each is filled with 1e-6 (`0x358637BD`) only when its pointer is
  non-null (a null one is replaced by the unfilled scratch `song+0x140C`/`+0x150C`).
- Channel c's own output entry is 2, 3, 4 or 11. It is always null in the app, so the device
  renders into the shared scratch `unk_1010012C` (a process global, overwritten by each device in
  turn), and only then is the channel mixed into the master (`cmp eax, ebp; jnz` at 0x100136B1).
  A device gets `float **` (slot 14, 0x10013580), an insert `float *` (slot 15).
- Each block first advances the meter index `song+0xA0`: `i = (i + 23) & 63` (0x10013481). The
  channel peak (`+0xB8` of the channel block) and the master peaks (`song+0`, `+4`, after the
  master gain) take `|bus[i]|` when larger. Display only.
- The channel block is 44 bytes at `song+0xA4 + 44c`: `+0` R target, `+4` L target, `+8` R gain,
  `+0xC` L gain, `+0x10` send gain, `+0x14` peak, `+0x18` the send bus pointer (`song+0x160C`),
  `+0x1C` word 1, `+0x20/+0x24/+0x28` the dist, PCF and comp insert pointers, called in that order.
- Send: `send[i] = f32(bus[i]*sendGain + send[i])`. The mixer never refills the send bus: the
  delay does, with 1e-6, in both its enabled and disabled paths (0x1000F940).
- [C] task-022: the song has three 256-byte send buses, `song+0x160C + 0x100k`, one per send-effect
  slot. `sub_100140B4` registers a send effect: its object at `song+0x1DC + 4k`, its bus pointer
  `song+0x1FC + 4k`, count `song+0x1E8`; the block calls slot 14 for `k < count` (0x1001381F). The
  ctor registers only the delay (its one caller), so buses 1 and 2 (`+0x170C`, `+0x180C`), which
  the ctor and msg 33 fill with 1e-6, have no reader.
- Smoothing (0x100136B7): `nL = oldL*0.93 + tgtL*0.07` and likewise for R, in f64
  (`dbl_10151F48` = 0.93, `dbl_10151F50` = 0.07); `f32(n)` is stored; the ramp step
  `(n - old) * (1/32)` (`dbl_10151F58`) is kept as a double. The mix loop runs in x87 registers
  from the stored old gains: samples 2p and 2p+1 use `old + p` steps, added one at a time, and
  each output sample is `f32(bus*g + master)`. The L gain mixes into entry 0, the R gain into 1.
- Master (0x100138AE): `m = mOld*0.93 + mTgt*0.07`, `f32(m)` stored, every master sample becomes
  `f32(x * m)` with the unrounded m, after the send effect (slot 14 with `(song+0x1FC[i], L, R)`)
  and the master compressor (slot 14 with `(L, R)`).

**Shared tables, built once in the mixer ctor by `0x100046B0` [C]:**
- `T[128]`: `T[127] = 1`. For i = 126 down to 1, `T[i] = f32(T[i+1] - f32(1/24)*T[i+1])`, i.e. about
  (23/24)^(127-i). `T[0] = 0`. This curve is used for every level, send and amount knob.
  [C] task-019: the step f32(1/24) = 0x3D2AAAAB is data at 0x1014F8E4 (section `.data1`, which is
  in the file), read at 0x100046C3.
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
- [C] task-021 (0x10012C14): the L target (`+0xA8`) is `f32(1e-6 + PAN[lev][pan][0])`, the R target
  (`+0xA4`) `f32(1e-6 + PAN[lev][pan][1])`; the send is `f32(T[lev]*T[send] + 1e-6)`, or with
  Vintage `f32((PAN[0] + PAN[1]) * T[send] * 0.5 + 1e-6)` (`dbl_10151F20` = 0.5, `dbl_10151F28` =
  1e-6f widened to double); the master target `f32(2.0*T[master] + 1e-6)` (`flt_10151F40`). The
  routing is rebuilt each update: dist when the channel's dist byte is set, PCF when `pcfId == c+1`,
  comp when `compId == c+2`, the master comp when `compId == 1`. Then slot 16 (the effects' update)
  runs on dist, PCF and comp, in that order (`song+0x1EC` list, 0x100130A4), routed or not. The
  meters: channel byte `T[lev]*peak` (127 above 1.0, else `trunc(x*127)`), then `peak *= 0.92`
  (`flt_10151F38`); master likewise.
- [C] task-021: the update runs only from the block and once at the end of the song ctor
  (0x100119D4; the only other xref, 0x10012C10, is a fall-through). The ctor's update reads the
  mixer device's constructor values (`sub_10019CC0`, `sub_10019D50`): master 100, comp and PCF off,
  every channel enabled, level 100, pan 64, send 0, no dist. The load's msg 41 does not run an
  update, so the first 6 blocks after the ctor smooth towards those targets.

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
  [C] task-019, the outcome: step 1 alone gives every table bit for bit (§10 item 4); nothing is
  embedded. The CRT pow 0x1002DDF0 is a table-driven x87 routine (tables at 0x10156000,
  0x10156410, 0x10156818, exponential in `sub_1002E3E0`); it was not emulated, since an f64 `pow`
  (the `libm` crate) already gives the same stored values.

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
  - At `pos > 767231` the song stops (999 bars; strictly greater, task-022). The DLL also drops
    track events above 767231 while the exe accepts up to 767976 ([OB-3](original_bugs.md)).
- **Shuffle.** `sp = 96*(t/96) + max(0, t%96 - shuffle/6)`. Only the second 16th of each 8th is
  delayed, by up to 21 ticks. Each device step function receives `(tick, shuffledTick, tempo)` and
  uses the shuffled tick when the pattern's shuffle flag is set.
- **Quantisation.** Every note trigger, event and PCF step is quantised to a 64-frame block boundary.
  - A note that starts within block k renders from sample 0 of block k.
  - Voices sample their parameters at trigger time.
- **Pattern changes and per-device enable** take effect only on absolute bar lines (`(tick/48)&15 == 0`).
- **`patStart` reset.** Reset only at a pattern change, at start and at relocate. A pattern shorter
  than 16 steps is therefore not realigned at every bar; it keeps cycling with its own length.
- **909 flam [C].** On a step change, `flamTick = tick + (tempo*flam)/700000 + 1` (32-bit integers). When
  `tick > flamTick`, every instrument with step value 3 or 4 retriggers at gain base×1.5.
  - [C] task-025, from the disassembly of 0x1002AAC0 (Rust: `crates/rb-engine/src/tr909.rs`; the
    three `909_flam_interval*` songs match byte for byte): the arithmetic is 32-bit, not int64
    (`imul eax, edx; cdq; idiv` by 0xAAE60 = 700000 at 0x1002ABBC, then `lea eax, [esi+eax+1]`),
    with the song's BPM*1000 tempo and the **unshuffled** tick. Every new step (or wrap) stores
    flamTick at `+0x1914` and arms the flam (`+0x1910` = 1), whatever the step holds. A later call
    on the same step (no wrap) with the flam armed and `tick > flamTick` (signed) disarms it and
    runs the flam pass once: the accent is recomputed from the step's AC byte and every column,
    from 11 down to 1, whose value is above 2 (`cmp byte, 2; jbe` at 0x1002B324) retriggers at
    `f32(accent*1.5)` (`dbl_10154350`). The flam pass skips the bar-line logic but still needs
    the enable latch. `flam` is the live byte 29 (`+0x18AE`), set by controller 29 (see §7.2).

[C] task-022, from the disassembly of `sub_1001EF20` (Rust: `crates/rb-engine/src/seq.rs`). The
sequencer object is at `song+8` (ctor `sub_1001EC50`): `+0` the position (double), `+8` the last
tick, `+0x10` the tick pattern mode saved, `+0xC` the track count, `+0x14 + 4i` the tracks, `+0x94`
the song. The transport state is the global struct `song+0x158` (`docs/song_load.md` §3).
```
if !play: return                                   // 0x1001EF34
inc = (double)tempo * 4.6439909297052154e-06       // fild; fmul dbl_10153808 (0x3ED37A7281B22126)
pos = pos + inc; t = trunc(pos)                    // fistp qword, low dword
if t > lastTick:                                   // signed
    if songMode:
        if loopOn and loopEnd <= pos and loopEnd > pos - inc:   // fild loopEnd; fcomp; pos - inc in f64
            every track: apply events with tick <= loopEnd - 1
            [record on: merge the recording, sub_1001F520]
            pos = pos + (double)(loopStart - loopEnd); t = trunc(pos)     // 0x1001F135
            every track: chase to loopStart; every device: slot 2 (relocate) with loopStart
        every track: apply events with tick <= t   // 0x1001F2AD
        if pos > 767231.0: song end, return        // dbl_10153810 (0x412769FE00000000)
    every device: slot 1 (tick, shuffled(tick), tempo)   // 0x1001F433
lastTick = t
```
- The tempo of the step is `sub_1001BCC0(transport)`: the MIDI clock's when MIDI sync is active,
  else the current song's (`transport+0x16C`), which is the song itself whenever it renders.
- Pattern mode applies no event, never wraps and never ends; the devices still step. The loop wrap
  and the song end are tested only in blocks whose tick moved.
- An event at the loop end itself never fires in a loop ([OB-4](original_bugs.md)): the wrap applies events up to
  `loopEnd - 1`, and the chase starts the next pass at `loopStart`.
- Every set-param call of a block (tracks 0..8, each in list order) comes before every step call
  (devices in song order: mixer, 303, 303, 808, 909, delay, dist, PCF, comp, `song+0x18C`). Only
  the 303s, the 808 and the 909 implement slots 1..3; the `CDevice` defaults are `retn 0Ch`
  (0x10010380), `retn 4` (0x1001BF40) and `retn` (0x10012C00).
- Track events go to the device's slot 11 (set-param, `+0x2C`) through `sub_10011E60`, which finds
  the device by id; a controller whose byte in the track's hold mask (`track+4`) is set is skipped
  (the mask is cleared by the recording path `sub_1001F520` and set by nothing the load or the
  export sends).
- **Song end** (0x1001F484): record off (`sub_10021194(0)`), the host is told (msg 35 `{2,0}`,
  `{1,0}`), `sub_100218A0` (stop without the bar locate), then locate 0. No step in that block.
- **Shuffle** (0x1001F3D0): `idiv` truncating, `96*(t/96) + max(0, t%96 - shuffle/6)`.

**The track list** (track ctor `sub_1001EDA0`, 0x22C bytes; list ctor `sub_100144B0` at `+0x108`).
A circular singly linked list whose head is the list itself with tick `0x7FFFFFFF`; nodes are
`{next, tick, ctrl, value}`. `+0x228` points at the last applied node, `+0x84` is the chase state
(128 bytes), `+4` the hold mask, `+0x104` the device id, list `+0xD + ctrl` a quantisation in 24-tick
units (1 for every controller from the ctor; [C] task-023: 32 for controllers 0 and 1 of the 303
tracks, set by the 303's slot 7 from the song ctor, §6.4).
- **Insert** (`sub_10014750`, from msg 37's `sub_10021010`): the tick is rounded up to a multiple of
  `24q` (`sub_100146E4`, no change for loaded ticks with q = 1), an event above 767231 after that is dropped,
  and the node is linked before the first node whose tick is not below it, unless a node of the same
  tick has the same controller, whose value is replaced. So the events of one tick are applied in
  **reverse arrival order**, and one controller has at most one event per tick.
- **Stop prune** (`sub_100149F0`, on every track at every stop): an event whose value equals the
  previous value of its controller (0xFF before the first) is unlinked and freed, unless its tick is
  0.

**Transport messages [C] (task-022).**

- **Locate** `sub_1001F984(p)` (msg 35 id 8 via `sub_1001BF70`, msg 32 with the current tick, the
  song ctor with 0): `p = max(p, 0)`; with record on, stopped and `p > trunc(pos)` the events up to
  `p` are applied forward; otherwise in song mode every track is chased to `p` (ctrl 0..127
  ascending, `sub_10012B40`); then `pos = (double)p`, slot 2 on every device, `lastTick = p`.
- **Stop** msg 35 `{1,0}` → `sub_1001E100`: `sub_100218A0` (when playing: play = 0 and slot 4 on
  every device; prune every track; in song mode chase to `trunc(pos)`), the host is told, and unless
  MIDI sync is active (`transport+0x160`, set by the DLL export `SetMIDISyncActive`) a locate to
  `768*(trunc(pos)/768)`, which chases again.
- **Play** msg 35 `{1,1}` → `sub_1001BE74`: a different playing song is stopped and silenced first;
  then (`0x1001F7E4`) when not playing, play = 1 and slot 3 on every device. The position is not
  touched; the song becomes current (`transport+0x16C`).
- **Song mode** msg 35 `{0,m}` → `sub_1001FBE0`: on a change the byte is stored and slot 9 runs on
  every device (`sub_10013FB0`); song mode then locates to the saved tick (`seq+0x10`), pattern mode
  relocates every device to 0 and saves `trunc(pos)`.
- Ids 3, 4, 12, 16, 20 store loop on, tempo, loop start, loop end and shuffle (`sub_10021110`,
  `sub_1001FCD0`, `sub_10021130`, `sub_10021150`, `sub_10021170`); with `transport+0x15C == 46`
  (a ctor argument, [G] the ReWire mode) the transport keeps them instead. Id 2 is record
  (`sub_10021194`: turning it off while on merges the recording with `sub_1001F520` and chases).
- **Msg 36** (0x100126B2) applies the 23 bytes in this order: song mode, play or stop, record, loop
  on, tempo, locate, loop start, loop end, shuffle, then stores bytes 21 and 22. The global struct's
  ctor (`sub_10018AC0`): tempo 120000, loop end 768, shuffle 64, byte 22 = 1, the rest 0; the song
  ctor then sets the tempo to 140000.
- **The mixer device's two sets** (`CMixerDevice`): the live parameters at `+0x30` and the
  pattern-mode copy at `+0x58`. Set-param 0x10015C00: ctrl 1 → comp id `+0x34`, 2 → PCF id `+0x38`,
  else `c = (ctrl-5)/6`, `k = (ctrl-5)%6`, k 1..4 → level, pan, send, dist of channel c (`+0x41 +
  6c` ..); k 0 (enabled) is never set. Slot 9 (0x10016280): entering song mode copies live to the
  pattern copy; leaving it copies the live master and enables into the pattern copy, then the pattern
  copy to live. Msg 143 (0x1001588F) unpacks into the pattern copy, then song mode copies only the
  master and enables live, pattern mode everything.

### 5.1 Chase, locate, start and stop [C]

- **Chase (`0x100151F0`).** Builds `state[ctrl]` from all events with `pos <= P`, then applies ctrl
  0..127 for each track in order.
- **Locate (`0x1001F984`).** Chases in song mode, sets `pos = P` and `lastTick = P`, and relocates the
  devices.
  - Nothing triggers until `trunc(pos)` reaches P+1, which at 120 BPM is the second block.
  - The first step at the loop start fires one block late relative to sample 0.
- **Start / stop.**
  - Start: devices' `vfunc[3]` (303: patStart = bar of the tick, force retrigger).
  - Stop: note-off on all devices (only when playing), the track prune, a chase, then the locate
    to the bar start (task-022, "Transport messages" above).

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
[C] task-019, the exact order (0x10023A89..0x10023B7E): `s = pow(2, 1/12)`;
`v = (((1*s)*(s*s))*55)*2` (the cube by the compiler's squaring loop; 55.0 at 0x10153A98, 2.0 at
0x10153A80); `p = pow(2, 1/768)` (f64 `0x3F55555555555555`); `k = (v*(1/44100)) / pow(p, 3072)`
(1/44100 at 0x10153AB0); `tab[i] = f32(pow(p, i)*k)`, all with the CRT pow 0x1002DDF0. The
tune table at 0x10033F90 is i16[32].

### 6.2 DSP (Vintage engine, inline in 0x10023F10; per 64 samples, control values every 4 samples) [C]

The survey called this the 2.0 engine. It runs when the sound-model flag +21 is non-zero, which is
the RBS "Vintage ReBirth sound" (`docs/song_load.md` §3). No shipped song uses it.

```
R  = reso*f32(1/127)
cs = (A*cs1 + (cut*0.0022834645669291337 + 0.46)*g) - b2*cs2; cs2 = cs1; cs1 = f32(cs)   // cutoff smoother
     // A = f32 0x3FF797CF, b2 = 0x3F7068F3, g = 0x3B9CA9F1 (static init 0x10026BB0)
     // [C] task-019: stored at 0x1012470C {b2, A, g}; w = f64 0x3FB015BF8C180485 (f32 0.0628 widened):
     // A = f32(exp(-w/2) * (cos(w)*2)), b2 = f32(exp(-w)), g = f32((1 - A) + b2), A unrounded in g
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
[C] task-019, the exact operand order (0x1002351C..0x100237BC; every intermediate a qword):
`q = (((((i*128)*(1/1024))*i)*128)*(1/1024))*0.000125 - 0.3`, clamped by `> 1 → 1` then `< 0 → 0`;
`D = q*m8C + (1-q)*m94` with `m94 = r*(1/6) + 0.5`; `1/D` is computed once and multiplied
(`(W*-π)*(1/D)`, `(W*-2π)*(1/D)`); `c0 = exp(...)*(cos(W*2π)*2)`; `D' + 1` adds f32 1.0. The
generated table is bit-identical to the dump (crates/rb-engine/src/tables).

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

[C] task-023, the block in operand order (0x100248C4..0x1002539A, traced instruction by instruction):
```
pitch = (11*pitch + target)/12 (u32); inc = PITCH[pitch>>4]
cs = (A*cs1 + (cut*0.0022834645669291337 + 0.46)*g) - b2*cs2; cs2 = cs1; cs1 = f32(cs)
amp0 = amp + 1e-6f; amp = f32(amp0); amp1 = amp0*ampFb + gate*ampIn      // once per block
E0 = 1e-6f + E; E = f32(E0)
acc = accCoef*accEnv + E0*((1 - accCoef)*accAmt); accEnv = f32(acc)
G = amp1*(acc + 1); em = envmod*0.0029921259842519685 + 0.08
kA = 0.10166666666666667 - (reso*f64(1/127))*(1/15)     // dbl_10153C08, not the f32 1/127 of §6.2
rf = reso*0.0078732288f
if retrig (+0xCA0):                                       // set by note-on without slide
    C = clamp((kA*f32(acc)) + (((E0 - 0.4)*em) + cs)); k = ROW(f32(C), rf)
    start = {k.c0, k.c1, 0, k.c3}; retrig = 0             // +0xCF4, +0xCF8, +0xCFC, +0xD00
E1 = E0*decay
C = clamp((kA*f32(acc)) + (((E1 - 0.4)*em) + cs)); t = ROW(f32(C), rf)
d0 = (t.c0 - start.c0)/32; d1 = ...; d2 = (t.c2*G - start.c2G)/32; d3 = ...
32×: 2 samples of §6.2's sample loop with {c0, c1, c2G, c3}; then each += its d
block end: the four coefficients, ph, s, y1, y2, o as f32; E = f32(E1); amp = f32(amp1)
```
`clamp` is `> 1 → 1` then `< 0 → 0`. `ROW` is `sub_10023800`: `idx = trunc(C*1023f)`,
`r = rf*7f` (kept in a register), `l = trunc(r)` (the low word), `frac = r - l`, and each of the four
coefficients `f32(lo + (hi - lo)*frac)` from FILT rows `l` and `l + 1` (reso 127 gives
`l = 6`, so `l + 1` stays in the table). Within a block only the output samples are stored as
f32; every state runs in f64 until the block end.

### 6.4 Device state, sequencing and slots [C] (task-023)

Implemented in `crates/rb-engine/src/tb303.rs`; every TB-303 song of the diagnostic corpus with the
delay disabled renders byte-identical to rbhost in both sound models (§8.4 for why the delay matters).

**Fields** (`CTB303Device`, 0xD04 bytes): `+4` patStart (CDevice, 0 from `sub_10010304`), `+0x30`
the pattern-mode parameter copy (12 bytes), `+0x3C` 32 patterns of 98 bytes, `+0xC7C` the live
parameters (12 bytes; byte 9, `+0xC85`, is the step function's last step), `+0xC88..+0xC93` the
panel's "changed" flags (set by setParam, read only by the panel update 0x10026700), `+0xC94` the
playing pattern, `+0xC95` the enable latched at the bar line, `+0xC96` the wrap flag, `+0xC98` the
pending gate-off position (-1 none), `+0xC9C` the last pattern position, `+0xCA0` the retrigger flag
(read and cleared only by the 2.0 engine), `+0xCA1` the slide flag of the last step, `+0xCA4` the
phase, `+0xCA8` the envelope E, `+0xCAC` its decay, `+0xCB0`/`+0xCB4` the target and gliding pitch,
`+0xCB8` the accent amount, `+0xCBC` the accent envelope, `+0xCC0`/`+0xCC4` the cutoff smoother,
`+0xCC8`/`+0xCCC` the amp's input and feedback, `+0xCD0` the gate (f32 0 or 1), `+0xCD4` the amp,
`+0xCDC`/`+0xCE0` y1/y2, `+0xCE4` the pre-filter s, `+0xCE8` the DC blocker o, `+0xCEC` the accent
feedback, `+0xCF4..+0xD00` the 2.0 coefficients `{c0, c1, c2·G, c3}`. `+0xCD8` and `+0xCF0` are
cleared by the ctor and reset and never read.

**Ctor `0x10023920`**: both parameter blocks `{1, 0, 64, 20, 90, 20, 100, 64, 0, 0, 0, 0}`
(`sub_10018C20`), every pattern 16 steps of rests (`sub_10018D20`, `sub_10018D30`), all state 0
except `cs1 = cs2 = 0.5`, enable 1, gate-off -1. The amp's input and feedback are 0, so a 303 that has
not had a note-off since its song was created outputs exactly 0 (with an "untitled" song active the
activation does not call its slot 6, `docs/export_state.md` §3; checked with rbhost `--dump-state`).

**Step (slot 1, 0x100253A0)** with `tick` and `shuffled` from the clock:
```
t' = pattern.shuffle ? shuffled : tick
v = (u32)(t' - (songMode ? patStart : 0)) % (48*len)     // div, unsigned
if lastPos - v > 24 (signed): if gateOff >= 0: note-off, gateOff = -1; wrap = 1
lastPos = v; step = v/48
if gateOff >= 0:
    if !enableLatch: slide = 0
    if v >= gateOff: note-off, gateOff = -1
elif step != lastStep or wrap:
    gateOff = v + 24; wrap = 0
    if (tick/48) & 15 == 0 (C remainder, the UNshuffled tick):
        if livePattern != pattern: patStart = 768*(tick/768); pattern = livePattern
        enableLatch = liveEnable
    enableLatch ? note-on(step of the (new) pattern) : note-off
    lastStep = step                                      // the step index of the old pattern's length
```
A rest step goes through note-on too: it sets the accent and decay coefficients, the pitch target
(and the pitch unless the last step slid) and the slide flag, but not the gate, the amp constants,
E or the retrigger flag. Note-on uses `n = 6` squarings (`^64`) for the 2.0 sound and `n = 2`
(`^4`) for Vintage, each `fmul st, st` from the stored f32, stored back as f32.

**Other slots.** Slot 2 (0x100255E0) ignores its argument and reads `trunc(pos)` of the song
(`sub_1001F870`): patStart = its bar, pattern = livePattern, enableLatch = liveEnable. Slot 3
(0x100255B0): wrap = 1, patStart = the bar of `trunc(pos)`. Slot 4 is slot 6. Slot 5 (0x10026CC0)
clears the flags and the DSP state of the field list (phase, E, amp, filters, pitch, accent, gate,
amp constants), sets `cs1 = cs2 = 0.5`, gate-off -1, then slot 6; it keeps `+0xCA0`, the patterns, the
pattern position and the 2.0 coefficients. Slot 6 (0x10025630): slide = 0, note-off. Slot 9
(0x10026B60): song mode on copies the live block into `+0x30`, off copies it back. Msg 39
(0x10025CB0, `sub_10018EF0`): the 12 bytes into `+0x30`, the patterns, and in pattern mode the 12
bytes live (so `+0xC85` becomes 0). Slot 11 (0x10026180): controllers 0..8 set the live byte; every
controller sets its `+0xC88` flag.

**Song ctor slots 7 and 8.** The song ctor calls slot 7 on every device (0x10011941), then slot 8
with tick 0 (0x10011985), before its tempo, locate and mixer update. The 303's slot 7 (0x10026C40)
sets the quantisation of controllers 0 and 1 of its track to 32 units of 24 ticks, a bar
(`sub_10021560` → `sub_10014600`, list `+0xD + ctrl`), so a loaded enable or pattern event is moved
to the next bar line; slot 8 (0x10025650) inserts a tick-0 event of each controller 0..8 with the
ctor's value (`sub_1001F900` → `sub_10014750`), which a song's own tick-0 event then replaces.
[C] task-024: the 808's slot 7 is the same function 0x10026C40; its slot 8 (0x10029DA0) inserts
tick-0 events of controllers 0..28 from its pattern-mode copy `+0x30` (§7.2). [C] task-034: every
other device's slot 7 is the no-op 0x1000F4E0, and every device's slot 8 inserts tick-0 events
from its pattern copy: the 909 (0x1002D480, 0..29 in RBS order), the delay (0x10010020, 0..4),
the distortion (0x10010FE0, 0..2), the PCF (0x10017200, 0..6), the compressor (0x1000F420, 0..2)
and the mixer (0x100160F0: 1, 2 and `6..9 + 6c`). The defaults they insert are tabled in
`docs/song_load.md` §4.9.

---

## 7. Drums (808, 909)

### 7.1 Voices (all mono; they add into the device's float bus, in a fixed order) [C]

[C] task-023: the devices' audio slots (808 0x100280B0, 909 0x1002B6F0) first zero the bus (and
any separate output bus), and a voice whose active byte `+8` is 0 adds nothing (tests at
0x1001E4D7, 0x1001E707, 0x1001E9A7), so an idle drum machine outputs exactly 0, not the 1e-6 guard
(the export-start dump of rbhost has the drum channels' peak meters at 0.0).

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
- [C] task-024, the details from the disassembly (Rust: `crates/rb-engine/src/drums.rs`, unit
  tests against hand-computed blocks; the 808 corpus songs match byte for byte):
  - Fields of every class: `+4` the sample object (a trigger with a null one keeps the last), `+8`
    active, `+0xC` the read pointer, `+0x10` frames left, `+0x14` hold, `+0x18` g, `+0x1C` m.
    FILTER `+0x20` c, `+0x24` y; DUAL `+0x20` B's pointer, `+0x24` g2, `+0x28` B's sample;
    PITCHED `+0x20` the sample base, `+0x24` step, `+0x28` phase, `+0x2C` `frames << 8`.
  - Trigger `sub_1001E2B4(sample, f32 gain, m, hold)`: `g = f32(gain * 2^-15)`
    (`dbl_10153640` = 0x3F00000000000000), pointer = the sample's start, left = its frame count
    (`CWaveSample+0x4C`), active. FILTER (0x1001E5C0) then sets `y = 0` and c; DUAL (0x1001E820)
    B, g2 and `left = min(lenA, lenB)`; PITCHED (0x1001EAD0) step, phase 0 and the end.
  - Per block: the multiplier is 1.0 while `hold > 0` (and hold is decremented), else m. SIMPLE
    (0x1001E300): `out = f32(out + g*s)`; after the block the voice ends if the unrounded g is
    below `3e-8f` (`fcom` with `flt_1015364C` = 0x3300D959, before the f32 store) or when
    `left -= 64` reaches 0 or less. FILTER (0x1001E4D0): `y = y*c + g*s` and `out = f32(out + y)`
    per sample, y in the x87 stack for the whole block. DUAL (0x1001E700):
    `out = f32(out + (A + B*g2)*g1)`; the multiplier applies to g2; g1 is constant (written back
    unchanged), and the end test compares g1's bits as an integer with 0x3300D959, so it ends at
    the first block only if g1 started below 3e-8. PITCHED (0x1001E9A0):
    `s = i16(a + ((f*(b - a)) >> 8))` (imul, arithmetic shift, then the low 16 bits),
    `out = f32(out + s*g)`, phase += step per sample; it ends when the phase is above
    `frames << 8` (unsigned); there is no gain test.
  - Slot 1 (0x1001EB30, all classes): active = 0, `+0x18` = 0.
- Each device zeroes its bus, then calls voices in a fixed order. This order affects f32 summation:
  - **808:** BD-A, BD-B, SD, LT, MT, HT, RS/CL, CP/MA, CB, OH, CH, CY.
  - **909:** BD-A, BD-B, SD, LT, MT, HT, RS, CP, CH, OH, Crash, Ride. [C] task-025: the audio slot
    0x1002B6F0 calls FILTER `+0x1904`, `+0x1908`, DUAL `+0x190C`, PITCHED `+0x18F0`, `+0x18F4`,
    `+0x18F8`, SIMPLE `+0x18E0`..`+0x18EC`, PITCHED `+0x18FC`, `+0x1900` in this order.
  - [G] For export the per-instrument ReWire outputs are null, so everything sums into bus 0.

### 7.2 Triggering and accent [C]

- **808 (step 0x10027690).**
  - `accent = f32(acc*AC*0.03125f + 1)`.
  - Instruments trigger in ascending order. As a result, OH and CH on the same step leaves OH cut.
  - [C] task-024 (Rust: `crates/rb-engine/src/tr808.rs`): the step function is the 303's position
    logic (§6.4) without the gate-off: `v = (u32)(t' - base) % (48*len)`, the wrap flag
    `+0x18D2` when `lastPos - v > 24`, and on a new step (`v/48` differs from the live byte 29,
    `+0x18AD`) or a wrap: at the bar line of the unshuffled tick the pending pattern
    (`patStart = 768*(tick/768)`) and the enable latch `+0x18D1`; if latched, the accent is
    stored at `+0x190C` and columns 1..11 with a non-zero byte trigger through `sub_10027894`
    (jump table at 0x10153F58 + 4i). The latch is 1 from the ctor. "OH cut" means the choke: CH
    clears every SIMPLE voice's hold (0x10027F40, `sub_1001E3C0(0)`), so an OH triggered on the
    same step decays (m = 0.998) from its first block.
  - [C] Fields (0x1910 bytes, ctor `sub_10026F70`): `+0x30` the pattern-mode parameters (32
    bytes; ctor `sub_10018FB0`: 1, 0, 40, 100, 64, 64, 100, 64, 64, 100, 64, 0, 100, 64, 0, 100,
    64, 0, 100, 0, 100, 0, 100, 100, 0, 100, 100, 100, 100, 0, 0, 0), `+0x50` 32 patterns of 194
    bytes (ctor: length 16, no steps), `+0x1890` the live parameters (byte 29 the last step),
    `+0x18B0` 32 "changed" flags (panel only, 0x100291A0), `+0x18D0` the playing pattern,
    `+0x18D4` the last position, `+0x18D8` the mod's sample table (slot n at `+4n`, the built-in
    kit slot n = resource 999 + n), `+0x18DC` five SIMPLE voices (RS/CL, CP/MA, CB, OH, CH),
    `+0x18F0` three PITCHED (LT/LC, MT/MC, HT/HC), `+0x18FC` three FILTER (BD A, BD B, CY),
    `+0x1908` the DUAL (SD). The voice-select switches in `sub_10027894` jump through constant
    table entries (0x10153F88..0x1015401C) that IDA shows as computed jumps; each resolves to
    one fixed voice.
  - [C] Slots: 2 (0x10028030) and 3 (0x10028080) as the 303's; 4 (0x100106C0) and 5
    (0x1000F4F0) call 6; 6 (0x1002A2D0) is slot 1 on every voice; 9 (0x10029E30) copies the 32
    bytes between `+0x30` and `+0x1890` like the 303; 11 (0x10029050) sets live controllers
    0..28; msg 56 (0x1002884C, `sub_10019490`) the parameters and patterns into `+0x30`, and in
    pattern mode all 32 bytes live. The audio (0x100280B0) zeroes the bus first.
- **909 (step 0x1002AAC0).**
  - `base = acc*AC*0.03125 + 1`.
  - Step value 1 or 3 gives base×0.8; value 2 or 4 gives base×2.0. Values 3 and 4 also flam (§5).
  - Instruments trigger in **descending** order.
  - [C] RBS 909 values 0..3 reach the step function unchanged (no shift to 1..4): 1 = ×0.8, 2 = ×2.0,
    3 = ×0.8 with flam; 4 never occurs. The AC column holds 0 or 2 (`docs/song_load.md` §4.8).
  - [C] task-025 (Rust: `crates/rb-engine/src/tr909.rs`; the 33 909 corpus songs match byte for
    byte): the step function is the 808's position logic (§6.4) with the 909's offsets: pattern
    `+0x18D3`, latch `+0x18D4` (1 from the ctor), wrap `+0x18D5`, last position `+0x18D8`, last
    step the live byte 30 (`+0x18AF`), plus the flam (§5). Accent
    `f32((AC*acc)*0.03125f + 1.0)` (`flt_10154368`, `dbl_10154348`) at `+0x1918`; then columns 11
    down to 1: values 1 and 3 set the trigger gain `+0x191C` to `f32(accent*0.8)`
    (`dbl_10154358` = 0x3FE999999999999A), 2 and 4 to `f32(accent*2.0)` (`dbl_10154360`); any other
    value triggers nothing. Each instrument then runs through one of three identical 11-case
    switches (0x1002AD2D, 0x1002B03C, 0x1002B359; tables at 0x1015436C, 0x10154398, 0x101543C4).
  - [C] Fields (0x1920 bytes, ctor `sub_1002A380`): `+0x30` the pattern-mode parameters (33 bytes;
    `sub_100195A0`, in the DLL's order: 1, 0, 32, 100, 64, 64, 127, 100, 64, 64, 64, 100, 64, 127,
    100, 64, 127, 100, 64, 127, 100, 100, 100, 32, 127, 100, 64, 100, 64, 64, 0, 0, 0), `+0x51` 32
    patterns of 194 bytes, `+0x1891` the live parameters (byte 29 the flam interval, 30 the last
    step), `+0x18B2` 33 "changed" flags, `+0x18DC` the mod's sample table (slot n = resource
    1000 + n, 25..45), `+0x18E0` four SIMPLE voices (RS, CP, CH, OH), `+0x18F0` five PITCHED (LT,
    MT, HT, crash, ride), `+0x1904` two FILTER (BD A, BD B), `+0x190C` the DUAL (SD), `+0x1910`
    the flam flag, `+0x1914` flamTick, `+0x1918` the accent, `+0x191C` the trigger gain.
  - [C] The DLL keeps bytes 20..22 of its parameter blocks as hi-hat, rim shot, clap levels; the
    RBS chunk, msg 73 and the controllers use rim shot, clap, hi-hat. Msg 73 (`'I'` at 0x1002BE90,
    `sub_100199A0` → `sub_100196F4`) permutes in, `sub_10019624` permutes back (slot 8, the
    host's read-back), and set-param (slot 11, 0x1002C6D0) maps controller 20 to `+0x18A6`, 21 to
    `+0x18A7`, 22 to `+0x18A5`. Set-param takes controllers 0..29 (`cmp edx, 1Dh; ja` at
    0x1002C6D7); Hex-Rays drops case 29 (0x1002C80B, `mov [ecx+18AEh], al`, the flam interval),
    which IDA files under another switch.
  - [C] Slots: 2 (0x1002B670), 3 (0x1002B6C0) as the 808's; 4, 5 call 6; 6 (0x1002DD40) slot 1 on
    every voice; 7 is the 303's 0x10026C40 (controllers 0 and 1 quantised to a bar); 8
    (0x1002D480) inserts tick-0 events of controllers 0..29 with the `+0x30` values in RBS order;
    9 (0x1002D510) copies the 33 bytes between `+0x30` and `+0x1891`; msg 73 puts the block into
    `+0x30` and, in pattern mode only, all 33 bytes live.
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

[C] task-024, checked in the disassembly and by the corpus (`crates/rb-engine/src/tr808.rs`;
`k` is `f32(1/127)` = 0x3C010204, every product in f64 in this order, one f32 store of the gain,
then the voice's `f32(gain * 2^-15)`):
- BD and CY (`sub_1002A0B4`): `x = (((L*k)*L)*0.005f)*acc`, BD `gain = (1.0 - c)*x`, CY
  `gain = x`; `m = DEC16[decay>>3]`, hold 0. The BD's voice: with `word_1014CBD0` non-zero
  voice B plays, voice A's m becomes 0.99f (0x3F7D70A4) and the flag 0; else voice A plays,
  B's m becomes 0.99 and the flag 1 (a fade of the other voice, whatever its state).
- SD (`sub_1002A1E4`): `g1 = f32((((L*k)*L)*0.025f)*acc)`, `g2 = f32((snappy*k)*(snappy*k))`,
  `m = 1`, hold 0, so neither gain decays: the snare plays until its shorter sample ends. A is
  slot `SD[(tone*5)>>7] + 1` (SD1..SD5), B is slot `N[SEQ808[c]] + 1` (N1..N4) with the counter
  pre-incremented.
- Toms (`sub_1002A004`): PITCHED voice `i - 3`, `m = 1`, hold 0; LT (selector set) / LC uses
  controllers 9..11, MT / MC 12..14, HT / HC 15..17 (level, tuning, selector).
- RS/CL, CP/MA, CB, OH and CH (inline in `sub_10027894`): SIMPLE voices 0..4, `m = 1` and hold 0
  except OH (`m = 0.998f` = 0x3F7F7CEE, `hold = 4*decay`). Their gain never decays, so they end
  with their sample; OH decays after its hold or a CH.

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

[C] task-025, checked in the disassembly and by the corpus (`crates/rb-engine/src/tr909.rs`;
`k` = 0x3C010204, `gain` is the step's f32 trigger gain `+0x191C` (accent times 0.8, 2.0 or 1.5),
every product in f64 in this order, one f32 store, then the voice's `f32(gain * 2^-15)`; every
`DT[x]` index is `(x << 4) >> 7`, i.e. `x >> 3`; hold 0 unless noted):
- BD (`sub_1002DB24(level, decay, tune, attack)`, controllers 3, 6, 4, 5): sample slot
  `25 + (tune >> 5)` (bd1..bd4), `gain = f32(((((L*(L*k))*k)*gain)*(1.0 - c))`,
  `c = C8[attack >> 4]`, `m = DT[decay >> 3]`; the voice by `word_1014E544` exactly as the 808
  (the other voice's m becomes 0.99f).
- SD (`sub_1002DC54(level, tone, tune, snappy)`, controllers 7, 9, 8, 10):
  `g1 = f32(((L*(L*k))*k)*gain)`, `g2 = f32(snappy*0.01f)` (`flt_10154648` = 0x3C23D70A),
  `m = 0.998f` (0x3F7F7CEE, on g2), `hold = tone + 30`; A is slot `25 + SD[(tune*5) >> 7]`
  (sd1..sd5), B is slot `25 + SDN[SEQ909[c]]` (sdn1..sdn3) with `word_1014E548` pre-incremented,
  `& 31`.
- Toms, crash, ride (`sub_1002DA24(sample, level, tune, decay, i)`): PITCHED voice `i - 3`
  (crash 3, ride 4), `gain = f32(((L*(L*6.2e-5f))*kv)*gain)` (`flt_10154600` = 0x38820610),
  kv 1.2 / 1.1 / 1.4 / 0.6 / 0.6 (`flt_1015460C`, `flt_10154608`, `dword_10154604`,
  `flt_101545FC`), `m = DT[decay >> 3]` (crash and ride pass decay 127: m = DT[15] = 1),
  `step = T909[tune]`. The toms use controllers 11..19 (level, tune, decay each), crash 25, 26,
  ride 27, 28.
- RS, CP, CH, OH (`sub_1002D8B4(sample, level, decay, i)`): SIMPLE voices 0..3,
  `gain = f32((((6.2e-5f*L)*L)*c)*gain)` with c = 1.1, 2.3, 0.8, 1.1 (`flt_101545D4`,
  `flt_101545E4`, `flt_101545DC`); RS and CP pass decay 127 (m = DT[15] = 1), OH `m = DT[d >> 3]`,
  CH `m = f32(DT[d >> 3]^8)` (three `fmul st, st` on the f32 DT value). CH and OH read the
  hi-hat level (controller 22), their decays are controllers 23 and 24. CH first sets the OH
  voice's m to 0.996f (0x3F7EF9DB, `sub_1001E3D0`); with the descending order an OH on the same
  step is triggered first and choked at once.

[C] task-019, the static drum tables and the exact builders:
- 808: `SD` sample slots i16[5] at 0x10034090 (1..5), SEQ808 u16[32] at 0x1003409C, `N` noise
  sample slots u16[4] at 0x100340DC (6..9), TONE8 f32[8] at 0x100340E4, DEC16 f32[16] at
  0x10034104.
- 909: `SD` sample slots i16[5] at 0x10034210 (4..8), SEQ909 u16[32] at 0x1003421C (values 0..2),
  `SDN` slots u16[4] at 0x1003425C (9, 10, 11, and an unused 0), C8 f32[8] at 0x10034264.
- T808 (0x10027061..0x10027141): `s = pow(2, 1/192)`, `k = 256 / (1*s^64)` with s^64 by six
  squarings, `T808[i] = trunc(pow(s, i)*k)` (fistp under a truncating control word, low 16 bits).
  T909 (0x1002A471..) is the same with `pow(2, 1/144)`. `T808[64]` is exactly 256 only because
  `pow(s, 64)` equals the squared value bit for bit; for the 909 it is 256 + 6 f64 ulps.
- DT (0x1002A551..0x1002A57C): `x = 1f`; for j = 15 down to 0, `DT[j] = f32(x)`, `x = x - 2.5e-5`
  with x kept in the x87 register (f64), not re-rounded to f32.

### 7.4 Samples and RBM mods [C, except where noted]

- **Built-in samples.** DLL resources of type `"AIFF"`:
  - 808: IDs 1000..1023 (BD, SD1-5, N1-4, CB, CH, CL, CP, CY, HC, HT, LC, LT, MA, MC, MT, OH, RS).
  - Startup sound: ID 1024.
  - 909: IDs 1025..1045 (bd1-4, sd1-5, sdn1-3, lt, mt, ht, rs, hc, chh, ohh, ccy, rcy).
  - rb does not ship them: `rb_engine::samples` reads them at run time from the user's
    `ReBirth.dll` (task-016, task-020).
  - [C] task-020, how the DLL loads them. `sub_1001A030` (0x1001A030) builds the sample table of 47
    `CWaveSample` objects: slots 1..24 = IDs 1000..1023, slots 25..45 = IDs 1025..1045, slot 46
    = ID 1024 (the startup sound); a mod replaces single slots (`sub_1001A500`). Each is loaded by
    `sub_1001A3E0` → `sub_1000D630` (`FindResourceA(module, MAKEINTRESOURCE(id), "AIFF")` through
    `sub_1000B620` at 0x1000B663: the type is the string "AIFF", the ID a 16-bit integer, no
    language given) → `sub_10003300`.
  - [C] The 2.01 DLL has exactly these 46 resources (one language each), all mono 16-bit AIFF;
    1034..1036 (the 909 `sdn1`..`sdn3`) have an SSND offset of 458 bytes, the rest 0. Frame counts, 1000..1045: 72704, 8800 x9, 51200, 4992, 1816, 87040, 174430, 11680,
    17498, 17056, 22063, 1468, 11603, 15351, 26578, 690, 84575 (1024), 22062, 22001, 22036,
    22099, 14007, 14008, 14006, 14014, 14034, 12180 x3, 28072, 20427, 20862, 5006, 30365, 8401,
    25811, 43690, 42458. `crates/rb-engine/tests/samples.rs` holds them with the sample sums.
- **AIFF parsing.** COMM and SSND chunks, 16-bit only. Big-endian int16 values are used as they are.
  The sample rate is ignored, and only channel 0 of multichannel files is used.
  - [C] task-020, the details (`sub_10003300` 0x10003300 ties them together):
    - Chunk walk `sub_10004F94` (0x10004F94): the 12-byte header is checked by `sub_100054D4`;
      then chunks are walked by id, a pad byte after an odd size (the reader's flag at `this+4`
      is set for AIFF); the first chunk with the wanted id is used. [G] What `sub_100054D4`
      accepts (whether an AIFC form type is rejected) is not confirmed; rb requires FORM+AIFF,
      and all 46 resources pass.
    - The drum voices read the sample as `i16` (the CWaveSample buffer: `frames + 512` i16,
      `rb_engine::samples::Sample::padded`).
    - COMM `sub_10001674` (0x10001674): u16 channels, u32 frames, u16 bits, 80-bit extended rate
      (`sub_10004110`, truncated to an integer by `fistp`). `sub_100016F4` accepts a non-zero
      channel count, frame count and rate, and bits 8..=32. `sub_10003170` then flags `bits ==
      16`; any other depth throws `XUnsupportedFormat` (0x10003376..0x10003388).
    - SSND `sub_10002FC4` (0x10002FC4): u32 offset, u32 blocksize; `offset > 0` bytes are
      skipped (vtable +0x2C seek) before the audio.
    - Mono: `2 * frames` bytes are read to the buffer start and byte-swapped in place
      (`sub_10009990`). Multichannel: `sub_10009DD0` reads 0x4000-byte chunks, swaps, and
      `sub_100099DC` copies channel 0 with a stride of the channel count (`a8 = 0`).
    - `CWaveSample` ctor `sub_1000A9C0` (0x1000A9C0): `this[1]` = the rate, `this[19]` = frames,
      buffer = `2 * frames + 1024` bytes, zeroed, so the audio starts at index 0 and 512 zero
      samples follow it (not before it).
- **RBM mods: the full format is in `docs/rbm.md`** (task-030). In short:
  - [C] Container (EXE 0x41813E / 0x414D18): an IFF `CAT ` of type `PRBM` (there is no `RBM `
    chunk) holding `HEAD` (256 bytes, not read), the `EMBF` chunks (a NUL-terminated file name, then
    the file) and `INFO` (1280 bytes: text, URLs, the menu name at 1003, the version at 1073).
  - [C] Samples are matched by file name, case-insensitively, first match (`sub_416710`); the
    names are at EXE 0x4BB388 (808), 0x4BB520 (909), 0x4BB378 (`Startup.aif`). A missing sample
    falls back to the built-in one, per slot.
  - [C] Scrambling: an EMBF payload (name included) larger than 1024 bytes is descrambled from byte
    1024 on before AIFF parsing (DLL 0x1001A500 → 0x100163D0): `P = (17*key % 1925) + 123`,
    `base = (key&1)*2048`, `buf[1024+n] += table[base + n % P]` (u8); `key` = the low byte of the
    chunk's ordinal in the CAT, HEAD = 0.
  - [C] The 4096-byte key `table` is two 2048-byte pages of `RB20FUL.DAT` (the CD; app field
    `this[325]`) at `256 + sum[j] % (0x8000000-2048)`, `sum = {0x2000, 0x3000}` plus, per EMBF, the
    big-endian u32 words 0..127 and 128..255 of its first 1024 bytes (0x65-padded). The
    anti-tamper stride of `sub_4149EC` is 4: checked against every page of the real `RB20FUL.DAT`
    (`tools/rbm.py check-dat`).
    Shown on the four shipped mods with the original DLL (rbhost): all 46 slots equal.

---

## 8. Effects [C]

All effects run their parameter update when `(cnt & 15) == 0`, then increment the counter
(`cnt` starts at 0). PCF reads its parameters live instead.

### 8.1 Distortion

Amount and shape above 127 index past their tables unchecked: [OB-9](original_bugs.md).

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

[C] task-026, from the disassembly (`CDistInsert`, vtable 0x10033144: [4] stop 0x100106C0 and
[5] reset 0x1000F4F0 both call slot 6, the no-op 0x10012C00; [8] 0x10010FE0; [9] 0x100111F0;
[10] messages 0x10010D60; [11] set-param 0x10010FB0; [15] block 0x10010A50; [16] 0x10010D40).
The engine's `crates/rb-engine/src/dist.rs` follows it and renders every distortion song of the
corpus exact:
- **Object.** ctor 0x10010700: live block `+0x39` (enabled, amount, shape, meter byte) and
  pattern copy `+0x3D`, both `sub_10019B30` = 1, 70, 0, 0; call counter `+0x30` (word), the
  counter at the last mixer update `+0x34`, clip flag `+0x38`, gains `+0x44` (in), `+0x48` (out),
  peak `+0x50`, targets `+0x54`, `+0x58`, row `+0x5C` (word), all 0.
- **One object, every routed channel.** The mixer calls slot 15 once per channel whose switch is
  on, so the counter and the smoothing advance once per routed channel per block. Disabled
  (`+0x39` = 0, 0x10010A61) the call returns at once: the bus passes unchanged and the counter
  stands.
- **Update** 0x10010894, when `counter & 15 == 0` before the increment: first the meter (the peak
  as an integer above 0x3F800000 gives byte 0x7F, else `trunc(peak*127f)`, then `peak =
  f32(peak*0.92f)`); clip flag = (shape == 0); `S = (shape * f32(1/127)) * 3.999`, `A` the same
  for amount (`dbl_10151B20` = 0x400FFDF3B645A1CB); `i = trunc(S)`, `j = trunc(A)`, `fs = S - i`,
  `fa = A - j`; with the grid `G[shape][amount]` at 0x10151AB8 (f32, rows: 1.2 0.4 0.28 0.2 0.15 /
  0.35 0.2 0.15 0.14 0.12 / 0.2 0.15 0.13 0.12 0.12 / 0.15 0.12 0.11 0.1 0.1 / 0.12 0.1 0.08 0.08
  0.08): `top = (G[i+1][j+1] - G[i+1][j])*fa + G[i+1][j]`, `bot = (G[i][j+1] - G[i][j])*fa +
  G[i][j]`, `r = fs*(top - bot) + bot`; `+0x54 = f32((T[amount] + 0.0625)*2048)`, `+0x58 =
  f32((r*5)*0.00390625)`, row = `(shape*32)/128` = `shape >> 2`.
- **Block.** The peak meter takes `|bus[meterIndex]|`. The gains are smoothed every call,
  `f32(g*0.9 + t*0.1)` (0x10010AC2), and the shaper loop reads the **stored f32** values (unlike
  the delay). Shaper (0x10010BF9): `v = gIn*x + 128`; `v < 0` gives `W[0]`, `v > 254`
  (`flt_10151B94` = 254.0) gives **`W[254]`** (`fld [ebp+3F8h]`), else `i = trunc(v)` and `y =
  (v - i)*(W[i+1] - W[i]) + W[i]`; `out = f32(y*gOut)`. Clip (0x10010B01): `g = 16*T[a] + 1`,
  `h = 16*T[90] + 1` when `a >= 90` else `g`, `th = 1/h`, `-th = th * -1.0`; `y = g*x`, `th` if
  `y > th`, else `-th` if `y < -th`, else `y`; `out = f32(y)`; the gains are unused there.
- **Slot 16** 0x10010D40 (every mixer update): the meter byte is cleared when the counter has not
  moved since the last update. No audio effect.
- Slot 8 inserts the pattern copy's controllers 0..2 into the track at the ctor's tick 0; slot 9
  copies the 4-byte block (on: live to pattern; off: pattern to live); slot 11 sets controllers
  0..2 live.

**W[32][256]** (BSS `flt_100D47A4`, built once by `0x10011210` from the dist ctor, guard
`byte_100D47A0`; [C] task-011: corrected from "0x10011210", the builder's address). Uses `fsin`, with phase increment `(t²/961)*(π/8) + π/1024`:
- `W[t][128±k] = ±f32((sin(ph)*256*(1-y) + 256*y*y)*0.7)` with `y = k/128`.
- `W[t][0] = -W[t][255]`.
- [C] task-019, exact: the constants are f32 1/961 = 0x3A88642A (0x10151C00), and "π/8" =
  0x3EC90FD0 (0x10151C04) and "π/1024" = 0x3B490FD0 (0x10151C08), which are not f32(π/8) =
  0x3EC90FDB but π cut short; with f32(π/8) 7780 of the 8192 entries come out 1-2 ulps off.
  `inc = (t*t*c961)*cπ8 + cπ1024` in f64. For e = 128..255, with `ph`, `y` and `z` starting at 0
  and advancing (f64) by `inc`, 1/128 and 1/128 after each entry:
  `v = ((sin(ph)*256f)*(1 - y) + ((y*z)*256f))*0.7`, then `W[t][e] = f32(v)` and
  `W[t][256-e] = f32(-v)`, in that order, so `W[t][128]` is -0.0.

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
[C] task-019: the constants {b2, A, g} at 0x10100260 are bit-identical to the 303 smoother's
(§6.2): the same code with the same constants (0x10152490..0x101524B0).

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
- [C] task-019, exact (LP 0x10017310..0x1001753F, BP 0x100175C0..0x10017803): as FILT (§6.2) but
  `base = 0.4/pow(K, 511)` and `W = (pow(K, i)*base)*aW` with the constant K itself (not
  recomputed), the index scale 1/512, so `q = (i/4)^2*0.000125 - 0.3`, and no c3.
  LP: `c2 = ((c1 + (1-c0))*2f) / ((D'+1f)*0.8)`. BP: `c2 = ((cos(W*π)*sqrt((1f-c0) + c1))*3f) /
  ((D'+1f)*0.8)`, π = 3.141592654; D' = 3 when D > 4 (BP reuses its f32 3.0 at 0x10152484).

[C] task-027, from the disassembly (`CPCFInsert`, vtable 0x10033AD4: [4] 0x100106C0 and [5]
0x1000F4F0 call slot 6, the no-op 0x10012C00; [8] 0x10017200; [9] 0x10017290; [10] messages
0x10016C50, msgs 111..116, 116 the data; [11] 0x10016EF0; [13] 0x10016F40, the panel report;
[15] block 0x100166D0; [16] 0x10016C30). `crates/rb-engine/src/pcf.rs` follows it and renders
every PCF song of the corpus exact:
- **Object.** ctor 0x10016490: live block `+0x38` and pattern copy `+0x40`, both `sub_10019C10`
  = 1, 70, 64, 40, 24, 90, 1, 0 (enabled, frequency, resonance, amount, wave, decay, mode, meter
  byte); `+0x50..+0x88` 0, then the last step `+0x74` = -1 (`sub_100178D0`) and the change flags
  `+0x48..+0x4F` 0. Fields: LP state `+0x50` (y1), `+0x54` (y2); BP state `+0x58`, `+0x5C`; E
  `+0x68`; base `+0x6C`; envelope `+0x70`; attack `r` `+0x78`; decay `+0x7C`; smoother `+0x80`
  (y1), `+0x84` (y2); peak `+0x88`; counter `+0x30`, its value at the last mixer update `+0x34`.
- **Block, control.** Disabled (`+0x38` = 0): returns at once. Else the meter update 0x10016664
  when `counter & 15 == 0` (byte `+0x3F` = 0x7F when the peak's bits are above 0x3F800000, else
  `trunc(peak*127f)`; `peak = f32(peak*0.92f)`), counter++, the peak takes `|bus[meterIndex]|`.
  `tick = sub_1001F870()` (the song position after this block's sequencer step); the wave byte
  `w` indexes `len = word_10033310[w]` and `res = word_10033380[w]` **unchecked**; `q = tick/24`,
  `/48`, `/96` (signed `idiv`) for `res` = 32, 16, 8, and for any other `res` the stack slot
  `var_1C` keeps whatever was there [G: not modelled, the engine keeps its last quotient];
  smoother `s = (A*y1 + ((freq*512f)*(1/128f))*g) - b2*y2`, `+0x84 = +0x80` stored **before**
  `step = q % len` (`idiv` at 0x10016811), then `+0x80 = +0x6C = f32(s)`. A new step (`!= +0x74`)
  stores it, sets `dec = dword_100332D0[decay >> 3]` and reads `v = byte_100333F0[w*32 + step]`; a
  non-zero `v` triggers `E = (v*amount) >> 5`, or `511 - base` when `E + base > 511`; odd `v`:
  `r = 0.995f` (0x3F7EB852), keep 0.995f, take f32 0x3BA3D700 (= 1 - 0.995f); even `v`: `r = 0`,
  keep 0, take 1. Without a trigger: keep `r`, take `1 - r` (f64), E = `+0x68`.
- **Block, audio.** Row `(res*8) >> 7`; mode 1 the LP table 0x1010C270, else BP 0x10100270. Per
  group of 4: `E *= dec`; `env = env*keep + (E + base)*take`; `i = trunc(clamp(env, 0, 511))` (C0
  test for the 0, so a NaN also gives 0); `(c0, c1, c2) = Tab[row][i]`; per sample `y = (x*c2 +
  c0*y1) - c1*y2`; LP outputs `f32(y)`, BP `f32(y - y1_old)`. y1, y2, env and E stay in x87
  registers (BP keeps env in a stack double) for the whole block; at the end the filter's state,
  `+0x68 = f32(E)` and `+0x70 = f32(env)` are stored, and the other filter's state is zeroed.
- **Past the patterns** (defects of the original: [OB-5 to OB-8](original_bugs.md), clamps
  OB-9 to OB-11). RBS42.txt gives wave 0..0x37 and the exe's track load accepts 0..127; the
  DLL checks nothing, so a wave of 55..127 reads the bytes after the 55 patterns (the steps end at
  0x10033AD0, the `CPCFInsert` vtable follows), all in the file up to 0x10034400. 73 of the
  waves 55..127 read a `res` other than 8, 16 or 32, so their quotient is the stale `var_1C`
  [G]. 17 of them, waves 55, 111 and 113..127, read `len` 0, and the `idiv` raises a divide
  error in every block (wave 112 reads `len` 120, so its steps reach 88 bytes past its own 32).
  Wave 64 reads `res` 0 (stale quotient) and `len` 16, whose 16 steps are all 0: no trigger
  ever, the filter follows the base alone. Wave 127 is the corpus' faulting wave. `SafeSoundCallback` (0x1001AE40) wraps the render in a C++ `try` with `catch (...)`
  (FuncInfo 0x10152AD8, handler 0x1001B055, continuation 0x1001B03D), which catches it; the
  continuation is past both `sub_100139E0`'s int16 conversion and the zero fill of the not-locked
  path, so the call returns with the caller's buffer unwritten (`pcf_*_wave127` export all zeros,
  task-039) and with the song as far as the block got (sequencer step, the channels before the
  PCF; `song+0x208` is stored only after the block loop, 0x10013BBA). The skipped `ReleaseMutex`
  calls leave the Win32 mutexes owned, which a single-threaded host such as rbhost re-acquires
  (`WaitForSingleObject(h, 0)` in `sub_10009454`). The engine models the fault
  (`frame::Fault`): the block and the render call end there; `Engine::fault` records the first
  one and the count, and the `rb` CLI warns. An export repeats what its int16 buffer held: rbhost
  renders the idle calls and every chunk into one static `chunk_buf`, and the app's export loop
  `sub_4280D5` renders every chunk into the one buffer `sub_427988` allocates (`operator
  new(0x2000)` at 0x427C2F, never cleared; the callback at 0x4281C3, the file write at
  0x4281F2), so a faulting chunk repeats the previous chunk, and an export that faults from the
  start is all zeros [G for the app: its buffer is uninitialized heap memory, which its
  `pcf_*_wave127` exports show as zeros]. The engine's export shares one zeroed buffer as rbhost
  does.
- **The file's bytes are the preferred base's.** Decided by the user on 2026-10-27 (task-027):
  `PCF_DATA` stays a generated static table of the file's bytes. The range
  0x10033AD0..0x10034400 holds 171 base-relocated pointer words (the `.reloc` HIGHLOW entries:
  the `CPCFInsert` vtable at 0x10033AD4 and later), and 57 waves of 55..127 have such a word
  among their 32 step bytes (35 of them within their pattern length, so a step can read it).
  Their velocities are the ones of a DLL loaded at its preferred base 0x10000000, as it is in the
  app and in rbhost.
- **Slots.** 8 inserts the pattern copy's controllers 0..6 at tick 0; 9 copies the 8-byte block
  (on: live to pattern; off: pattern to live); 11 sets controllers 0..6 live; 16 clears the meter
  byte when the counter has not moved since the last update.

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

[C] task-027, from the disassembly (`CCompInsert`, vtable 0x10032B24: [4] 0x100106C0 and [5]
0x1000F4F0 call slot 6, the no-op 0x10012C00; [8] 0x1000F420; [9] 0x1000F4B0; [10] messages
0x1000EFD0, msgs 99..104, 104 the data; [11] 0x1000F220; [14] stereo 0x1000EDE0; [15] mono
0x1000EC70; [16] 0x1000EFB0). `crates/rb-engine/src/comp.rs` follows it and renders every
compressor song of the corpus exact:
- **Object.** ctor 0x1000E990: live block `+0x38` and pattern copy `+0x3D`, both `sub_10019B90` =
  1, 70, 64, 127, 0 (enabled, amount, threshold, then two meter bytes); counter `+0x30` 0, X
  `+0x48` = 1.0, offset `+0x4C` = 1.0, env `+0x50` = 1.0, attack `+0x54` = 0x3C23D70A (0.01),
  release `+0x58` = 0x3F7F9724 (0.9984), G `+0x5C` = 0, floor `+0x60` = 1000.0, hold `+0x64` 0,
  reload `+0x66` = 300, peak `+0x68` 0, change flags `+0x42..+0x46` 0. Nothing but the ctor writes
  the attack, the release and the reload.
- **Update** 0x1000EB44, when `counter & 15 == 0` before the increment: byte 3 = `trunc(min(G*18,
  127))` (the gain meter); `af = amount*f32(1/127)`; `off = 1 - af*0.99`; `X = (af*(th*(1/32) +
  1))*0.3662109375`; `floor = ((th*f32(1/127))*80000f)*(1/32768) + 0.01220703125` (each stored as
  f32; `dbl_10151628`..`dbl_10151658`, `flt_10151660`..`flt_10151670`); byte 4 = the peak meter
  (0x7F when the peak's bits are above 0x3F800000, else `trunc(127f*peak)`), `peak =
  f32(peak*0.92f)`.
- **Blocks.** Disabled: return. Else the update, counter++, the peak takes `|first bus[meterIndex]|`.
  Per group of 4: `s = |x1 + x3|` (stereo `|((L1 + L3) + R1) + R3|`, the sign test `fcom 0.0`);
  if `s > env`: `env = env + (s - env)*atk`, `hold = 300`; else if `hold == 0`: `env = env*rel +
  1e-6f`; else `hold--`; `env = floor` when below it (C0); `d = ((X/env + off) - G)*0.25`; sample k
  of the group becomes `f32(x_k * G_{k+1})` with `G_{j+1} = G_j + d`. env and G stay in x87
  registers (stereo keeps env in a stack double) and are stored as f32 at the block end. Mono
  takes the channel bus; stereo (the master, `compId == 1`) takes L and R, both scaled by the one
  gain.
- **Slots.** 8 inserts the pattern copy's controllers 0..2 at tick 0; 9 copies the 5-byte block
  (on: live to pattern; off: pattern to live); 11 sets controllers 0..2 live; 16 clears both meter
  bytes when the counter has not moved since the last update.
- **The mixer's change flags** (task-022's review note): the mixer device's set-param 0x10015C00
  stores the compressor and PCF ids as dwords (`+0x34`, `+0x38`; msg 143's `sub_10019D14` widens
  the payload bytes the same way, and the mixer update 0x10012C14 reads them as dwords) and sets
  the change bytes `+0x81` (0x10015CFB), `+0x82` (0x10015CE4) and `+0x85 + 6c + k` (0x10015CCC).
  The panel's own set message, msg 145 (the handler `sub_10015520` dispatches through the table
  0x10151ECC indexed by the raw byte, `[145]` = 0x10015900; its switch at 0x10015926), stores the
  value and clears the byte (0x1001593F, 0x10015981), then records the change (`sub_10012BC0`).
  The only reader is mixer slot 13, `sub_10015D40`: it reads the channel bytes `+0x86..+0x89 + 6c`
  from 0x10015DEE on and the id bytes `+0x81`, `+0x82` at 0x10015F9B..0x1001609E, clears each one
  it finds set and posts msg 147 (channel fields) or 145 (ids) to the host's port (`sub_1001D5F0`
  into `byte_1011D0B8`): the panel only, no audio effect, not modelled.

### 8.4 Delay (send; global buffer of 131072 f32, mask 0x1FFFF, prefilled with 1e-6)

A pan above 127 indexes past `PAN[127]` unchecked: [OB-10](original_bugs.md).

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
- [C] Corrected by task-004: the write pointer (delay `+0x6C`) is per delay object, 0 when the song
  is created, and never reset afterwards; the buffer is global and is refilled with 1e-6 by msg 33,
  on a song switch, and once when a disabled delay runs (`docs/export_state.md` §6.1, §7.4).
- [C] task-023: an **enabled** delay changes the output even with every send at 0. The send bus is
  filled with the 1e-6 guard and every channel adds `bus*sendGain` with `sendGain = 1e-6` at send 0
  (§4), so the delay line carries the guard plus 1e-6 × the channels' signal, and the delay adds
  it, D samples later, into the master at a gain of about 1 (pan 64: `1e-6 + PAN[127][64]`).
  rbhost `--dump-float` on `303_saw_lowcut` (delay enabled, steps 3, 16ths, feedback 0, no send):
  the float output at frame 0, before any note, is 1.2676785e-6 = 2e-6 × master gain; the engine
  without a delay gives 6.338391e-7 = 1e-6 × master (the master guard alone). Missing, it flips
  the int16 truncation of 3516 frames (7032 samples) of `303_saw_lowcut` and 1656 frames of
  `303_vintage_saw`, all by -1 LSB; the part fed by the 303 starts after D = 16537 samples at 120
  BPM. Proof that this is the whole difference: with a test-only model of the delay (the code
  below; task-023, removed in task-049 once the engine's delay of task-026 matched) the engine
  rendered all 36 TB-303-only diag songs equal to their corpus references; with the engine's
  delay they are (`crates/rb-engine/tests/corpus_tb303.rs`). Only the disabled path
  (0x1000FAD1) leaves L/R untouched. Every song of `tests/songs/diag` has the delay enabled (the
  template's), so the disabled path's only reference check is the rbhost renders of the 36
  songs with the delay disabled (`crates/rb-engine/tests/fixtures/tb303_nodelay.json`,
  `tools/nodelay.py`).
- [C] task-023, the enabled block 0x1000F940..0x1000FACE and the update 0x1000F754 (every 16
  blocks, when the block counter `+0x30 & 15 == 0` before it is incremented, so in the first
  block): `q = mode == 0 ? 3*T/100 : 4*T/100` (signed `idiv`, T = the song's tempo from
  `sub_1001FCF0`), `D = min((u32)(26460000*steps)/(u32)q, 0x1FFFF)` (`+0x68`), targets
  `f32(1e-6f + PAN[127][pan][k])` (`+0x54`, `+0x58`), `fb = f32(fbByte*(1/127))` (`+0x64`). Per block
  `g = g*0.9 + t*0.1` is stored as f32 (`+0x5C`, `+0x60`) but the loop uses the unrounded value; per
  sample, from `r = (w - D) & 0x1FFFF`: `x = buf[r]; L = f32(x*gL + L); R = f32(R + x*gR);
  buf[w] = f32(x*fb + send[i]); send[i] = 1e-6`; r and w advance (`w` is `+0x6C`, never reset). At
  the corpus export start (rbhost dump) `D = 16537`, `w = 57344` (9088 blocks × 64 mod 2^17), the
  counter 9088 and both gains f32 1.0000005.
- [C] task-026, the rest of `CDelayEffect` (vtable 0x10032CB4: [5] reset 0x1000F4F0 calls [6]
  0x100100E0; [7] no-op 0x1000F4E0; [8] 0x10010020; [9] 0x100100B0; [10] messages 0x1000FB40;
  [11] set-param 0x1000FDC0; [14] block 0x1000F940), implemented in
  `crates/rb-engine/src/delay.rs`, which renders every corpus song exact (the 123 diag songs with
  no PCF or compressor routed, split among `crates/rb-engine/tests/corpus_tb303.rs`,
  `corpus_tr808.rs`, `corpus_tr909.rs` and `corpus_effects.rs`, task-049):
  - ctor 0x1000F5A0: live block `+0x34` and pattern copy `+0x3A` both `sub_10019AB0` = 1, 3, 1,
    80, 64, 0 (byte 5 is the meter byte); `+0x54..+0x70` 0 (targets, gains, feedback, D, w,
    peak).
  - Slot 6 fills `flt_10054794` with 0x358637BD and sets `byte_10054790` = 1; slot 5 calls it,
    so msg 33 refills the line. The enabled block clears the gate (0x1000F95F); the disabled one
    (0x1000FAD1) refills the line only while the gate is clear, refills the send bus, and sets the
    meter byte to 0 and the peak `+0x70` to 1e-6.
  - The update's head (0x1000F75B..0x1000F7BA) averages the tempo over 25 updates into `+0x48`,
    `+0x4C`, `+0x50`, but 0x1000F7E2 then overwrites `+0x4C` with the current tempo on every
    update, before both length reads (0x1000F7EC and 0x1000F83F; the `jne` at 0x1000F7EA only picks
    the formula): the average is dead on every path ([OB-12](original_bugs.md)), D follows the tempo at every update. Its tail is the
    meter: the peak as an integer above 0x3F800000 gives byte 0x7F, else `trunc(peak*127f)`, into
    live byte 5; then `peak = f32(peak*0.92f)`. The block's peak is `|send[meterIndex]|`.
  - Slot 8 inserts the pattern copy's controllers 0..4 into the track at tick 0; slot 9 copies
    the 6-byte block (on: live to pattern; off: pattern to live); slot 11 sets controllers 0..4.
  - Constants: `flt_101517C0` 1e-6, `dbl_101517B8` 1/127 (0x3F80204081020408), `flt_101517C8`
    0.92, `flt_101517CC` 127, `dbl_101517D0` 0.9, `dbl_101517D8` 0.1. The pan row
    `flt_10054150` is `PAN[127]` (0x10034550 + 127·128·8).
  - In the engine the line is `ProcessState`'s (`crates/rb-engine/src/process.rs`), shared by
    the song's devices and presettable at export start (`set_delay_line`), like the drum counters.

---

## 9. State at export start: the main byte-exactness risk

The full trace is `docs/export_state.md` (task-004); this section keeps the summary.

Stop, locate and play reset no DSP state (stop silences the drum voices and releases the 303s). Msg
33 (full reset) is sent only *after* an export (and after the benchmark) [C], and resets only part of
the state (`docs/export_state.md` §6.4). It is **not** sent on song load: opening a song makes a new
song in the engine (top-level msg 52) and sends its data (`docs/song_load.md` §2) [C]. [C] The new
song gets **new device objects** (`0x1001D840` → song constructor `0x10011350`), so its 303, drum and
effect state starts from the constructors; only process globals carry over from earlier songs.

The realtime callback renders the active song every 1024 frames (MME), playing or stopped, so a song
runs `N` idle blocks (a multiple of 16) between its activation and the export.

| State | Effect on the export |
|---|---|
| 303 oscillator phase | [C] Free-running while idle (`+0xCA4`, not reset by note-on): depends on `N` and changes the waveform of every 303 note |
| 303 filter (y1, y2, s), amp, E, accent env, cutoff smoother | Settle while idle towards fixed points driven by the 1e-6 offsets [G: convergence not proven] |
| Leftover sub-block buffer in `sub_100139E0` | [C] Refuted: always empty at export start in the app (`docs/export_state.md` §5) |
| Mixer 16-block counter (starts at 10), effect counters | [C] With MME, a fixed phase (`10 + 16k`; effects 0 mod 16); with DirectSound (512-frame calls) it can be off by 8 |
| Mixer and effect smoothed gains | [G] Settle to their targets while idle (expected from the smoothing, not proven) |
| Delay buffer (global) and write pointer (per song) | [C] Cleared to 1e-6 on a song switch and by msg 33; the pointer advances 64 per enabled block and depends on `N` |
| Compressor env and G, PCF filter and envelope | Fresh per song, settle while idle [G] |
| Snare noise counters and BD alternation flags (global) | [C] `word_1014CBD4`, `word_1014E548`, `word_1014CBD0`, `word_1014E544`: shift by every snare or bass drum hit since the DLL loaded; an export of an 8-hit 808 snare loop moves `word_1014CBD4` by 9 (fitted to exports, `docs/export_state.md` §11.2). [G] The ninth hit is step 0, triggered by the loop wrap in the dropped tail of the last chunk: derived from the engine model (task-024), not observed, since the corpus cannot see the dropped frames (`docs/export_state.md` §5) |

**Reference-capture protocol:** `docs/export_state.md` §9 (fresh process, startup sound off, open the
song, wait a fixed time, export, quit), measured by task-010 (`docs/export_state.md` §11: byte-identical
when the 303s are silent, on two songs; otherwise the differences fit whole idle buffers `N/16`
([C] on the 145 diagnostic songs: a fit in task-010, confirmed by task-012's matches below), and no settling residual was seen between waits of 3, 10 and 30 s). It leaves `N` unknown, so model
"load, activate, `N` idle blocks, export" and fit `N`. [C] That model reproduces app exports byte for
byte: `tools/rbhost` matched three songs of one session exactly (`docs/export_state.md` §12,
task-011). It then matched the fresh app exports of all 145 diagnostic songs
(`docs/export_state.md` §13, task-012): 142 at exactly one `N` in 9040..9264, and 3 silent songs at
every `N`. [C] So, on that set and under the §11.5 protocol, a fresh export depends on the song and
`N` only. [G] It holds for other songs and protocols: not tested.

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
3. **RBM key table**: done in task-030 (`docs/rbm.md`) [C]. The source file is `RB20FUL.DAT` on
   the CD, the stride is 4, and each mod needs two 2048-byte pages of it.
4. **Table generation fidelity.** Dump the BSS tables from a live process and diff them against Rust.
   `tools/rbhost` `--dump-tables` writes them with a manifest (task-011); all are built by the
   constructors of the first new song (msg 52), so they exist from the first song on [C]:

   | Table | Address | Type, shape | Builder (guard) |
   |---|---|---|---|
   | T | 0x10054550 | f32[128] | `sub_100046B0` from the song ctor (`byte_100E0128`) |
   | PAN | 0x10034550 | f32[128][128][2] | `sub_100046B0` |
   | 303 pitch | 0x10125364 | f32[6144] | 303 ctor `sub_10023920` (`byte_10124704`) |
   | 303 FILT | 0x1012B364 | f32[8][1024][4] | `sub_100234A0`, every 303 ctor |
   | PCF LP | 0x1010C270 | f32[8][512][3] | `sub_100172C4` from the PCF ctor `sub_10016490` (`byte_1010026C`) |
   | PCF BP | 0x10100270 | f32[8][512][3] | `sub_10017574` (same guard) |
   | Dist W | 0x100D47A4 | f32[32][256] | `sub_10011210` (`byte_100D47A0`) |
   | T808 | 0x1014CBD8 | u16[128] | 808 ctor `sub_10026F70`, first instance (`word_1014CCD8`) |
   | T909 | 0x1014E54C | u16[128] | 909 ctor `sub_1002A380`, first instance (`word_1014E68C`) |
   | DT | 0x1014E64C | f32[16] | 909 ctor, the same block |
   | 303 smoother | 0x1012470C | f32[3] {b2, A, g} | static init `sub_10026BB0` (task-019) |
   | PCF smoother | 0x10100260 | f32[3] {b2, A, g} | static init `sub_10017840` (task-019) |

   Dumped values agree with the formulas of §4, §6 and §7.3 at the points checked (T[127] = 1,
   T808[64] = T909[64] = 256, T808[0] = 203, T909[0] = 188, DT[15] = 1.0, pitch[3072]·22050 = 65.406).

   [C] task-019: every table above is generated in Rust (`crates/rb-engine/src/tables`) with
   plain f64 arithmetic in the builders' operand order and `pow`/`exp`/`cos`/`sin` of the
   pure-Rust `libm` crate (pinned `=0.2.16`, decided in task-048), and is bit-identical to the
   dump (fixtures in `crates/rb-engine/tests/fixtures/tables`); none is embedded and no x87 or CRT
   emulation is needed. macOS's system libm (Rust std) gave the same tables. Across the f32 stores
   the nearest f64 value is at least 1358 f64 ulps from an f32 rounding midpoint (FILT 1625, PCF
   1358, the others far more; the unit test `store_margins` measures and asserts it). The only
   fragile entry is `T808[64]` (§7.3). The static-init list is the function
   pointer array at 0x10030010..0x10030060 (0x10003740, 0x10017840, 0x1001A920, 0x10021CD0,
   0x10026BB0). Builder constants sit in `.data1` (0x1014F000..) and `_data` (0x10156000..),
   which are in the file, unlike `.data` above 0x10034400.
5. **Export start state** (§9): traced in `docs/export_state.md` (task-004); the protocol is
   settled by experiment in task-010.
6. **Loop-end wrap inside the final pre-rendered block.** The last block can wrap to the loop start
   and chase; this only affects the leftover buffer, which the export's final
   `SafeSoundCallback(NULL,0,0)` drops [C].

## 11. Suggested Rust architecture

```
rbs        – IFF parser for RBS 4.2 (docs/RBS42.txt) → Song model
rbm        – RBM container parser and descrambler (key table from RB20FUL.DAT, docs/rbm.md)
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

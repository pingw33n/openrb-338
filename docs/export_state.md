# Engine state at export start, and how the app drives the engine

What Rebirth.exe does with ReBirth.dll in real time, around an "Export Loop as Audio File", and which
engine state reaches the first exported sample. Traced for task-004 in headless copies of
`ida/Rebirth.exe.i64` and `ida/ReBirth.dll.i64` (2026-10-26). `0x4xxxxx` is the exe, `0x10xxxxxx` the
DLL. **[C]** is checked in the disassembly; **[G]** is a guess or only partly traced. The message
names (msg 33, 35, 52, ...) are those of `docs/song_load.md` §1 and `docs/initial_investigation.md`
§5.2; "top-level" messages are the first packet byte, "song" messages are inside a type-78 packet.

## 1. Summary

- The app keeps a realtime audio stream open from start to quit, and pauses it only around an export
  and the benchmark. Every realtime callback renders the **active song** (the active MDI child),
  playing or stopped: while stopped, every device still runs every 64-frame block (§2.4).
- With the MME driver every realtime call is exactly 1024 frames (16 blocks); with DirectSound a
  multiple of 512 frames (§2.1). Song messages are applied only at the start of a call (§2.3). So
  every state change and every idle stretch is a whole number of 16-block (MME) groups.
- An export starts with an **empty leftover buffer**, always: realtime calls never leave one, and the
  export ends with a 0-frame call that discards its own (§5). The survey's "up to 63 samples rendered
  before the locate" (investigation §9) does not happen in the app.
- Opening a song creates a **new song object with new devices** (top-level msg 52, `0x1001D840`): no
  per-song state carries over from an earlier song. Msg 33 is sent only after an export and after
  the benchmark; it resets less than its name says (§6.4).
- The state at export start is therefore a deterministic function of the song, of the number `N` of
  idle blocks the active song rendered between its activation and the export (a multiple of 16), and
  of a few process-global values. [G] Most of the `N`-dependence should die out after a few seconds
  of idle (the decaying states of §7; convergence is not proven); the **303 oscillator phase never
  settles** [C] (§7.2). The protocol in §9 fixes everything else and leaves `N`
  as the one unknown, to be fitted.
- Measured (§11): under that protocol an export is byte-identical across runs when the song's 303s
  are silent (two songs); otherwise its differences fit whole idle buffers of 303 phase,
  `k = N/16` [G: a fit, not a proof], and are ±1 LSB for songs without 303 notes. Each export moves the global drum counters (9 snare hits for an
  8-hit loop, fitted; [G] the ninth is the step 0 that the dropped tail triggers).

## 2. The realtime path

### 2.1 Driver objects [C]

The app object (`sub_40CB32()`) holds the audio channel at `+0x254C` (`this[2387]`, read by
`sub_40C594`). It is created by `sub_40C863(app, cardIndex)` (callers `sub_40DEF2` at start-up and
`sub_41E959`, the Preferences dialog): the card list `dword_4E05AC` (`sub_4A2D0B` walks it) holds
`CRegisteredDirectSoundCard` and `REGISTERED_MME_SOUND_CARD` objects, and the card's vtable slot 0
creates the channel with `(44100, sub_40C7F0, sub_40C834, 0, 2)` (0x40C954), after which channel slot 1
starts it (0x40C973). `sub_40DEF2` picks the card whose name equals the preference string at
prefs `+264` (the "Remote Audio" of `docs/test_machine.md` §5), else the first card that opens, then
sets the latency index from prefs `+394` through channel slot 6 [G: slot 6 stores it at `+0x30`].
`sub_40C5DA` is an unreferenced variant with obfuscated arguments.

Channel vtable slots: 1 start, 2 stop, 3 is-running, 4 refill (timer), 8 play-ahead in ms.

**MME (`MME_CHANNEL`, vtable 0x4BE764, ctor `sub_4A5C17`).**
- Fields: `+4` frames per buffer = **1024** (`sub_4A2870(.., 1024, ..)` at 0x4A5C5D), `+8` 44100,
  `+0xC`/`+0x10` the two callbacks, `+0x18` 2 channels, `+0x24` HWAVEOUT, `+0x28` 16 WAVEHDRs of
  1024 frames × 4 bytes (`sub_4A58B7`), `+0x30` latency index (14 after construction),
  `+0x34` underrun counter.
- Buffers in flight: `word_4BE728[index]` = 2, 3, … 16 for index 0..14. Play-ahead
  `sub_4A5539` = `1024000*count/44100` ms: the test machine's **371 ms is 16 buffers** (index 14).
- Start `sub_4A5EA7`: waveOutPause, fills and writes `count` buffers at once (each through
  `sub_4A5A55`), waveOutRestart, then creates a periodic worker `sub_48D6EA(sub_4A6033, this,
  0xFA000 / (3 * rate))` = `1024000 / 132300` in unsigned integer division = 7 ms (call at 0x4A6003;
  the exact quotient is 7.74) that calls slot 4 [G: a thread or timer object, not read through].
- Refill `sub_4A5B68` (slot 4): while fewer than `count` buffers are queued, fill the next one (index
  `+0x2C & 15`) and write it. `sub_4A5A55` calls callback 1 then callback 2 with `(buf, 1024, 0)`,
  then waveOutWrite.
- Stop `sub_4A5E38` (slot 2): clears the running flag `+0x20`, waveOutReset, spins until every header
  is back (`sub_4A5AF9`), deletes the worker.

**DirectSound (`CDirectSoundChannel`, vtable 0x4BE69C).** Refill `sub_4A42B4` → `sub_4A3F79` /
`sub_4A4040`: a 16384-frame ring (`& 0x3FFF`, 0x4A4030); the frames to write are rounded down to a
multiple of 512 (`*a3 -= *a3 & 0x1FF`, 0x4A4028); a write that wraps is split into two callback pairs,
each a multiple of 512 frames. Not used on the test machine (MME).

### 2.2 The two callbacks [C]

- `sub_40C7F0(buf, n, x)`: `SyncSoundCallback(buf, n, x)`, then `SafeSoundCallback(buf, n, x)` unless
  app byte `+0x500` is set.
- `sub_40C834(buf, n, x)`: `SafeSoundCallback(buf, n, x)` if app byte `+0x500` is set.

So each buffer gets exactly one `SafeSoundCallback`. [G] `+0x500` (written at 0x40C588, 0x40D285 and
in `sub_43EB64`, `sub_43ED76`, `sub_4405B5`) only changes the order relative to the sync callback.

`SyncSoundCallback` (0x1001ADB0) takes the engine mutex, and `sub_1001C474` stores
`(timeGetTime-like sub_1000E080(), trunc(song position))` into the transport object
(`sub_10007E30`). It renders nothing and changes no audio state.

### 2.3 `SafeSoundCallback` (0x1001AE40) [C]

The engine object is `[dword_1011D0B0 + 0x370]` (0x430 bytes, `EngineOpen` 0x1001B0E0 →
`sub_1001C7B0`); the transport object is `dword_1011D0B4` (0x174 bytes, `sub_1001BB80`).

1. `WaitForSingleObject(engine[0], 0)` (`sub_10009454`): a try-lock. On failure the buffer is zeroed
   (0x1001AEE5) and step 2–3 are skipped.
2. Drain the panel's messages, `sub_1001CC40(engine, n)` (below).
3. If there is an active song, `[transport + 0x16C]`, render it: `sub_100139E0(song, buf, n)`;
   otherwise zero the buffer. Release the mutex.
4. Mix the preview player into the int16 buffer (§8).

**The drain `sub_1001CC40` [C].**
```
budget   = (n*1000/44100) / 3            // integer ms: 15 for n = 2048, 7 for 1024, 0 for 0
deadline = timeGetTime() + budget        // sub_1000E070
forward every queued reply from the engine's reply queue (+0x41C) to the port
do {
    if port.read(...) != 12: break       // no message
    dispatch it (sub_1001CDF0)
} while timeGetTime() <= deadline        // signed compare, 0x1001CDB1
```
At least one message is processed per call when one is waiting; further messages wait for the next
call if the clock passed the deadline. Neither the exe nor the DLL calls `timeBeginPeriod`, so the
clock's step is the system default [G: its size on the test machine is not measured].

**`sub_100139E0(song, buf, n)` [C]** (full description of the conversion: investigation §3):
- `n` is clamped to 16384 frames (0x100139ED).
- It takes the song's own mutex at `song+0x154`, also as a try-lock; on failure no block is rendered
  and the int16 output is converted from the float buffers of the previous call
  (`0x100E012C`/`0x100F012C`). Same-thread callers succeed because Win32 mutexes are recursive. [G]
  Other holders: `sub_10011D84`, `sub_10012940`, `sub_10013240`, `sub_1001DC64` (song delete).
- Leftover logic in §5.

### 2.4 What runs while stopped [C]

`sub_10013444`, the 64-frame block, has no "playing" test: every block runs the 16-block mixer update
when `(song[+0x188] & 15) == 0`, the sequencer tick `sub_1001EF20` (which does nothing while stopped),
**every channel device's audio** (vtable slot 14, 0x10013580), the inserts, the send effects and the
master. So the 303s, drum voices, delay, compressor, distortion, PCF and all smoothers keep running
whenever the song is active and a callback renders it, whether the transport plays or not. A song
that is not active is never rendered: its state is frozen.

### 2.5 Pumping without audio [C]

The reply wait `sub_40AC44` (used by `sub_40ABE5`) loops on the port's read with a 10000 ms timeout;
between tries it calls `EngineIdle` (`sub_40A001`, when the DLL is loaded), and, **only when there is
no audio channel**, `sub_408DF5`: `SyncSoundCallback(v, 0, 0)` and `SafeSoundCallback(v, 0, 0)`. A
0-frame call drains messages and renders nothing (unless a leftover exists, §5). With a channel, the
realtime callbacks do the draining, so a reply wait while the channel is stopped would hang; the
export sends no query while the channel is stopped.

There is no way to run the app's export without a channel: `sub_427988` calls the channel's slot 2 at
0x427BD9 without a null test.

### 2.6 `EngineIdle` and the queued messages [C]

The dispatcher `sub_1001CDF0` (table at 0x10153164, indexed by the type byte) handles most top-level
types inline, in the audio thread. Types 52, 54, 55, 57, 58 and 63 (and 56 when engine byte `+0x374`
is set, 0x1001CF5D) instead go through
`sub_1001D694`, which copies the packet into the engine's queue `+0x410` and signals `+0x414`;
`EngineIdle` (0x1001B070 → `sub_1001CB00`, UI thread) pops one packet per call and dispatches it with
`sub_1001D6D4` (table 0x101531F8, by type): 52 → `sub_1001D840` (new song), 54 →
`sub_1001DC64` (delete song, reply 'B'), 57 → `sub_1001DB60` (clear the preview, `sub_1001A6B0`,
reply 'D'), 58 → `sub_1001DC14` (start the preview, §8). Replies go to the queue `+0x41C`, which the
next drain forwards to the port. Top-level 53 (activate a song) is handled inline: 0x1001CF13 →
`sub_1001D7F4`.

### 2.7 The shared-memory port (for rbhost) [C]

`sub_1001C7B0` creates `CReBirthDLLPort` (`sub_1001B980`, vtable 0x10033C44) at `engine+4`: a writer
on `"ReBirth DLL->Panel"` and a reader on `"ReBirth Panel->DLL"`. Each side (`sub_100179E0`) is:
- a semaphore or mutex object named `"<name>_SEMA"` (`sub_100091B0`) [G: which kind],
- an auto-reset, initially unsignaled event `"<name>_EVENT"` (`CreateEventA(0, FALSE, FALSE, ..)`),
- a pagefile mapping `"<name>"` of 0x7D10 bytes (`CreateFileMappingA(-1, 0, PAGE_READWRITE, 0,
  0x7D10, name)`), mapped whole; if it did not exist yet it is zeroed and dword 1 is set to 32000.

[G] The ring layout inside the mapping (read `sub_100095E0`/`sub_100096D0` family, port slots 1/2)
is not traced; rbhost must read it from those functions.

## 3. Songs: create, activate, delete [C]

**New song (top-level 52, `sub_1001D840`).** Takes the first free slot of `engine+0x14 + 4*slot`
(255 slots), `operator new(0x1910)` and the song constructor `sub_10011350`, which builds new device
objects: the mixer device (0xA0), TB-303 ×2 (0xD04, `sub_10023920`), TR-808 (0x1910, `sub_10026F70`),
TR-909 (0x1920, `sub_1002A380`), delay (0x74, `sub_1000F5A0`), distortion (0x60, `sub_10010700`), PCF
(0x8C, `sub_10016490`) and compressor (0x6C, `sub_1000E990`). It then calls every device's slots 7 and
8, sets the tempo to 140000 (`sub_1001FCD0`), locates to 0 (`sub_1001F980`) and runs one mixer update
(`sub_10012C14`). Replies 'A' with the slot. So **every load, New and app start gives fresh per-song
state**; only the process globals of §7.5 survive. Constructor values that matter here:
leftover position `song+0x208` = 0 (0x10011406), the 18 leftover blocks zeroed, block counter
`song+0x188` = 10 (0x1001145A; the loop that follows clears `+0x18C..+0x1C0`, not `+0x188`), master
gain current and target = 1.0 (`+0x180`, `+0x17C`), channel gains 1e-6, effect counters 0 (`CEffect`
base `sub_10010420`), 303 phase and filter state 0 (§7.2).

**Activate (top-level 53 `{slot}` or `{0xFF}` for none, `sub_1001BDA4`).** Sets the pending song
`transport+0x170`; if the current song `transport+0x16C` is playing, nothing else happens [G: the
pending switch is then made elsewhere]. Otherwise the old song's devices get slot 6 (`sub_10013D40`:
303 note-off, drum voices silenced, **the global delay buffer cleared**, §6) and `sub_1001BD24` makes
the new song current. Sent by `sub_40D163` (app `+0x2544` = current document) from the song window
constructor (`sub_42DA72`, 0x42E5AE: after the load), from MDI activation (`sub_42D7BE`), from the
document destructor (`sub_426E4A`, deactivates before deleting) and from the benchmark (0x402024).
With no active song, `SafeSoundCallback` outputs silence (plus the preview).

**Delete (top-level 54, `sub_1001DC64`)**, sent by `sub_426E4A` with a reply wait.

## 4. The export, step by step (`sub_427988`, from `sub_4276F9`) [C]

| Address | Action | Processed |
|---|---|---|
| 0x427A02 | song msg 35 `{1,0}` stop | by a realtime drain |
| 0x427A35, 0x427ACB, 0x427B5E | msg 20 queries tempo (4), loop start (12), loop end (16), each with a reply wait | realtime drains; the waits guarantee stop and the queries were applied |
| 0x427BD9 | channel slot 2: **realtime stopped** (MME: reset and wait for all buffers) | — |
| 0x427C1D | msg 35 `{8, loopStart}` locate | queued |
| 0x427C45 | open the output file (`sub_484206`) | — |
| 0x427C83 | msg 35 `{1,1}` play | queued |
| 0x427C98.. | progress box, then `sub_427EA7` (AIFF, `byte_4E01A0`) or `sub_4280D5` (WAV, `byte_4E0448`) | the first `SafeSoundCallback(buf, 2048, 0)` drains locate then play, then renders |
| 0x427DA6 | msg 35 `{1,0}` stop | queued |
| 0x427DC9 | msg 33 full reset (no payload, `sub_40B194`) | queued |
| 0x427DD6 | `SafeSoundCallback(NULL, 0, 0)` | drains at least the stop; 33 too unless the clock ticks in between, else at the first realtime call |
| 0x427DE7 | channel slot 1: realtime restarted (MME: 16 buffers rendered at once) | — |

The render loop (`sub_4280D5`, `sub_427EA7`) is the double-precision chunking of investigation §3,
writing `4*n` bytes per call. It sends nothing; the progress update `sub_474D14` [G: whether it pumps
window messages] and the cancel test `sub_474EB3` (a flag) are the only other calls.

**Messages at the first chunk.** Both locate and play wait in the port. They are applied in the same
call when the second dispatch starts no later than `t0 + 15 ms` (§2.3); a locate (one chase) takes far
less, so this fails only if the millisecond clock steps past the deadline between them [G: rare]. If
it does, play is applied one chunk late and the export starts with 2048 frames rendered stopped.

**No realtime callback runs during the export**: the channel is stopped first. [G] The MME worker is
deleted in `sub_4A5E38` after the buffers are back; a refill already running at that moment could
overlap the first export call (both try-lock the engine mutex; the loser outputs silence or skips).

**The benchmark** (`sub_401EE0`, from `sub_4026DB` / `sub_402937`) loads the bundled song into a new
song, activates it, and runs the same sequence (stop, queries, channel stop, locate, play, the same
chunking into a scratch buffer, stop, 33, `SafeSoundCallback(buf,0,0)`, channel start), then deletes
the song.

## 5. The leftover buffer [C]

`song+0x208` (`L`) is the position in the partial block kept at `song+0x20C + 256*ch` (18 channels of
64 f32; channels 0 and 1 are the output L/R, `sub_100139E0` sets only `byte_100DF754[0..1]`).
`L > 0` means samples `L..63` of that block are still to be output.

```
if L > 0:   copy samples L..63 to the output start (64-L samples, even if n is smaller), refill them with 1e-6
            rem = n - (64-L)
else:       rem = n
while rem > 63: render a block into the output; rem -= 64
if rem > 0: render a block into the leftover block, copy its first rem samples out
L = rem     // stored at 0x10013BBA, may be <= 0
```

- Realtime calls are multiples of 64 frames (§2.1), so they never create a leftover and keep `L` at 0.
  A negative `L` is treated like 0 (the `L > 0` test at 0x10013AE0 fails, so `rem = n`), and the next
  call stores its own `rem`: 0 for any multiple of 64, including `n = 0`.
- The final export chunk leaves `0 < L < 64` when its length is not a multiple of 64; the
  `SafeSoundCallback(NULL, 0, 0)` that follows (0x427DD6) runs the `L > 0` branch with `n = 0`, so `L`
  becomes `-(64-L)` and those samples are dropped. Msg 33 does not touch `L` (it only zeroes the
  buffers).
- A new song starts with `L = 0`.

So the first sample of every export is sample 0 of a block rendered after the locate and play. (The
loop-end wrap inside the export's last partial block, investigation §10 item 6, affects the
exported samples only through the dropped ones; [G] but the notes it triggers at the loop start
seem to run, so a drum hit on the loop's first step moves the global drum counters of §7.3 once more
per export: inferred from the +9 snare-counter fit of §11.2, not traced.)

## 6. Reset points [C]

### 6.1 Device vtable slots used for resets

| Class | slot 4 (stop) | slot 5 (msg 33) | slot 6 (note-off / silence) |
|---|---|---|---|
| CTB303Device 0x10033FD4 | `0x100106C0` → slot 6 | `0x10026CC0` reset, then slot 6 | `0x10025630`: gate `+0xCA1` = 0, `sub_10023C74` sets the release constants `+0xCC8/+0xCCC`, `+0xCD0` = 0 |
| CTR808Device 0x10034154 | → slot 6 | `0x1000F4F0` → slot 6 | `0x1002A2D0`: every voice's slot 1 |
| CTR909Device 0x10034294 | → slot 6 | → slot 6 | `0x1002DD40`: every voice's slot 1 |
| CDelayEffect 0x10032CB4 | `nullsub_1` 0x10012C00 | → slot 6 | `0x100100E0`: fill the global buffer `0x10054794` (131072 f32) with 1e-6, `byte_10054790` = 1 |
| CDistInsert, CPCFInsert, CCompInsert, CMixerDevice | → slot 6 | → slot 6 | `nullsub_1` |

Drum voice slot 1 is `0x1001EB30` for all four voice classes (vtables 0x10033D54/64/74/84): it clears
the active byte `+8` and the gain `+0x18`. Voice gains and the FILTER_VOICE state are not cleared.

### 6.2 Stop (song msg 35 `{1,0}`, `sub_1001E100`)

`sub_100218A0`: clears the play flag, calls slot 4 on the song's devices (`sub_10021840`), and in
song mode chases all tracks at the current position; then (unless `transport+0x160` is set) locates
to the start of the current bar. So stop silences the drum voices and releases the 303s, and leaves
the delay, the effects and the mixer alone.

### 6.3 Play, locate

Play calls each device's slot 3 (investigation §5.1); locate (`sub_1001F984`) chases and relocates
the devices. Neither resets DSP state. [G] Device slot 3 and the relocate were not read through for
this task beyond investigation §5.1.

### 6.4 Msg 33 (`sub_10014140`, song msg 33 → 0x10012546)

Resets: master current gain `song+0x180` = **0** (the constructor's value is 1.0, so the master
fades in again over the next blocks); meters `+0`, `+4` = 0; the three 64-sample buffers at
`song+0x160C` = 1e-6; the 18 leftover blocks = 0; then slot 5 on every device: the 303 reset
(§7.2), the drum voices silenced, the delay buffer cleared. Does **not** reset: `L`, the block counter,
the channel gains, the master target, the effects' counters, gains and envelopes, the delay write
pointer, the 303 filter coefficients `+0xCF4..+0xD00`, any process global.

## 7. State that reaches the export

"Load" below means the song constructor plus the load messages; "activation" is §3; `N` is the number
of blocks the song rendered while active before the export (with MME a multiple of 16). Classes:
**fixed** (a function of the song file only), **settles** (depends on `N` but converges to a fixed
point within seconds of idle, given no change), **free** (depends on `N` forever), **global**
(process-wide, depends on what earlier songs did).

### 7.1 Song object (`sub_100139E0` / `sub_10013444` `this`, 0x1910 bytes)

| State | Address | Init / reset | Class | Effect |
|---|---|---|---|---|
| Leftover position and block | `+0x208`, `+0x20C` | 0 at load; never non-empty at export start (§5) | fixed | none |
| Block counter (mixer update when `& 15 == 0`) | `+0x188` | 10 at load; not reset | fixed with MME: `10 + 16k`, so the first mixer update of the export is in block 6; with DirectSound `10` or `2 mod 16` | phase of every 16-block parameter update |
| Channel gains, targets `[41],[42]+11c`, currents `[43],[44]+11c` | `+0xA4 + 44c` | 1e-6 at load; per-block smoothing `0.93/0.07` | [G] settles (a contraction with f32 rounding; the fixed point is not proven) | starting gains of the export; a target that changes at the locate (song-mode chase) ramps from the idle value |
| Master target, current | `+0x17C`, `+0x180` | 1.0, 1.0 at load; current = 0 after msg 33 | [G] settles | master level of the first blocks |
| Channel peaks, meters | `+0xB8 + 44c`, `+0`, `+4` | — | — | display only (`song+0xA0` index) |
| Scratch buses | `+0x140C`, `+0x150C`, `+0x160C` ×3 | 1e-6 at load and msg 33 | fixed | rewritten every block [G: roles] |
| Sequencer, globals | `+0x8`, `+0x158` | load messages | fixed | position set by the export's locate |

### 7.2 TB-303 ×2 (`CTB303Device`, 0xD04 bytes; ctor `0x10023920`, reset `0x10026CC0`, audio 2.0 sound `0x100248C4`, Vintage inline in `0x10023F10`)

Fields the 2.0 engine reads and stores at block end (0x10025319..0x10025394): `+0xCA4` (oscillator
phase: `+= table[pitch]` per sample, `-= 2` when above 1.0), `+0xCA8` (amp, `+= 1e-6` per block),
`+0xCBC`, `+0xCC0`, `+0xCC4`, `+0xCD4` (envelope, `+= 1e-6` per block), `+0xCDC`, `+0xCE0`,
`+0xCE4`, `+0xCE8` (filter state), `+0xCF4..+0xD00` (filter coefficients, ramped from their old
value every block), `+0xCB4` (smoothed pitch `(11*old + target)/12`, 0x10024940). The target pitch
`+0xCB0` is set by note-on (0x10023EBF); the coefficients are re-initialized without ramp when
`+0xCA0` is set (note-on with the gate off, 0x10023E5B).

| State | Init / reset | Class | Effect |
|---|---|---|---|
| Oscillator phase `+0xCA4` | 0 at load (ctor store 0x10023A6B) and msg 33 (0x10026D40); note-on does **not** reset it | **free** | the waveform of every 303 note of the export; also the tiny idle output (amp floor × oscillator) mixed into the master before the first note |
| Idle pitch `+0xCB0/+0xCB4` | 0 at load (0x100239ED, 0x100239F3) and msg 33 (0x10026CD4, 0x10026CDA); stays at the last note after notes | fixed while no note plays before the export | with no earlier note the phase runs at `table[0]` = 130.8128/16/44100 = 1.8538e-4 per sample (`0x10125364`, built at 0x10023B4D: `110·2^(1/4)/(16·44100)·2^(i/768)`): 0.190 per 1024-frame buffer |
| Amp `+0xCA8`, envelope `+0xCD4`, `+0xCBC`, `+0xCC0`/`+0xCC4` (0.5), filter state `+0xCDC..+0xCE8`, `+0xCAC`, `+0xCB8`, `+0xCC8..+0xCD8`, `+0xCEC`, `+0xCF0` [G: which of these is the accent envelope and which the cutoff smoother is not mapped] | 0 (`+0xCC0`/`+0xCC4` 0.5) at load (0x10023920) and msg 33 (stores 0x10026CE6..0x10026D51) | [G] settles (they decay towards fixed points driven by the 1e-6 offsets) [G: convergence not proven] | first note's attack |
| Filter coefficients `+0xCF4..+0xD00` | 0 at load (0x10023A2F..0x10023A41); not reset by msg 33 | [G] settles (ramped to the current target every block; that the ramp ends exactly on the target is not checked) | — |
| Gate, slide state `+0xCA1`, `+0xC96`, `+0xC88..+0xC93` | 0 at load, msg 33, stop | fixed | — |

`dword_10124708` (incremented per 303 note-on, wraps at 19, 0x10023EDD) is never read: no effect.
[G] The Vintage engine's own fields (investigation §6.2) were not mapped to offsets; its oscillator
phase behaves the same way (it is free-running in the inline loop).

### 7.3 Drums

| State | Address | Init / reset | Class | Effect |
|---|---|---|---|---|
| Voice active flag, gain `g` | voice `+8`, `+0x18` (FILTER_VOICE process 0x1001E4D0) | cleared by stop, msg 33 and the activation switch (0x1001EB30) | fixed (no hit while idle; an inactive voice is not processed) | — |
| Voice sample pointer, samples left, hold, multiplier `m`, FILTER coefficient `c`, FILTER state `y` | voice `+0xC`, `+0x10`, `+0x14`, `+0x1C`, `+0x20`, `+0x24` | never cleared; [G] whether the voice constructors and the trigger (`sub_1001E5C0`, `sub_1001E820`) set `y` is not read | fixed while idle (frozen while inactive) | `y` feeds the first sample of the next FILTER_VOICE hit [G] |
| 808 snare noise counter | `word_1014CBD4` (`sub_1002A1E4`, pre-increment `& 31`) | BSS: 0 at DLL load; never reset | **global** | which noise sample each snare hit uses: `c = (c+1) & 31` per hit (0x1002A25D..0x1002A27D), then sample slot `word_100340DC[word_1003409C[c]]`; `word_1003409C` is 32 words with values 0..3 (`0,1,2,3,1,0,3,1,0,3,2,1,0,2,3,1,0,2,3,2,0,1,2,0,3,1,2,0,1,3,2,1`) and `word_100340DC[0..3]` = 6, 7, 8, 9, so four noise samples in a 32-hit sequence (both tables are 16-bit words, read as `mov bx, word_1003409C[ecx*2]` at 0x1002A28C and `mov si, word_100340DC[ebx*2]` at 0x1002A29D; values read from the database's bytes for task-010; the model fits the exports, §11.3) |
| 909 snare noise counter | `word_1014E548` (`sub_1002DC54`) | same | **global** | same, SEQ909 |
| 808 bass-drum voice toggle | `word_1014CBD0` (0x1002A106 / 0x1002A12A) | same | **global** | which BD voice a hit uses and which one fades |
| 909 bass-drum voice toggle | `word_1014E544` (0x1002DB82 / 0x1002DC3B) | same | **global** | same |

They change only on hits, so they depend on every snare and bass-drum hit since the process started
(earlier exports, earlier playback, the benchmark), not on `N`.

### 7.4 Effects

| State | Address | Init / reset | Class | Effect |
|---|---|---|---|---|
| Delay buffer | `flt_10054794`, 131072 f32, **global** | BSS 0 at DLL load; 1e-6 on msg 33, on a song switch (old song's slot 6), and once when a delay runs disabled (`byte_10054790` gate, 0x1000FAD8) | [G] settles towards the idle send signal, which carries the 303's idle output (not proven) | repeats of earlier audio when the export follows playback without a switch or reset; otherwise values near 1e-6 that can flip the int16 truncation of a sample |
| Delay write pointer, length | delay `+0x6C`, `+0x68` | 0 at load; never reset | **free**: `+64` per enabled block, `64N mod 131072` | which buffer cells the export reads first |
| Delay update counter | delay `+0x30` (word) | 0 at load (`sub_10010420`); counts enabled blocks | fixed with MME (multiple of 16) | update phase |
| Delay pan gains (smoothed 0.9/0.1) | delay `+0x54..+0x60` | 0 at load (`sub_1000F5A0`) | [G] settles | — |
| Distortion, PCF, compressor counters | effect `+0x30` | 0 at load | fixed with MME | update phase |
| Distortion gains: current in/out `+0x44`/`+0x48`, targets `+0x54`/`+0x58` (smoothing at 0x10010A50) | dist object | load values [G: ctor `sub_10010700` not read]; never reset | [G] settles | first blocks of the export |
| Compressor: env `+0x50`, gain G `+0x5C`, hold counter `+0x64` (word, reload `+0x66`) (mono 0x1000EC70) | comp object | load values [G: ctor `sub_1000E990` not read]; never reset | [G] settles | first blocks of the export |
| PCF: LP state `+0x50`/`+0x54`, BP state `+0x58`/`+0x5C`, trigger level E `+0x68`, cutoff base `+0x6C` and its smoother `+0x80`/`+0x84`, envelope `+0x70`, last step `+0x74`, attack `r` `+0x78`, decay `+0x7C` (0x100166D0) | PCF object | load values [G: ctor `sub_10016490` not read]; never reset | [G] settles; the last step is recomputed from the song position every block | first blocks of the export |

### 7.5 Outside the songs

| State | Where | Effect |
|---|---|---|
| Preview player sample, position | engine `+0x428`, `+0x42C` | mixed into the int16 output while active (§8) |
| Float output buffers | `0x100E012C`, `0x100F012C` | rewritten every call; stale only after a failed song-mutex try-lock (§2.3) |
| Lookup tables (FILT, pitch, T/PAN, PCF, dist, drum) | BSS, built once at init | fixed per process; investigation §10 item 4 |

## 8. The preview player and the startup sound [C]

`SafeSoundCallback` mixes, after the song, the mono int16 sample at engine `+0x428` from position
`+0x42C` (0x1001AF15..0x1001B037): for each of `min(len - pos, n)` frames, `h = s/2` rounded toward
zero (`add 80000000h; adc 80000000h; sar 1`, 0x1001AF7E), and `L = sat16(L + h)`, `R = sat16(R + h)`.
At the end of the sample `+0x428` is cleared. It runs in every `SafeSoundCallback`, export chunks
included, and advances only with the frames those calls render.

It is started by top-level 58 `{u16 mod}` (`sub_1001DC14`: sample = mod record `+184` of
`sub_1001A860(mod)`, position 0) and cleared by 57. The exe sends 58 through `sub_426C67` when the mod
record's word `+1088` is 1 (0x426C6E) and app byte `+0x533` ("Play Mod Startup Sound") is set: the
byte is tested by `sub_4098C3` (called at 0x426C85) and written by `sub_4098AA` (0x4098B7), which
`sub_40DEF2` calls with prefs `+536` (0x40E2F8). Senders: from the song load `sub_425C4D` (0x425FAB, 0x426400), `sub_426CA8`,
`sub_426FB3`, `sub_41A0C5`; and through `sub_40ABB5` from `sub_42A799` / `sub_42A80E` [G: the Mods
menu]. [G] Whether the built-in "Standard ReBirth" record has `+1088 = 1`, and the sample's length,
are not known. Task-010 measured that with the preference on, an export started 0.04 s after the
open returned (about 1.5 s after the load) is identical to one with it off (§11.4, one run per
variant): in those runs the startup sound did not reach the export. Nothing else is mixed into the output.

## 9. Proposed capture protocol

The aim: every state of §7 fixed except `N`, which is then recovered.

1. **Machine state.** The preferences file restored from `C:\re\rb\prf_before.prf` with the app
   closed (`docs/test_machine.md` §5): driver `Remote Audio`, MME, latency index 14 (16 × 1024
   frames). Keep MME: DirectSound's 512-frame granularity would make the mixer-counter phase depend
   on timing (§7.1).
2. **Startup sound off.** Clear "Play Mod Startup Sound" in Preferences once and keep that `.prf`
   (decision for task-010); otherwise wait longer than the sample after every load.
3. **Fresh process per export.** Start the app, dismiss the MIDI box, touch nothing (no Play, no
   knob, no pattern click: any drum hit moves the global counters of §7.3).
4. **Open the song** (Ctrl+O, full path). Load creates a new song and activates it; activating it
   switches away from `Untitled song`, which clears the delay buffer to 1e-6.
5. **Wait a fixed time `T`** (proposal: 10 s), so the settling states of §7 reach their fixed points.
   Do not touch the song.
6. **Export** (File, Export Loop as Audio File, full path), wait for the file, quit the app.

What the protocol leaves uncontrolled:
- **`N`**, the idle blocks between activation and the export: `T·44100/64` rounded to a multiple of 16
  plus the load and dialog times; it sets the 303 phase (§7.2) and the delay write pointer (§7.4).
  Songs without 303 notes and with the delay off depend on it only through the 303 idle output, which
  is below one LSB but can flip truncations. **Recover `N` by fitting:** render the song with the
  model "load, activate, `N` idle blocks, export sequence" (rbhost, task-011, or the Rust engine) for
  `N = 16k` around the expected value and keep the `k` that matches; the 303 phase cycles every ~10.5
  buffers (§7.2), so the search is short. rbhost should take `N` as an option.
- **The two-message drain at the first chunk** (§4): a late play shifts the export by 2048 frames;
  rare, and easy to detect.
- **The DLL heap**: device fields that the constructors do not set were not audited [G]; the
  constructors read in this task set every field listed in §7.
- **Unaudited settling**: that the 303 envelopes, compressor and PCF states reach a fixed point (or a
  cycle of even length in blocks) during `T` is expected from their decay, not proven. Task-010
  checks it with two exports at different `T`.

Exporting twice in one process is worse: the second export starts after the first one's msg 33
(303 reset, delay cleared, master at 0) and after the global drum counters moved.

## 10. Open items

- [G] The MME worker object `sub_48D6EA` (thread or timer) and whether a refill can overlap the first
  export call.
- [G] The port's ring layout (§2.7) and the `_SEMA` object type.
- [G] The pending song switch while playing (`transport+0x170`).
- [G] Field-level reset audit of the 808/909 devices, PCF, compressor and distortion constructors.
- [G] The startup sample of the standard mod and its length (it did not reach any export, §11.4).
- [G] The constant phase offset of 0.0114 note periods between first and later exports in one
  process (§11.2).

## 11. Results: determinism of the export per protocol (task-010)

Measured on 2026-10-26 with `tools/rb_export.py` (all runs with `--restore-prf`, so MME and 16 ×
1024-frame buffers) and `tools/rbcmp.py`, on five diagnostic songs (`tests/songs/diag/songs/`):
`silence`, `303_accent0`, `808_snare`, `delay_straight_fb64` (808 clap into the delay, feedback 64,
4 bars) and `comp_ch_a100_t10` (303 #1 through the channel compressor). Each of P1–P4 was run three
times, the extra variants once each unless noted (32 `rb_export.py` runs in all); the hashes and
every pairwise comparison are in task-010's notes. The scripts, the variant songs and the exact
commands are in `tools/experiments/task010/` (README.md); the WAVs are not kept.

### 11.1 Results per protocol

| Protocol | silence | 808_snare, delay (no 303 notes) | 303_accent0, comp (303 notes) |
|---|---|---|---|
| P1: fresh process, open, wait 10 s, export (§9; startup sound off) | identical ×3 | ±1 LSB on 0.03–1.6 % of the samples, from the first snare hit or the first echo on | identical or a large difference (max \|d\| 4881, 21735): 2 distinct results in 3 runs each |
| P2: fresh process, two exports in a row | identical ×6 | first exports: ±1 LSB variants like P1 (808: 2 of 3 equal, and equal to P1 r1; delay: 2 of 3 equal); second exports differ from the first by the snare's noise (max 4940) and among themselves by ±1 LSB | first exports: 303_accent0 identical ×3 but on another `k` than any P1 run (§11.2), comp 2 of 3 equal (and equal to P1 r1); second exports differ from the first; among themselves all differ (303_accent0), 2 of 3 equal (comp) |
| P3: fresh process, play 3 s, stop, export | identical ×3 | ±1 LSB among themselves; 808: noise of every hit differs from P1 (max 4940) | all differ (303_accent0: 2 of 3 equal) |
| P4: load and export `drums_kit` first, then open the song in the same process | identical ×3 | ±1 LSB (identical in 3 of 3 for 808, 2 of 3 for delay); 808: differs from P1 by the snare's noise | identical ×3 (303_accent0, same as 2 P1 runs), 2 of 3 (comp) |
| P1 with both 303s silenced (device and channel off, `808_snare`, `delay_straight_fb64`) | — | **identical ×3** | — |
| P1 with T = 3 s and T = 30 s (one run each) | identical | ±1 LSB from P1 | phase-shifted (as P1) |
| P1 with the startup sound on, T = 0 and 10 s (one run each) | identical | ±1 LSB from P1, i.e. another discrete `k` variant (T = 10 s: identical to P2 r3's first export, `a465d579b148`) | — |

### 11.2 Attribution

- **The 303 oscillator phase (`+0xCA4`, §7.2), i.e. `N`.** Within one note, the export of
  `303_accent0` matches another run's export shifted in time by part of a period (`shift.py`, frames
  1000..4000, left channel: rms difference 1214 at no shift and 101 at the best shift of −194 samples
  for P1 r1 against P2 r1's first export; 989 and 34 at −65 samples for P1 r1 against P1 r3, with a
  signal rms of 809): a phase offset. Expressed in note periods (the note at frames 1000..4000 has a
  period of 680.80 samples) the offsets between fresh-process exports are +0.0951 and +0.2850 periods.
  **Fitted** to whole idle buffers of 1024 frames at the idle increment
  `1024 · 1.8538e-4 / 2 = 0.09491` periods, they are `k` = 1 and 3 buffers with misfits of 0.0001 and
  0.0002 periods (`phase.py`). [G] So a fresh export is a function of the song and of `k = N/16`
  only; the fit is consistent with that, it does not prove it. Across the six fresh first exports of
  `303_accent0` (P1 ×3, P2 ×3) `k` took the values 0, 0, +1 (P1) and +3, +3, +3 (P2), relative to
  P1 r1; P4's three runs had `k` = 0.
  [G, unexplained] Why all three P2 first exports landed on the same `k` (+3), which no P1 run hit:
  the runner's steps before the first export are the same for P1 and P2 (`--exports` only changes
  what follows it); the runs were not timed precisely enough to tell.
  The three second exports of P2 differ among themselves by whole buffers too (`k` = −11, −12, −10,
  steps of 1 and 2), but sit 0.0114 periods off the first-export grid [G: not attributed; a
  candidate is the 303 filter coefficients `+0xCF4..+0xD00`, which msg 33 does not reset (§6.4,
  `sub_10026CC0`) and a fresh song has at 0]. After play (P3) the idle pitch is the last note's, so the
  grid differs and is not checked.
- **The 303 idle output.** With no 303 notes the residual is ±1 LSB, and it disappears when both
  303s are silenced (§11.1, identical ×3 for both songs). [G] The mechanism: the idle 303 signal (amp
  floor × free-running oscillator) is too small to show on its own (`silence` is all zeros) but
  flips the truncation of louder samples, and its phase is set by `N`; its amplitude was not
  measured. The ±1 LSB results of a song are discrete variants (the same hash recurs across runs and
  protocols, e.g. `808_snare` `c51aafa6ef7b` in P1 r1 and P2 r1/r2, `a465d579b148` in P2 r3 and the
  startup-sound T = 10 s run), as expected from a dependence on `k`. [G] The delay write pointer
  (§7.4) had no visible effect: the 303-silenced delay song was identical in three runs, but `N` was
  not recorded for those runs, so that they had different `N` is only likely (the other songs' runs
  show `k` varying).
- **The 808 snare noise counter (`word_1014CBD4`, §7.3).** The 40 snare hits of five exports in one
  process (`808_snare`, `--exports 5`) were clustered by equality (within 1 LSB, first 8000 frames
  after the hit): a hit sounds like the same-position hit of another export exactly when the model
  `noise = word_1003409C[(s + h + 1) & 31]` gives the same sample, with the starting counters `s` =
  0, 9, 18, 27, 4 (the unique fit, `snare.py chain`). So the counter advances by **9 per export of
  an 8-hit loop** (a fit to one five-export run). [G] The ninth hit is the step-0 hit triggered by the
  loop-end wrap in the dropped tail of the last chunk (§5): that explains the 9, but the trigger was
  not observed directly or traced in the sequencer.
  After a 3 s play (P3) the fit gives `s = 7` in all three runs; after a `drums_kit` export (4 snare
  hits, none on step 0) `s = 4`. Within one export the hits differ among themselves even with the
  same noise sample (their offsets within the 64-frame block differ), so only same-position hits
  compare.
- **Second export (P2)**: the msg 33 reset (303 phase to 0, delay cleared, §6.4), a new `k`, and the
  drum counters moved by the first export (+9 for `808_snare`).
- **The delay buffer after play (P3)**: the delay song's ±1 LSB residual starts at frame 85–90
  instead of at the first echo, consistent with the leftover echoes of the play in the buffer (the
  song is not switched before the export).
- **Settling.** No difference beyond the ones above was found between T = 3, 10 and 30 s (one run
  each for 3 and 30 s); every
  exported file starts with 64 frames equal across all runs (the first difference is never before
  frame 64). The task-008 data point (two Fonda Workout exports in one process differ) is the P2
  case.

### 11.3 What the measurements support, and how firmly

- The global 808 snare counter and its 32-entry table (§7.3, read in the disassembly) fit the
  exports: one five-export run fitted uniquely, and the P3 and P4 counters (7 and 4) agree with the
  hits that preceded them.
- [G] The loop-end wrap in the dropped tail of the last export chunk triggers the loop's first step
  (one more drum hit per export when step 0 has one): inferred from the +9 fit only.
- [G] The startup sound did not reach an export in the two runs with it on (one run per variant,
  §11.4).

### 11.4 Startup sound

One run per variant, with "Play Mod Startup Sound" on: `silence` exported 0.04–0.07 s after the
open returned (about 1.5 s after the load, `--wait-after-open 0`) is all zeros, identical to the
export with it off; `808_snare` in the same run differs from P1 only by ±1 LSB, i.e. it is another
discrete `k` variant (and with T = 10 s it is identical to P2 r3's first export, `a465d579b148`). So
in these runs no startup sound reached the export: [G] either the sound is not played for the
standard mod, or it ends within about 1.5 s of the load. Decision (task-010, on this evidence): the
protocol keeps it off (byte 0x20A = 0, `--startup-sound off`), which costs nothing and removes the
path for mods with a longer sample (m-2).

### 11.5 Recommended protocol

The protocol of §9, with T = 10 s: `rb_export.py --restore-prf --startup-sound off
--wait-after-open 10 SONG...` (a fresh process per song, which is the runner's default). Nothing
else is needed:

- Songs whose 303s are silent are **byte-identical** across runs (measured on two songs, three runs
  each, with both 303 devices off and their channels off at level 0; [G] any one of these is
  probably enough, not tested).
- Songs with no 303 notes but audible 303 channels vary by ±1 LSB on up to 1.6 % of the samples.
- Songs with 303 notes vary with `k = N/16` (spread 3 buffers over the 6 fresh first exports of
  `303_accent0`). [G] The export should then be reproducible by a model given `k` (not shown: no
  model render exists yet), so rbhost (task-011/012) should take `k` as a parameter and fit it
  (`phase.py` gives `k` modulo about 10.53 buffers from one note, so the search is short).

Not recommended: two exports in one process (P2: msg 33 state and drum counters), playing before the
export (P3: drum counters, pitch, delay contents), and loading another song first (P4) unless that
song has no snare or bass-drum hits.

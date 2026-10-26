# Song load: from `.rbs` to engine messages

How Rebirth.exe turns an `.rbs` file into messages to the engine in ReBirth.dll, and what the engine
does with them. Traced for task-003 in `ida/Rebirth.exe.i64` and `ida/ReBirth.dll.i64` (2026-10-26).
`0x4xxxxx` is the exe, `0x10xxxxxx` the DLL. **[C]** is checked in the disassembly; **[G]** is a
guess or only partly traced. `docs/RBS42.txt` is the file format; its errors are listed in §10.

Everything was also checked against the 18 songs in `tests/songs/shipped/` (17 are file version 4.1,
`ReBirth 1.0 Default Song.rbs` is 4.2).

## 1. Transport [C]

The exe sends through the shared-memory port object `dword_4DA5EC` (vtable slot 2). All multi-byte
values in every payload are **big-endian** (writers `sub_465FFF` u32, `sub_465F5E` u16; the DLL
reads them back with `sub_100040B0`).

| Form | Bytes | Built by | Sent by |
|---|---|---|---|
| Top level | `[type, payload…]` | `sub_423B76` | `sub_40B142(type, buf, len)` |
| Song message | `[78, song, msg, payload…]` | `sub_423C17` | `sub_40B1E3(song, msg, buf, len)` |
| Song message, no payload | `[78, song, msg]` | `sub_423BA0` | `sub_40B194(song, msg)` |
| Device data | `[78, song, 41, dev, sub, payload…]` | `sub_423C98` | `sub_40B28C(song, dev, sub, buf, len)` |

`song` is the slot the engine returned for this song (§2 step 1), not a device. Replies are read with
`sub_40ABE5`, which skips asynchronous packets of type 69/70; type 65 (`'A'`) is an ack and 75
(`'K'`) means out of memory (the loader then throws `XOutOfMemory`).

**DLL side.** The dispatcher `0x1001CDF0` jumps through the table at `0x10153164`, indexed by the
type byte (valid 5..78). Type 78 (`0x1001D24F`) calls the song object's handler `0x10011ED0` with
`payload+2` (that is, starting at `msg`) and `len-2`. The song handler's table is at `0x10151D6C`
(msg 19..41). Msg 41 (`0x1001285E`) finds the song's device whose id (`sub_10010350`) equals the
`dev` byte among `[song+0x18C + 4i]`, `i < [song+0x1C4]`, and calls its vtable slot 10 (host
message) with the bytes from `sub` on.

| Device | dev | Host-message handler | Data `sub` | Payload |
|---|---|---|---|---|
| Mixer (`CMixerDevice`) | 0 | `0x10015520` | 143 | 29 |
| TB-303 #1 / #2 | 1 / 2 | `0x100256E0` | 39 | 3148 |
| TR-808 | 3 | `0x100282B0` | 56 | 6240 |
| TR-909 | 4 | `0x1002B8E0` | 73 | 6241 |
| Delay | 5 | `0x1000FB40` | 86 | 6 |
| Distortion | 6 | `0x10010D60` | 94 | 4 |
| PCF | 7 | `0x10016C50` | 116 | 8 |
| Compressor | 8 | `0x1000EFD0` | 104 | 5 |

The TRAK track numbers 0..8 (file order) are the same as these device ids.

## 2. The messages at song load, in order [C]

File > Open reaches `sub_40B575`, which builds the song document with `sub_425C4D`. The benchmark
song uses the same reader from `sub_4269CE`.

1. `sub_4349C7` peeks HEAD and GLOB (version, mod name). No message.
2. The mod named in GLOB is looked up (`sub_4155D1`, with a dialog if missing). The engine already
   knows it: `sub_41693C`, called from the mod constructors `sub_417AE9` (the built-in "Standard
   ReBirth", id 411) and `sub_41813E` [G: when that one runs was not traced], registers a mod with
   top-level 56 {u16 id} or 55 (a file mod), after the start-up handshake 50
   (`docs/export_state.md` §12, task-011).
3. Top-level **52** `{u16 mod}` (`sub_425A5D`, 0x425A94): the engine opens a new song and replies
   `'A'` with the song slot in the second byte. Every load asks for a new song slot. [C] The DLL
   queues msg 52 through `sub_1001D694`; `EngineIdle` then runs `sub_1001D840`, which allocates a new
   song (0x1910 bytes, constructor `sub_10011350`) with **new device objects**
   (`docs/export_state.md` §3, task-004).
4. Optional top-level **58** `{u16 mod}` (`sub_426C67` → `sub_40ABB5`), only when the mod record's
   word +1088 is 1 and `sub_4098C3` (app byte `+0x533`, the "Play Mod Startup Sound" preference)
   agrees. [C] It starts the engine's preview player on the mod's startup sample (`sub_1001DC14`,
   `docs/export_state.md` §8); it does not select a sample bank.
5. `sub_434D05` reads the file and sends, in this order:
   1. **36** globals, 23 bytes (`sub_438B4B`), right after GLOB is read (§3).
   2. **41** mixer `0/143` (`sub_438D43`).
   3. **41** delay `5/86` (`sub_438F22`).
   4. **41** PCF `7/116` (`sub_439016`).
   5. **41** distortion `6/94` (`sub_4390F8`).
   6. **41** compressor `8/104` (`sub_4391B6`).
   7. **41** TB-303 #1 `1/39`, then TB-303 #2 `2/39` (`sub_4395B8(…, 1)` / `(…, 2)`).
   8. **41** TR-808 `3/56` (`sub_439DCC`).
   9. **41** TR-909 `4/73` (`sub_43A637`).
   10. **37** track events for tracks 0..8 in file order (`sub_436B6F`; §5), each in one or more
       batches, each batch acknowledged before the next.
6. **32**, no payload (0x426610): the engine relocates to its current position, which chases the
   tracks in song mode (§6).

**No msg 33 is sent at load.** Msg 33 (`sub_10014140`) is sent after an export (0x427DC9) and after
the benchmark (0x4022FE).

**No msg 35, 20 or 21 is sent at load.** The only senders reached from `sub_425C4D` are
`sub_425A5D` (52), `sub_426C67` (58), `sub_434D05`'s callees `sub_438B4B` (36), the nine msg-41
packers through `sub_40B28C`, `sub_436B6F` / `sub_436F5A` / `sub_437380` (37 through `sub_40B1E3`),
and the final `sub_40B194(song, 32)` at 0x426610; `sub_434D05` itself calls no sender. [G] The song
window `sub_42DA72`, built by `sub_40B575` after the load, was not traced for queries.

Do not confuse the song messages 21 and 35 (song-handler table at `0x10151D6C`, indexed by msg):
- msg 21 → `0x100122FC`: a query. It packs the global struct (`sub_10018B00`) and replies with a
  23-byte msg 36. It writes nothing.
- msg 20 → `0x10011FBD`: a query `{id}` that replies with msg 35 `{id, value}`.
- msg 35 → `0x10012555`: set `{id, value}`, switching on the id through the table at `0x10151E14`.
  Id 21 → `0x10012690`, whose store at 0x10012696 writes `+0x16D`. The "msg 21 case at
  0x10012696" of the survey is this msg-35 id 21.

**Version handling.** The file version is HEAD bytes 5..7 (major, minor, revision; HEAD[4] must be
0xBC, `sub_47DB95`). HEAD must start with `[T[T` and be 256 bytes, the major must be 4 (`sub_47DB0B`,
0x434F4E), GLOB must be 512 bytes and USRI 712 bytes. Version-dependent paths:

| Where | 4.0 | 4.1 | 4.2 |
|---|---|---|---|
| MIXR (0x4352C0) | 4×7-byte channels converted (§4.1) | 4×5-byte channels; all four "enabled" forced to 1 (0x435708) | 4×5-byte channels as stored |
| GLOB vintage byte (0x434063) | 0 | 0 | GLOB[482] |
| 808 tom tunings 10, 13, 16 (0x432DA2) | `v/2 + 32` | `v/2 + 32` | as stored |
| Mixer track (0x4364DC) | `sub_437380` converter [G, not traced] | `sub_436B6F` | `sub_436B6F` |
| 808 track (0x4366F6) | `sub_436F5A`: ctrls 10, 13, 16 get `v/2 + 32` | same | `sub_436B6F` |

## 3. Msg 36: globals [C]

Built from GLOB by `sub_433F96` (parse), `sub_438B4B` and `sub_420AE7` (pack; defaults from
`sub_420A81`). The DLL unpacks it with `sub_10018BA0` into the song's global struct at `song+0x158`
and applies each field (0x100126B2).

| Byte | From | Value at load | DLL action (msg 35 id) |
|---|---|---|---|
| 0 | GLOB[0] | 0 pattern / 1 song mode | `sub_1001FBE0` (id 0) |
| 1 | — | 0 | play/stop: 0 → `sub_1001E100` stop (id 1) |
| 2 | — | 0 | `sub_10021190`, a thunk to `0x10021194` (id 2) [G: meaning not traced] |
| 3 | GLOB[1] | loop on | `sub_1001C0A0` (id 3) |
| 4..7 | GLOB[2..5] | tempo, BPM×1000 | `sub_1001BFC0` (id 4) |
| 8..11 | — | 0 | locate to this tick, `sub_1001BF70` (id 8) |
| 12..15 | GLOB[6..9] | loop start in ticks (bar×768), **not scaled** | `sub_1001C000` (id 12) |
| 16..19 | GLOB[10..13] | loop end in ticks | `sub_1001C050` (id 16) |
| 20 | GLOB[14] | shuffle 0..127 | `sub_10021170` (id 20) |
| 21 | GLOB[482] (0 before 4.2) | sound model, stored at `song+0x16D` | (id 21) |
| 22 | — | 1 | stored at `song+0x16E`; no reader found in the DLL |

Msg 35 is `{id, value}`: one value byte, except ids 4, 8, 12 and 16, which carry a u32
(`{8, start}` is 5 bytes).

**The sound-model byte (`song+0x16D`, "+21" of the global struct).** It is GLOB[482] unchanged; the
exe never inverts it. The menu item "Vintage ReBirth Synth Sound" (command 323, handler 0x42C32E)
toggles the byte and sends msg 35 `{21, v}` (`sub_429D39`); the menu check mark shows the byte
(0x42BF09). In the engine:
- 303 audio `0x10023F10` tests the byte (0x10023F2D): **zero tail-calls `0x100248C4`**, non-zero runs
  the inline code. Note-on (0x10023D13) uses `^64` coefficients when zero and `^4` when non-zero;
  note-on and note-off amp constants switch on it too (0x10023E1E, 0x10023C94).
- The mixer's 16-block update (0x10012C9F) uses the plain send `T[lev]*T[send] + 1e-6f` when zero and
  the pan-dependent send when non-zero.

So value 0 (RBS "ReBirth 2.0 sound", every shipped song) selects `0x100248C4` and the plain send,
and value 1 ("Vintage") selects the inline engine of `0x10023F10` and the pan-dependent send. The labels
"2.0" and "vintage" in `docs/initial_investigation.md` §6 were the wrong way round and are corrected
there. Nothing else writes the byte: the export driver sends only msg 35 ids 1 and 8.

## 4. Msg 41: device data

The exe parses each DEVL chunk into an intermediate struct, then copies that into the message buffer
`unk_4DB948` with a per-device packer. The packers use obfuscated indices (§9); the layouts below are
the intended ones, which are also the ones the DLL unpackers read.

**Pattern mode versus song mode in the DLL [C for the mixer and the 303].** Each device keeps the
received block as its pattern-mode copy. The live values are replaced from it only when the song is
in pattern mode (`sub_1001FCB0` returns global byte 0):
- 303 (msg 39, 0x10025CB0): patterns always go live (the step function reads them at `+0x3C`); the
  12 parameter bytes go to the live `+0xC7C..+0xC87` only in pattern mode.
- Mixer (msg 143, 0x1001588F): pattern mode copies everything; song mode copies only the master level
  and the four channel "enabled" bytes, the ones no TRAK controller can set.
- [G] Delay, PCF, distortion, compressor, 808 and 909 follow the same rule; their msg-41 cases were
  located (§1) but not read through.

### 4.1 Mixer, `0/143`, 29 bytes

Parse `sub_433C51` + `sub_433D0B` (4.1/4.2), pack `sub_423427` + `sub_423597` via `sub_4236AD`,
DLL unpack `sub_10019DE0` / `sub_10019D14` into `+0x58`, live copy at `+0x30`.

| Payload | From MIXR | Notes |
|---|---|---|
| 0 | [0] master level | |
| 1 | [1] compressor id | 0 off, 1 master (stereo), 2..5 = channel 0..3 |
| 2 | [2] PCF id | 0 off, **1..4 = channel 0..3** |
| 3, 4 | — | 0 |
| 5 + 6c + 0..4 | [16+12c .. 20+12c] | enabled, level, pan, delay send, dist on; c = 0 303 #1, 1 303 #2, 2 808, 3 909 |
| 5 + 6c + 5 | — | 0 |

Routing in the DLL (`sub_10012C14`, 0x10012D9D..0x1001306E): channel c gets the distortion insert if
its dist byte is non-zero, the PCF insert if the PCF id is c+1, and the mono compressor if the
compressor id is c+2; id 1 puts the stereo compressor on the master. The pattern-mode MIXR PCF byte
is 0 in every shipped song; the TRAK values are 0..4.

The 4.0 layout has 7 bytes per channel: `[?, level, pan, send, dist, PCF flag, comp flag]`. The
converter (0x4352C0) sets enabled = 1, compressor id = (MIXR[1] != 0) or `c+2` for a channel with the
comp flag, PCF id = `c+1` for a channel with the PCF flag. No 4.0 song was available to check.

### 4.2 Delay, `5/86`, 6 bytes

DELY[0..4] = enabled, steps (1..32), mode, feedback, pan; byte 5 = 0 (parse `sub_43465C`, pack
`sub_422EB7`, defaults `sub_422E80` = 1, 3, 1, 80, 64). Live struct at `+0x34`.

**Mode.** The update `0x1000F754` (test at 0x1000F7E5) uses `q = 4*T/100` when the mode byte is
non-zero and `q = 3*T/100` when it is zero: **0 = triplet steps (1/3 beat), 1 = sixteenths
(1/4 beat)**. The exe passes DELY[2] and the TRAK value unchanged. RBS42.txt has the polarity
backwards. Shipped songs use mode 1 (the default is 3 steps of mode 1, a dotted eighth).

### 4.3 PCF, `7/116`, 8 bytes

PCF[0..6] = enabled, frequency, resonance, amount, wave, decay, mode; byte 7 = 0 (parse
`sub_434750`, pack `sub_4232A3`, defaults `sub_42325E` = 1, 70, 64, 40, 24, 90, 1). Mode 0 = band-pass,
1 = low-pass, as RBS42.txt says (the DLL selects the table by mode, `docs/initial_investigation.md`
§8.2).

### 4.4 Distortion, `6/94`, 4 bytes

DIST[0..2] = enabled, amount, shape; byte 3 = 0 (parse `sub_4345A6`, pack `sub_423048`, defaults
`sub_42301F` = 1, 70, 0).

### 4.5 Compressor, `8/104`, 5 bytes

COMP[0..2] = enabled, amount, threshold; bytes 3, 4 = 127, 0 (parse `sub_434810`, pack `sub_42314F`,
defaults `sub_42311F` = 1, 70, 64, 127, 0).

### 4.6 TB-303, `1/39` and `2/39`, 3148 bytes

Parse `sub_43414D` + `sub_434202` + `sub_434309` (via `sub_4344E1`), pack `sub_420D10` + `sub_420ECF`
+ `sub_420FB7` (via `sub_4213E7` / `sub_421296`), DLL unpack `sub_10018EF0` into `+0x30`.

| Payload | Content |
|---|---|
| 0..8 | 303 chunk [0..8]: enabled, pattern, tune, cutoff, resonance, envmod, decay, accent, waveform |
| 9..11 | 0 |
| 12 + 98p | pattern p (0..31): shuffle flag, length |
| 14 + 98p + 6s | step s (0..15): `{note, slide, accent, up, down, rest}` |

**Step conversion** (`sub_434309`, checked instruction by instruction):
`note = byte0` (0..12); then from the flags byte, each normalised to 0/1:
`slide = bit0`, `accent = bit1`, `up = bit2`, `down = bit3`, `rest = bit4`.
Bit 4 set means **rest** (pause): every step of `Silent Default Song.rbs` is `00 10`, and the DLL
note-on plays when `step[5] == 0` (0x10023E04). RBS42.txt's "bit 4 = Pause/Note" means 1 = pause.

The pattern flag 0 = shuffle off selects the unshuffled tick in the step function (0x100253D0).

### 4.7 TR-808, `3/56`, 6240 bytes

Parse `sub_432C58` (with the file version) + `sub_432E6E` + `sub_432F8D` (via `sub_4331CC`), pack
`sub_4215E5` + `sub_42199E` + `sub_421ABD` (via `sub_422063`), defaults `sub_4214F8`.

| Payload | Content |
|---|---|
| 0..28 | 808 chunk [0..28] in RBS42 order; for files before 4.2, [10], [13], [16] (tom tunings) = `v/2 + 32` (`sub_432E1B`) |
| 29..31 | 0 |
| 32 + 194p | pattern p: shuffle flag, length |
| 34 + 194p + 12s + i | step s, instrument i in RBS order (AC, BD, SD, LT, MT, HT, RS, CP, CB, CY, OH, CH): `raw != 0` (0 or 1) |

The accent column (i = 0) is a normal 0/1 byte. The 808 step function uses it as
`accent = f32(acc*AC*0.03125f + 1)` (`docs/initial_investigation.md` §7.2).

### 4.8 TR-909, `4/73`, 6241 bytes

Parse `sub_43348D` + `sub_43363E` + `sub_43375D` (via `sub_43392A`), pack `sub_42222B` + `sub_4225DF`
+ `sub_42272B` (via `sub_422CE4`, `sub_4228CA`), defaults `sub_422137`.

| Payload | Content |
|---|---|
| 0..29 | 909 chunk [0..29] in RBS42 order, except byte 16 (see below) |
| 30..32 | 0 |
| 33 + 194p | pattern p: shuffle flag, length |
| 35 + 194p + 12s + i | step s, instrument i in RBS order (AC, BD, SD, LT, MT, HT, RS, CP, CH, OH, CC, RC): the RBS value **unchanged**, 0..3 |

- The parse reorders bytes 20..22 (RimShot, Clap, HiHat level) internally and the packer puts them
  back, so the message keeps the RBS order.
- **Quirk:** the parse copies RBS byte 13 (LowTomDecay) into slot 16 (MidTomDecay) and drops RBS byte
  16 (0x433558: `mov dl,[ecx+0Dh]; mov [eax+10h],dl`). The pattern-mode MidTomDecay is therefore the
  LowTomDecay. The TRAK controller 0x10 is not affected.
- **Step values.** The DLL step function `0x1002AAC0` reads them raw: 1 or 3 → base×0.8, 2 or 4 →
  base×2.0, values > 2 also flam. With RBS values: 0 off, 1 on, 2 accented, 3 flam (at the 1 level);
  4 never occurs. There is no shift to 1..4.
- The AC column holds 0 or 2 in the shipped songs; `base = AC*acc*0.03125 + 1` (0x1002ACD1), so a 909
  accent step weighs twice an 808 one.

## 5. Msg 37: track events [C]

`sub_436B6F(…, song, track, rangeTable)` reads one TRAK chunk: a u32 event count, then per event a
MIDI-style variable-length delta (`sub_4A09D1`), a controller and a value.

**Position.** `abs += delta`; `tick = (abs * 0xC0) >> 3` in u32, i.e. **`tick = abs × 24`** (0x436CF3).
The RBS position unit is a 32nd note (32 per bar, 8 per quarter), not 24 PPQ: in the shipped songs
the last event divided by 32 lands just before the loop end in bars (TGV 544 vs 547, Groove 233 vs
235, Fast Forward 248 vs 250). Pattern changes (controller 1 of tracks 1..4) are all multiples of 32.

**Validation** (events that fail are dropped, not clamped):
- The tick must be below 768000 (= 1000 bars) and a multiple of 24 (`sub_42FD60`). When it fails, the
  running position `abs` is **not** advanced by that event's delta.
- The controller must be enabled for the track and the value within its range (`sub_42FCE5`,
  `sub_42FD00`; tables built by `sub_42FB5C`, `sub_42FAD8` and the range builders below).

| Track | Controllers accepted | Ranges | Range table |
|---|---|---|---|
| 0 mixer | 1, 2, and 6..9 + 6c for c = 0..3 (6–9, 12–15, 18–21, 24–27) | 1: 0..5, 2: 0..4, level/pan/send 0..127, dist 0..1 | `sub_42F8F0` → 0x4DB5F8 |
| 1, 2 TB-303 | 0..8 | 0: 0..1, 1: 0..31, 2..7: 0..127, 8: 0..1 | `sub_42F34B` → 0x4DB580 |
| 3 TR-808 | 0..28 | 0: 0..1, 1: 0..31, 2..28: 0..127 | `sub_42F3E9` → 0x4DB5B8 |
| 4 TR-909 | 0..29 | 0: 0..1, 1: 0..31, 2..29: 0..127 | `sub_42F59F` → 0x4DB638 |
| 5 delay | 0..4 | 0: 0..1, 1: 1..32, 2: 0..1, 3, 4: 0..127 | `sub_42F763` → 0x4DB678 |
| 6 distortion | 0..2 | 0: 0..1, 1, 2: 0..127 | `sub_42F7C9` → 0x4DB688 |
| 7 PCF | 0..6 | 0: 0..1, 1..5: 0..127, 6: 0..1 | `sub_42F86E` → 0x4DB5A8 |
| 8 compressor | 0..2 | 0: 0..1, 1, 2: 0..127 | `sub_42F813` → 0x4DB598 |

Mixer controller numbering: `1` compressor id, `2` PCF id, and for channel c (0 303 #1, 1 303 #2,
2 808, 3 909): `6+6c` level, `7+6c` pan, `8+6c` delay send, `9+6c` dist on. The DLL mixer setParam
`0x10015C00` decodes `(ctrl-5)/6` and `(ctrl-5)%6` the same way. Master level and channel enable are
not automatable. Other tracks use the controller numbers of RBS42.txt, which equal the chunk byte
offsets.

**Packet.** The events are sent in batches of at most 1284 file events (`sub_467E70(1284, left)`):
`[track, u32 first, u32 last]` (`sub_423928`), then for each accepted event `[99, u32 tick, ctrl, value]`
(`sub_423A37`), where `first` and `last` count accepted events before and after the batch. A batch
with no accepted event is not sent. Each batch waits for the reply; `'K'` throws out of memory. The
DLL appends them with `sub_10021010` (song msg 37, 0x100127EB).

## 6. What decides the state at an export [C]

The song's global byte 0 (song mode) decides:

- **Pattern mode:** the device chunks (msg 41) are the live values; tracks are not chased.
- **Song mode:** locate (`0x1001F984`) chases every track: `sub_100151F0` fills a 128-entry state
  with 0xFF, then sets each controller to the value of its last event with `tick <= P`; every entry
  that is not 0xFF is applied through `sub_10012B40` (device setParam). Controllers without such an
  event keep their live value, which in song mode is **not** taken from the device chunk (§4), except
  the mixer master and channel enables. Song mode therefore takes every automatable value from TRAK.

Every shipped song has an event at tick 0 for every accepted controller of every track, so for them
the TRAK values at the loop start are the whole automatable state; the chunks still provide the
patterns, the mixer master and enables, and the bytes no controller reaches. The export's locate to
the loop start (msg 35 `{8, start}`) runs this chase.

Load itself: msg 36 sets the mode (song mode chases the still empty tracks) and locates to tick 0;
the device data follows; msg 32 at the end re-locates to the current position (tick 0), which chases
the tracks just loaded.

## 7. Byte `+0x16D` (task-003 item "mixer flag")

The song-object byte `+0x16D` is the global struct's byte 21. It is written only by msg 36 byte 21
(0x100127D7) and msg 35 `{21, v}` (0x10012696), and read by the mixer update (0x10012C9F) and the
303 code (§3). The exe sends GLOB[482] at load (0 for files before 4.2) and the menu toggle sends
0/1. **During an export it holds the song's GLOB[482], 0 for every shipped song.**

## 8. Values that pass unchanged

All other chunk bytes and TRAK values reach the engine unchanged. In particular the 303 waveform
(0 = saw, 1 = square per `docs/initial_investigation.md` §6.1; RBS42.txt says "triangle" for 0), the
shuffle amount, the tempo, and the loop points (already in ticks).

## 9. The copy protection in the packers

Many packers write one field at a computed index instead of a constant:

| Packer | Field | Intended index |
|---|---|---|
| `sub_420AE7` (msg 36) | shuffle | 20 |
| `sub_420FB7` (303 step) | accent, down | 2, 4 |
| `sub_421296` (303 pattern) | step loop increment | 1 |
| `sub_420D10` (303 params) | accent knob | 7 |
| `sub_4236AD` (mixer) | channel count | 4 |
| `sub_423427` / `sub_423597` (mixer) | header byte 4 / channel byte 5 | 4 / 5 |
| `sub_423C98` / `sub_423BA0` (headers) | payload offset / length | 5 / 3 |
| `sub_422EB7` (delay) | mode | 2 |
| `sub_4232A3` (PCF) | wave | 4 |
| `sub_423048` (dist) | amount | 1 |
| `sub_42314F` (comp) | amount | 1 |
| `sub_4215E5` (808 params) | bytes 14, 24 | 14, 24 |
| `sub_421ABD` (808 step) | LT column | 3 |
| `sub_42272B` / `sub_4228CA` / `sub_422CE4` (909) | RC column / pattern offset / pattern count | 11 / 0 / 32 |
| `sub_423A37` (track event) | tick offset | 0 |

The intended values follow from fixed parts of the code (for example `sub_423BA0`'s result is
used as `result+1 = 4` in `sub_423427`, and the same expression appears as 7 in `sub_420D10` and
`7-2 = 5` in `sub_423597`).

**How it works.** The expressions combine `.bss` words that are never written by code with values
read from the system:
- the words lie in four 2048-byte pages filled by `sub_4065F2` from `Rb20ful.dat`, at file offsets
  chosen by the process id (kernel32 table slot +72, masked 0x3EF7 [G: the slot is read as
  GetCurrentProcessId; `sub_42314F` calls GetCurrentProcessId directly with the same mask]), by the
  parent window (`GetWindowLongA(hwnd, -8)`), by the module handle (`GetWindowLongA(hwnd, -6)`) and
  one fixed offset;
- the code mixes in the same runtime values (`GetWindowThreadProcessId` on a handle, `GetParent`, the
  PID), so each page holds constants that cancel them;
- `sub_406A79` / `sub_4063DF` look for the CD (volume "ReBirth Win", `Rb20upd.dat` / `Rb20ful.dat`)
  and validate the file (magic 0x03E1F11E, the string `RB338V20`, major version 4). The strings are
  stored encoded: decode with `s[j] -= s[j+1]` from the end.
- Anti-debugger branches (`IsDebuggerPresent` in `sub_420FB7`, `\\.\SICE` in `sub_4236AD`,
  `sub_421ABD`, `sub_4120EF`, an IDT check on Windows 9x in `sub_421ABD` / `sub_423A37`) select the
  intended constant directly.

**When it fires.** Only when the pages do not hold the genuine data: a build that skips the CD check,
or a missing or altered `Rb20ful.dat`. Then fields land at wrong offsets (for example the 303 accent
overwrites the note, or the compressor amount moves with the PID) and some writes leave the buffer.
A normal load cannot trigger it: if `sub_4063DF` fails, the app throws `XBase` at start-up (0x40EA7A)
and never loads a song. The app on the test machine starts and loads songs, so it reads valid pages,
and its packets have the intended layout. rbhost and the Rust engine use the intended layout.

## 10. Errata for RBS42.txt

| RBS42.txt | Actual |
|---|---|
| TRAK positions are "192 PPQ divided by 8, 24 PPQ (32th notes)" | The unit is a 32nd note: 32 per bar; engine tick = position × 24 |
| Mixer track: 909 delay send 0x20, dist 0x21 | 0x1A, 0x1B (controllers 24..27 are the 909 channel) |
| PCF device id 0x02..0x05 | 0x01..0x04 (303 #1, 303 #2, 808, 909) |
| 303 flags "bit 4 = Pause/Note" | bit 4 set = rest |
| Delay step mode 0 = quarter notes, 1 = triplets | 0 = 1/3 beat (triplets), 1 = 1/4 beat (sixteenths) |
| Selected pattern 0x00 to 0x20 | 0..31 (the exe rejects 32 in tracks) |
| 303 waveform 0 = triangle | 0 = saw |
| USRI is 512 bytes | 712 bytes in real files; the exe requires 712. From the 18 shipped songs (tools/rbs.py): title ZSTRING at 0 (41), text ZSTRING at 41 (401 bytes, not 201), web page ZSTRING at 442 (101), show-on-open byte at 543; bytes 544..711 hold leftover junk that the engine does not use |
| TRAK accumulated position must not exceed 31976 | `Alloy Cowboy Resurrection.rbs` has events above 31976 in every track, up to 41024; the exe drops those at or above 32000 (tick 768000, section 5) |

## 11. Open items

Follow-up: task-034.

- [C] Top-level msg 52/58 handling inside the DLL: a new song creates new device objects, and 58
  starts the startup-sound preview (task-004, `docs/export_state.md` §3, §8).
- [G] The constructor values of the controllers that a song-mode song never sets.
- [G] The msg-41 song-mode rule for delay, PCF, distortion, compressor, 808 and 909.
- [G] The 4.0 mixer-track converter `sub_437380`.
- [G] Global byte 2 (msg 35 id 2, `0x10021194`) and byte 22 (`+0x16E`).

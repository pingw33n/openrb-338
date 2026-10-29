# Song load: from `.rbs` to engine messages

How Rebirth.exe turns an `.rbs` file into messages to the engine in ReBirth.dll, and what the engine
does with them. Traced for task-003 in `Rebirth.exe.i64` and `ReBirth.dll.i64` (2026-10-26).
`0x4xxxxx` is the exe, `0x10xxxxxx` the DLL. **[C]** is checked in the disassembly; **[G]** is a
guess or only partly traced. `docs/RBS42.txt` is the file format; its errors are listed in §10.

Everything was also checked against the 18 songs in `tests/songs/shipped/` (17 are file version 4.1,
`ReBirth 1.0 Default Song.rbs` is 4.2).

Two implementations follow this document: `tools/rbhost/rbsmsg.py` (for rbhost) and
`crates/orb-engine/src/load.rs` (the engine, task-018), which also covers the 4.0 paths. The test
`crates/orb-engine/tests/rbsmsg_corpus.rs` checks that both give the same messages for every
shipped and diagnostic song.

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
song uses the same reader from `sub_4269CE`. `sub_425C4D` first tries the file as a ReBirth 1.x
song, a MIDI file (0x425DE4); that import is section 12. The steps below are the RBS path.

1. `sub_4349C7` peeks HEAD and GLOB (version, mod name). No message.
2. The mod named in GLOB is looked up (`sub_4155D1`, with a dialog if missing). The engine already
   knows it: `sub_41693C`, called from the mod constructors `sub_417AE9` (the built-in "Standard
   ReBirth", id 411) and `sub_41813E` (a file mod; both built by `sub_416300` for the mod's first
   user, `docs/rbm.md` section 14.1 [C], task-056), registers a mod with
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
| Mixer track (0x4364DC) | `sub_437380` converter (§5.1) | `sub_436B6F` | `sub_436B6F` |
| 808 track (0x4366F6) | `sub_436F5A`: ctrls 10, 13, 16 get `v/2 + 32` | same | `sub_436B6F` |

**What "4.0", "4.1" and "4.2" mean [C] (task-018).** The exe keeps the version as a CVersion
(`sub_47D768`) and compares the key `major << 16 | minor << 8 | revision` (`sub_47D9ED`). The
constants are CVersion(4,0) at 0x4DB938 (`sub_43295F`), (4,1) at 0x4DB928 (`sub_4329A2`) and (4,2)
at 0x4DB918 (`sub_4329E5`), all with revision 0. Two tests are used:
- `sub_47DA82` is **equality**: the 4.0 MIXR converter (0x4352C0) and the 4.0 mixer track
  (0x4364DC) run only for exactly 4.0.0, the 4.1 enable forcing (0x435708) only for exactly 4.1.0.
- `sub_47DAB5(file, 4.2, 4.2)` is key >= 4.2.0 and major <= 4: the GLOB vintage byte (0x43405E), the
  808 chunk tunings (0x432D95) and the 808 track (0x4366F6).

So a file of version 4.1.1 gets neither the 4.1 enable forcing nor any 4.2 path; 4.0.1 is read like
4.1 without the forcing. Every shipped and diagnostic song has revision 0.

## 3. Msg 36: globals [C]

Built from GLOB by `sub_433F96` (parse), `sub_438B4B` and `sub_420AE7` (pack; defaults from
`sub_420A81`). The DLL unpacks it with `sub_10018BA0` into the song's global struct at `song+0x158`
and applies each field (0x100126B2).

| Byte | From | Value at load | DLL action (msg 35 id) |
|---|---|---|---|
| 0 | GLOB[0] | 0 pattern / 1 song mode | `sub_1001FBE0` (id 0) |
| 1 | — | 0 | play/stop: 0 → `sub_1001E100` stop (id 1) |
| 2 | — | 0 | `sub_10021190` (called at 0x100125B6 and 0x10012724), a 4-byte prologue `mov eax, [esp+4]` that falls through into `0x10021194` (id 2): record (task-022) |
| 3 | GLOB[1] | loop on | `sub_1001C0A0` (id 3) |
| 4..7 | GLOB[2..5] | tempo, BPM×1000 | `sub_1001BFC0` (id 4) |
| 8..11 | — | 0 | locate to this tick, `sub_1001BF70` (id 8) |
| 12..15 | GLOB[6..9] | loop start in ticks (bar×768), **not scaled** | `sub_1001C000` (id 12) |
| 16..19 | GLOB[10..13] | loop end in ticks | `sub_1001C050` (id 16) |
| 20 | GLOB[14] | shuffle 0..127 | `sub_10021170` (id 20) |
| 21 | GLOB[482] (0 before 4.2) | sound model, stored at `song+0x16D` | (id 21) |
| 22 | — | 1 | stored at `song+0x16E`; no effect in the DLL (below) |

Msg 35 is `{id, value}`: one value byte, except ids 4, 8, 12 and 16, which carry a u32
(`{8, start}` is 5 bytes).

**Byte 22 (`song+0x16E`) [C] (task-034).** It has no effect on the engine. The globals ctor
`sub_10018AC0` sets it to 1 (0x10018AEA); it is written only by msg 35 id 22 (0x100126A7, the
switch's case 22 at 0x100126A1) and msg 36 (0x100127E0); the only read is the msg 21 reply's
pack `sub_10018B00` (0x10018B8B), which echoes it to the exe. A scan of the whole DLL
(`llvm-objdump` of the installed `ReBirth.dll`) finds no other `+0x16E` displacement and no other
byte access at `+0x16` outside the globals struct's own functions (`sub_10018AC0`,
`sub_10018B00`, `sub_10018BA0`) and the parameter-block functions of other structs; msg 20's
query answers only ids 0..2 (switch at 0x10011FC2). The exe always sends 1. What the exe means by
it is not traced [G]; for orb it is a stored byte (`Globals::byte22`).

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

**Pattern mode versus song mode in the DLL [C for every device].** Each device keeps the
received block as its pattern-mode copy. The live values are replaced from it only when the song is
in pattern mode (`sub_1001FCB0` returns global byte 0):
- 303 (msg 39, 0x10025CB0): patterns always go live (the step function reads them at `+0x3C`); the
  12 parameter bytes go to the live `+0xC7C..+0xC87` only in pattern mode.
- Mixer (msg 143, 0x1001588F): pattern mode copies everything; song mode copies only the master level
  and the four channel "enabled" bytes, the ones no TRAK controller can set.
- [C] 808 (msg 56, 0x1002884C, task-024) and 909 (msg 73, 0x1002BE90, task-025): the parameter
  block and the patterns go into the pattern-mode copy `+0x30`; the whole parameter block (32 or 33
  bytes) goes live only in pattern mode (the patterns are read from `+0x50` / `+0x51` in both modes).
- [C] Delay (msg 86, 0x1000FD7D) and distortion (msg 94, 0x10010F77), task-026: the block goes
  into the pattern copy (`+0x3A`, 6 bytes, `sub_10019B00`; `+0x3D`, 4 bytes, `sub_10019B70`) and
  is copied live (`+0x34`, `+0x39`) only when `sub_1001FCB0` (song mode) is zero. Their message
  handlers (0x1000FB40: msgs 81..86; 0x10010D60: 89..94) index the jump table with the raw message
  byte (`jmp [eax*4 + 0x101516A8]`, table at 0x101517EC for 81). Song mode reaches the live block
  only through the track's controllers (slot 11) and the chase.
- [C] task-027: the PCF (msg 116, 0x10016EA5: `sub_10019C80` into `+0x40`, 8 bytes; live `+0x38`)
  and the compressor (msg 104, 0x1000F1DF: `sub_10019BE0` into `+0x3D`, 5 bytes; live `+0x38`)
  follow the same rule: the block goes live only when `sub_1001FCB0` is zero. Their handlers
  (0x10016C50: msgs 111..116; 0x1000EFD0: 99..104) index the jump table with the raw byte too.
  Neither checks a byte: the PCF's wave reaches the pattern lookup unchanged (§4.3 and
  `docs/initial_investigation.md` §8.2).

So in song mode no msg-41 byte that a controller can reach goes live, for any of the nine
devices; such a controller takes its value from the track, whose tick-0 event is the song's or,
when the song has none, the constructor's (§4.9).

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
comp flag, PCF id = `c+1` for a channel with the PCF flag. Two 4.0 songs render equal in orb and in
the original DLL through rbhost (task-061; see the open items at the end).
[C] (task-018) The parse `sub_433B86` copies MIXR[0], MIXR[1] and the four 7-byte channels at
`MIXR[16 + 12c]`, the same offsets as the 5-byte channels of later versions. The target struct starts
from `sub_433BEA`'s defaults (master 100, compressor id 0, PCF id 0). The channels are processed in
order 0..3, so the last channel with a flag wins. MIXR[2] (the later PCF id) is not read, and channel
byte 0 is ignored. `sub_438D43` packs the result like a 4.1 mixer.

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
- [C] task-025: the DLL permutes them again. Its blocks (`+0x30`, live `+0x1891`) hold the hi-hat
  level at 20, the rim shot's at 21 and the clap's at 22: msg 73's parse `sub_100196F4` stores
  message bytes 20, 21, 22 at 21, 22, 20, the read-back `sub_10019624` inverts it, and set-param
  (0x1002C6D0) maps controllers 20, 21, 22 (RBS order, §5) to bytes 21, 22, 20. Controller 29
  (flam interval) is set-param's case 29 (0x1002C80B), so a TRAK event can set it
  (`docs/initial_investigation.md` §7.2).
- **Quirk:** the parse copies RBS byte 13 (LowTomDecay) into slot 16 (MidTomDecay) and drops RBS byte
  16 (0x433558: `mov dl,[ecx+0Dh]; mov [eax+10h],dl`). The pattern-mode MidTomDecay is therefore the
  LowTomDecay. The TRAK controller 0x10 is not affected. (A defect of the original:
  [OB-1](original_bugs.md).)
- **Step values.** The DLL step function `0x1002AAC0` reads them raw: 1 or 3 → base×0.8, 2 or 4 →
  base×2.0, values > 2 also flam. With RBS values: 0 off, 1 on, 2 accented, 3 flam (at the 1 level);
  4 never occurs. There is no shift to 1..4.
- The AC column holds 0 or 2 in the shipped songs; `base = AC*acc*0.03125 + 1` (0x1002ACD1), so a 909
  accent step weighs twice an 808 one.

### 4.9 Constructor defaults of the automatable parameters [C] (task-034)

Msg 52 builds a new song with new device objects (§2 step 3; `docs/export_state.md` §3). Each
device ctor fills **both** parameter blocks, the live one and the pattern-mode copy, with one
default function; then the song ctor (`sub_10011350`) calls slot 7 on every device (0x10011941;
a no-op `0x1000F4E0` except the 303/808/909's `0x10026C40`, which quantises controllers 0 and 1
to a bar) and slot 8 with tick 0 (0x10011985), which inserts a **tick-0 event of every
automatable controller with the pattern copy's value** into the device's own track
(`sub_1001F900` → `sub_10014750`). A song's own tick-0 event of the same controller replaces it
(§6). So in song mode every automatable controller (for the mixer 1, 2 and `6..9 + 6c`) always
has a tick-0 event, and the chase
(§6) sets it to the song's last event at or before the position or, when the song has none
there, to the default below.

| Device (track) | Default function: call, block | Slot 8: controllers, from |
|---|---|---|
| Mixer (0) | `sub_10019CC0` header + `sub_10019D50` ×4 channels: 0x10015457 / 0x10015466 live `+0x30` / `+0x40`; 0x10015477 / 0x10015489 pattern `+0x58` / `+0x68` | `0x100160F0`: 1, 2, then `6..9 + 6c`, pattern copy |
| TB-303 (1, 2) | `sub_10018C20`: 0x10023977 pattern `+0x30`; 0x100239B8 live `+0xC7C` | `0x10025650`: 0..8 |
| TR-808 (3) | `sub_10018FB0`: 0x10026FD3 pattern `+0x30`; 0x1002701A live `+0x1890` | `0x10029DA0`: 0..28, pattern copy |
| TR-909 (4) | `sub_100195A0`: 0x1002A3E3 pattern `+0x30`; 0x1002A42A live `+0x1891` | `0x1002D480`: 0..29, pattern copy in RBS order (`sub_10019620`) |
| Delay (5) | `sub_10019AB0`: 0x1000F5FD live `+0x34`; 0x1000F608 pattern `+0x3A` | `0x10010020`: 0..4, pattern copy |
| Distortion (6) | `sub_10019B30`: 0x1001075D live `+0x39`; 0x10010768 pattern `+0x3D` | `0x10010FE0`: 0..2, pattern copy |
| PCF (7) | `sub_10019C10`: 0x100164ED live `+0x38`; 0x100164F8 pattern `+0x40` | `0x10017200`: 0..6, pattern copy |
| Compressor (8) | `sub_10019B90`: 0x1000E9ED live `+0x38`; 0x1000E9F8 pattern `+0x3D` | `0x1000F420`: 0..2, pattern copy |

Both blocks get the same values, so which one slot 8 reads does not matter at the ctor.

The defaults, by controller (= RBS chunk byte, §5), from the stores of the default functions:

| Device | Controller = default |
|---|---|
| Mixer | 1 compressor id = 0, 2 PCF id = 0; per channel c: `6+6c` level = 100, `7+6c` pan = 64, `8+6c` delay send = 0, `9+6c` dist = 0. Not automatable: master = 100, enabled = 1 |
| TB-303 | 0 enabled = 1, 1 pattern = 0, 2 tune = 64, 3 cutoff = 20, 4 resonance = 90, 5 envmod = 20, 6 decay = 100, 7 accent = 64, 8 waveform = 0 (saw); bytes 9..11 = 0 |
| TR-808 | 0 enabled = 1, 1 pattern = 0, 2 accent = 40, 3 BD level = 100, 4 BD tone = 64, 5 BD decay = 64, 6 SD level = 100, 7 SD tone = 64, 8 SD snappy = 64, 9 LT level = 100, 10 LT tuning = 64, 11 LT selector = 0, 12 MT level = 100, 13 MT tuning = 64, 14 MT selector = 0, 15 HT level = 100, 16 HT tuning = 64, 17 HT selector = 0, 18 RS level = 100, 19 RS selector = 0, 20 CP level = 100, 21 CP selector = 0, 22 CB level = 100, 23 CY level = 100, 24 CY tone = 0, 25 CY decay = 100, 26 OH level = 100, 27 OH decay = 100, 28 CH level = 100; bytes 29..31 = 0 |
| TR-909 | 0 enabled = 1, 1 pattern = 0, 2 accent = 32, 3 BD level = 100, 4 BD tune = 64, 5 BD attack = 64, 6 BD decay = 127, 7 SD level = 100, 8 SD tune = 64, 9 SD tone = 64, 10 SD snappy = 64, 11 LT level = 100, 12 LT tune = 64, 13 LT decay = 127, 14 MT level = 100, 15 MT tune = 64, 16 MT decay = 127, 17 HT level = 100, 18 HT tune = 64, 19 HT decay = 127, 20 RS level = 100, 21 CP level = 100, 22 HH level = 100, 23 CH decay = 32, 24 OH decay = 127, 25 crash level = 100, 26 crash tune = 64, 27 ride level = 100, 28 ride tune = 64, 29 flam interval = 64; bytes 30..32 = 0 (the DLL's byte permutation of 20..22, §4.8, does not matter: all three are 100) |
| Delay | 0 enabled = 1, 1 steps = 3, 2 mode = 1 (sixteenths), 3 feedback = 80, 4 pan = 64; byte 5 (meter) = 0 |
| Distortion | 0 enabled = 1, 1 amount = 70, 2 shape = 0; byte 3 (meter) = 0 |
| PCF | 0 enabled = 1, 1 frequency = 70, 2 resonance = 64, 3 amount = 40, 4 wave = 24, 5 decay = 90, 6 mode = 1 (low-pass); byte 7 (meter) = 0 |
| Compressor | 0 enabled = 1, 1 amount = 70, 2 threshold = 64; bytes 3, 4 (meters) = 127, 0 |

The mixer's struct is not the message layout: `sub_10019CC0` stores the master byte at `+0`, the
compressor id and the PCF id as dwords at `+4` and `+8` (bytes `+1..+3` are not written),
bytes `+0xC`, `+0xD` 0; the channels follow at `+0x10 + 6c`
(enabled, level, pan, send, dist, 0). The msg-41 unpack `sub_10019D14` / `sub_10019DE0` fills the
same fields. The defaults equal the exe's own chunk defaults (§4.2 to §4.5) for the four effects.

orb: `MixerParams::default`, `tb303::DEFAULT_PARAMS`, `tr808::DEFAULT_PARAMS`,
`tr909::DEFAULT_PARAMS`, `delay::DEFAULTS`, `dist::DEFAULTS`, `pcf::DEFAULTS`, `comp::DEFAULTS`
hold these values, and every device's `SongDevice::init_track` models its slot 8; the mixer's
(0x100160F0) was missing until task-034 (`frame::tests::constructor_tracks_hold_the_defaults`,
`song_mode_chase_restores_the_mixer_ctor_values`).

### 4.10 The panel's live device messages [C] (task-043, task-055)

Besides the data block, each device's host-message handler (slot 10) takes the panel's live
changes, all as msg 41 with the device's own sub id.

**The exe side.** Every edit goes through `sub_40B28C(song, dev, sub, data, n)` →
`sub_423C98`, which builds `[78, song, 41, dev, sub, data…]` (§1); a query through
`sub_40B23A(song, dev, sub)` → `sub_423C51` (no payload), whose reply the caller reads with
`sub_40ABE5`. The pattern edits address the view's current pattern (the 303's `+1277`, the
808's `+1617`, the 909's `+1726`). The senders (callers of `sub_40B28C`; the load's own
`sub_431747`, `sub_438D43` … `sub_43A637` excluded). The "Called from" column is the xrefs; the
polls are the ones `docs/ui_spec.md` read, the other callers' roles (the pattern section's
buttons, Clear, the Edit menu, the tap keys) are from their place and are [G] where not read
in `docs/ui_spec.md`; the ones marked [G: not read] are probably MIDI remote control:

| Device | Sub | Payload | Exe sender | Called from |
|---|---|---|---|---|
| TB-303 | 40 | `{ctrl, v}` | `sub_43EC83` | the poll `sub_43ED76` (the six knobs, the waveform), the pattern section's on/off and pattern buttons (`sub_43D3C0`, `sub_43D3F3`, `sub_43D48F`), `sub_4405B5` [G: not read] |
| TB-303 | 42 | `{p, field, v}` | `sub_43ECCA` | `sub_43D426`, `sub_43D45B` (the pattern section's Shuffle, Steps) |
| TB-303 | 44 | `{p, 98 bytes}` | `sub_43F617` | Clear (`sub_43F5F5`), the Edit menu's pattern functions `sub_42B319`, `sub_42B928`, `sub_42C204` |
| TB-303 | 45 | `{p, step, field, v}` | `sub_43ED1D` | the step editor (`sub_43ED76`, the keys `sub_43F590`), the tap keys `sub_440213` |
| TR-808 | 57 | `{ctrl, v}` | `sub_441C82` | the poll `sub_441E79`, the pattern section (`sub_440C3C`, `sub_440C6F`, `sub_440D0B`), `sub_443C92` [G] |
| TR-808 | 59 | `{p, field, v}` | `sub_441CC9` | `sub_440CA2`, `sub_440CD7` |
| TR-808 | 61 | `{p, 194 bytes}` | `sub_443257` | `sub_42B319`, `sub_42B928`, `sub_42C204` |
| TR-808 | 62 | `{p, step, inst, v}` | `sub_441D1C` | `sub_44305C` (the step buttons, the tap keys) |
| TR-909 | 74 | `{ctrl, v}` | `sub_445C41` | the poll `sub_445FB0`, the pattern section (`sub_4447C2`, `sub_4447F5`, `sub_444B43`), `sub_447D9D` [G] |
| TR-909 | 76 | `{p, field, v}` | `sub_445C88` | `sub_444ADA`, `sub_444B0F` |
| TR-909 | 78 | `{p, 194 bytes}` | `sub_4471C5` | `sub_42B319`, `sub_42B928`, `sub_42C204` |
| TR-909 | 79 | `{p, step, inst, v}` | `sub_445CDB` | `sub_447153` (the step buttons; it also clears the other hi-hat) |
| Delay | 84 | `{ctrl, v}` | `sub_407AD3` | the poll `sub_407F7A`, `sub_40821A`, `sub_40827C` |
| Distortion | 92 | `{ctrl, v}` | `sub_408976` | the poll `sub_408AF5`, `sub_408C36`, `sub_408C98` |
| Compressor | 102 | `{ctrl, v}` | `sub_403FE5` | the poll `sub_404396`, `sub_404736`, `sub_404798` |
| PCF | 114 | `{ctrl, v}` | `sub_41B82B` | the poll `sub_41B914`, `sub_41BE32`, `sub_41BE94` |
| Mixer | 145 | `{id, v}` | `sub_4131BC` | the master poll `sub_412DC7`, the channel poll `sub_413F47`, `sub_413256`, `sub_4137E2` |
| Mixer | 147 | `{c, k, v}` | `sub_413206` | `sub_413F47`, `sub_413256`, `sub_4137E2` |

Queries the panel sends (`sub_40B23A`): the 303's 31 (`sub_43EAC5`) and 33 (`sub_43EA26`), the
808's and 909's counterparts (`sub_442978`, `sub_4428D9`, `sub_446AD7`, `sub_446A38`), from
`sub_42D92C` / `sub_42D9CF`; the effect and mixer views' reads (`sub_403DC1`, `sub_407944`,
`sub_408736`, `sub_41B6AA`, `sub_412F04` with the mixer's 137); and the save path
(`sub_43920E`: 303 31, `sub_438EAA`: delay 83, and their siblings `sub_438BCE`…`sub_43A1F4`).

**The DLL side.** The handlers and their tables (`jmp [table + 4*sub]`, the raw sub byte):
303 `sub_100256E0` (table 0x10153BD8, subs 31..46), 808 `sub_100282B0` (0x10153F74, 48..63),
909 `sub_1002B8E0` (0x101542F0, 65..80), delay `sub_1000FB40` (0x101516A8, 81..86), distortion
`sub_10010D60` (0x10151A34, 89..94), compressor `sub_1000EFD0` (0x1015151C, 99..104), PCF
`sub_10016C50` (0x101520F4, 111..116), mixer `sub_10015520` (0x10151ECC, 137..148). What each
sub that changes state does (`p` pattern, `s` step; offsets in the device object):

| Device: sub (address) | Effect |
|---|---|
| 303: 40 (0x10025CFC) | ctrl 0..8 the live byte `+0xC7C + ctrl`, then the record hook; 9 nothing; 10 the unused live byte `+0xC86`; every ctrl then clears its changed flag `+0xC88 + ctrl` |
| 808: 57 (0x100288C5) | ctrl 0..28 `+0x1890 + ctrl` and the hook; 29 nothing (the last-step byte); 30 `+0x18AE`; clears `+0x18B0 + ctrl` |
| 909: 74 (0x1002BF12) | ctrl 0..29 `+0x1891 + ctrl` with 20, 21, 22 at `+0x18A6`, `+0x18A7`, `+0x18A5` (the permutation of §4.8), and the hook; 30 nothing (the last-step byte); 31 `+0x18B0`; clears `+0x18B2 + ctrl` |
| Delay: 84 (0x1000FCE5) | k 0..4 `+0x34 + k`, raw (orb clamps the pan to 127, below); 5 (the meter) nothing; clears `+0x40 + k`; the hook for every k |
| Distortion: 92 (0x10010EF1) | k 0..2 `+0x39 + k`, raw (orb clamps amount and shape to 127, below); clears `+0x4C + k`; the hook |
| PCF: 114 (0x10016DFB) | k 0..6 `+0x38 + k`; the hook; clears `+0x48 + k` |
| Compressor: 102 (0x1000F178) | k 0..2 `+0x38 + k`; 3, 4 nothing; clears `+0x42 + k`; the hook |
| Mixer: 145 (0x10015900) | id 0 the live master `+0x30` (no hook); 1 the compressor id, dword `+0x34`, and 2 the PCF id, dword `+0x38` (each clears `+0x81` / `+0x82`, then the hook with controller 1 / 2); 3, 4 nothing |
| Mixer: 147 (0x100159DB) | k 0 the live enabled byte `+0x40 + 6c` (no hook: the mute is not automatable); k 1..4 the level, pan, send, dist `+0x41..+0x44 + 6c`, then the hook with controller `5 + 6c + k`; k 5 nothing; c unchecked (OB-17; rb drops c above 3) |
| 303: 42 (0x10025F25); 808: 59 (0x10028EFE); 909: 76 (0x1002C57F) | `{p, field, v}`: field 0 the pattern's shuffle flag, 1 its length (303 `+0x3C`/`+0x3D + 98p`, 808 `+0x50`/`+0x51 + 194p`, 909 `+0x51`/`+0x52 + 194p`); another field nothing |
| 303: 43 (0x10025F62); 808: 60 (0x10028F3B); 909: 77 (0x1002C5BC) | `{p, shuffle, length}`: both header bytes (`sub_100191E0`) [not sent by the exe] |
| 303: 44 (0x10025F93); 808: 61 (0x10028F6C); 909: 78 (0x1002C5ED) | `{p, bytes}`: the whole pattern, 98 bytes (`sub_10018DC0`) or 194 (`sub_100192C0`), as the data block holds it |
| 303: 45 (0x10025FC4) | `{p, s, field, v}`: field 0..5 (note, slide, accent, up, down, rest; the switch's `ja` drops a larger field) at `+0x3E + 98p + 6s + field` |
| 808: 62 (0x10028F9D); 909: 79 (0x1002C61E) | `{p, s, inst, v}`: `+0x52 + 194p + 12s + inst` (808) or `+0x53 + …` (909), inst unchecked |
| 303: 46 (0x10026112); 808: 63 (0x10028FE2); 909: 80 (0x1002C663) | `{p, s, bytes}`: a whole step, 6 bytes (`sub_10019B00`) or 12 (`sub_10019840`) [not sent by the exe] |
| Mixer: 146 (0x100159C2), 148 (0x10015B8B) | the header (`sub_10019D10`) or one 6-byte channel `{c, bytes}` (`sub_10019B00`) into the **live** set [not sent by the exe] |

Notes:
- **The record hook** `sub_10012BC0` → `sub_10021340` acts only with globals byte 0 (song
  mode) and byte 2 (record) set: it sets the track's hold byte `+4 + ctrl` and inserts the event
  into the track's recording list (`sub_10014750` on `+0x198`; the rounding, the loop rule and
  the merge into the track are in `docs/initial_investigation.md` §5.3). [C] task-057: every
  hook call passes the message's own controller and value (e.g. 0x10025D3B..0x10025D57 for the
  303), for the controllers the table names; the distortion's, the PCF's and the compressor's
  call it for every k, the ones their switch does not store included (0x10010F66, 0x10016E8A,
  0x1000F1CE), like the delay's (0x1000FD6C). With record off, a `{ctrl, v}` message is slot 11's store for every
  controller slot 11 takes (303 0x10026180, 808 0x10029050, 909 0x1002C6D0, delay 0x1000FDC0,
  dist 0x10010FB0, PCF 0x10016EF0, compressor 0x1000F220, mixer 0x10015C00), plus the one extra
  live byte of the pattern devices, which no audio reads. The changed flags are the panel
  update's (slot 11 sets them, the message clears them).
- **The patterns are read live.** The step functions read the pattern array in both modes, so a
  pattern edit is heard the next time the step plays, and it does not touch the live parameters
  (a data block in pattern mode resets them to the block's).
- **Bounds.** Only the 303's step field and the header field are checked. A pattern above 31, a
  step above 15 or an instrument above 11 addresses another step, another pattern or, past the
  array, the live block and the rest of the object (303 pattern 32's header is the live
  enabled byte). The panel sends only its current pattern (0..31), steps 0..15 and instruments
  0..11, so this never happens from the panel. A defect of the original:
  [OB-17](original_bugs.md).
- **The queries** reply only, except the whole-block query of each device (303 31 at
  0x10025751, 808 48 at 0x10028321, 909 65 at 0x1002B951, delay 83 at 0x1000FC66, dist 91 at
  0x10010E7C, PCF 113 at 0x10016D80, compressor 101 at 0x1000F0FD), which in pattern mode first
  copies the live parameter block into the pattern-mode copy. That copy is not observable: in
  pattern mode nothing reads the copy's parameters (slot 9 overwrites it from the live block on
  entering song mode, msg 41 overwrites it), so the queries need no model. The mixer's 137
  replies the live set without copying.

orb: `Engine::device_param` (the `{ctrl, v}` subs; `SongDevice::param_message`),
`Engine::mixer_global` (145), `Engine::mixer_channel` (147), `Engine::pattern_field`,
`Engine::pattern_header`, `Engine::pattern`, `Engine::step_field`, `Engine::step`
(`crates/orb-engine/src/frame/edit.rs`), sent by the `orb_player::Command` variants
`DeviceParam`, `MixerGlobal`, `MixerChannel`, `PatternField`, `PatternHeader`, `Pattern`,
`StepField`, `Step`. The controller messages call the record hook as the DLL's do
(`frame::edit::hooks`, `Sequencer::record_hook`, task-057). Where orb differs from the
DLL's stores:
- **Delay pan and distortion amount/shape above 127.** The DLL's subs 84 and 92 store the byte
  raw; orb's slot 11, which `Engine::device_param` uses, clamps the delay's pan
  (`delay.rs:74-76`) and the distortion's amount and shape (`dist.rs:116-118`) to 127, because
  the DLL's updates then index past their tables (OB-9, OB-10). The panel sends 0..127 only, so
  the messages render alike. The same clamp is on the load path (msg 86 and 94,
  `Delay::data`, `Distortion::data`), and there a song file can carry such a byte: the exe's DELY
  and DIST parses (`sub_43465C` at 0x43589A, `sub_4345A6` at 0x435B28) and their packers
  (`sub_422EB7`, `sub_423048`) copy the chunk bytes unchanged [C]; a TRAK event cannot (the
  track load's range is 0..127). A hand-made file with such a byte therefore renders
  differently in orb; no shipped or corpus song has one. Whether to keep the clamp is task-071.
- **Mixer channel above 3.** Sub 147 does not bound c (OB-17); `Engine::mixer_channel` drops a
  channel above 3 (`edit.rs`), and so do the pattern messages past the pattern array.

Checked against the DLL: a
600-call session over `full_mix` with 135 of these messages, in pattern mode and in song mode
while playing, rendered by rbhost (`--live`, `tools/rbhost/README.md`) and by orb-player, is
byte-identical (`crates/orb-player/tests/live_messages.rs`). The one difference that session
work found is not a message: orb's PCF step after a return to pattern mode while playing
(task-069).

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
DLL inserts them with `sub_10021010` (song msg 37, 0x100127EB; each in tick order, section 12.6), which refuses a batch longer than
the node pool's free count (section 5.2).

**The 808 track before 4.2 (`sub_436F5A`) [C] (task-018).** The tom tuning `v/2 + 32` (`sub_432E1B`,
called at 0x437139 for controllers 10, 13 and 16) is applied **before** the range check:
`sub_42FD00` at 0x4371A3 gets the tuned byte. A file value of 190 thus becomes 127 and is accepted,
and 192 becomes 128 and is dropped. The rest equals `sub_436B6F`. ([OB-2](original_bugs.md).)

### 5.1 The mixer track of a 4.0 file (`sub_437380`) [C] (task-018)

This is checked in the disassembly (the switch at 0x43764E, jump table 0x437C73). The old mixer
track has 8 controllers per channel c, starting at `4 + 8c`. Each event is remapped before the
validation:

| Old controller | New controller | New value |
|---|---|---|
| 0 | 0 (not accepted) | unchanged |
| 1 (master compressor flag) | 1 | 1 if the value is non-zero, else see below |
| `4 + 8c` | `5 + 6c` (not accepted, so the event is dropped) | unchanged |
| `5..8 + 8c` (level, pan, send, dist) | `6..9 + 6c` | unchanged |
| `9 + 8c` (PCF flag) | 2 | `c + 1` if non-zero, else see below |
| `10 + 8c` (compressor flag) | 1 | `c + 2` if non-zero, else see below |
| 2, 3, 11, 19, 27, above 34 | unchanged | unchanged |

Unchanged controllers then pass the normal checks, so an old 19 or 27 lands on the 808 pan or the
909 dist of the new numbering.

**Flag off.** A flag event of value 0 becomes value 0 only if its tick is greater than the tick of
the last event of the same kind: compressor (`var_18`) or PCF (`var_14`). Both start at 0 and are
compared signed (`jg`). Otherwise the event is dropped. Every compressor or PCF flag event sets its
tracker to its tick, the dropped ones excepted. The trackers are updated in the switch, before the
tick check `sub_42FD60` (0x437A32), so an event whose tick is later found invalid still updates
them. So an off event at tick 0 is always dropped. After the switch the checks of `sub_436B6F`
follow (0x437A28..0x437A8D): the running position advances on every valid tick, even when the event
is then dropped.

### 5.2 A batch the node pool refuses [C] (task-083)

**DLL.** `sub_10021010` reads the header (`sub_10019EB0`), takes `count = last - first` (`sub ebx,
edx` at 0x10021042) and the free count of the node pool `dword_10100230` (`call sub_1000DEE0` at
0x10021054, whose body is `mov eax, [ecx+4]; ret`; `docs/initial_investigation.md` section 5, the
pool of 100000 nodes shared by the whole engine instance). When `count > free` (`cmp ebx, eax; ja` at 0x1002105B, unsigned) it
inserts nothing and replies `sub_1001DE00(22)` (0x100210ED): the packet `['K', 22]` (`sub_10018110`
with type 0x4B and the one byte 22). Otherwise it inserts every event (`sub_10014750`, the loop
0x1002106F..0x100210C8) and replies `sub_1001DF80(0x49)` (0x100210CA): the one-byte packet `['I']`
(`sub_10018100`). The check counts the header, not the nodes the batch will take: a replaced value
takes no node and still counts, and a batch of exactly `free` events passes and then loses its last
insert to `sub_10014434`, which takes a node only while more than one is free.

**Exe.** `sub_436B6F` compares the reply's first byte with 0x49 (`'I'`, 0x436E52: go on) and 0x4B
(`'K'`, 0x436E65) and on `'K'` throws `XOutOfMemory` (0x436E95; the 1.x path `sub_430163`:
`cmp eax, 0x4B` at 0x430305, the throw info pushed at 0x430315). Its `catch(...)` (0x436EEA) deletes the TRAK
stream and rethrows, and so does `sub_434D05`'s (0x436AD7, the rethrow at 0x436B20). `sub_425C4D`
catches it (0x426430 for the RBS branch, try at 0x4263B2; 0x425FDB for the 1.x branch) and throws
`XSongOutOfMemory` (vftable 0x4BBD9C). No further batch is sent, and neither is msg 32 (0x426610).
`sub_40B575` catches that (0x40B778): `sub_475D5C(0x1DCF592A)` shows a `CAlertBox` (`sub_4754B9`)
with the text of id 500128042 (`sub_47B074`: group 128, item 42 of the exe's `SALR` text, file
offset 0x159897):

> There is not enough memory for opening the Song. [OK]

then it rethrows. The unwinds delete the half-built document (`operator new(0x338)` of
`sub_40B575`) and the streams; the song window (`sub_42DA72`) is never built. File > Open
(`sub_40A2A3`, its `catch(...)` at 0x40A3D4) swallows the exception and returns no document. So the
user gets that alert and no song.

**The DLL's song stays.** None of these unwinds (`sub_436B6F`, `sub_434D05`, `sub_425C4D`: its
member destructor 0x425900 is empty, `sub_40B575`, `sub_40A2A3`) sends a message to the engine, so
the song slot the engine gave at msg 52 (`sub_425A5D`) is never closed: it keeps the nodes of the
batches before the refused one, and every later load has that many fewer free nodes
([OB-20](original_bugs.md)).

**When it happens.** Only for a song of about 100000 events or more. The free count at a load is
100000 less the nodes of every song the engine already holds and of the new song's constructor:
in the app at least the "Untitled song", which an opened song replaces (`docs/export_state.md`
section 3, msg 53), with its constructor's tick-0 events, which take pool nodes
(`docs/initial_investigation.md` section 5, the node pool, task-082). [G] How many nodes that is,
and whether other lists hold nodes then, is not counted here. An RBS track is sent
in batches of 1284 file events, track after track; a 1.x song's track buffers are sent as they fill
(section 12.2), then what is left of them in track order.

**orb.** `load::load` and `load::load_1x` keep the send order and each batch's count in
`song::Song::batches`; `Engine::load` sends the tracks in those batches and
`Sequencer::add_events` refuses a batch as `sub_10021010` does. A refused batch ends the load with
`Err(LoadRefused)` (also `Engine::load_refused`): the engine keeps the earlier batches, as the DLL
does, and sends no msg 32; `orb render` fails with the alert's text. orb's pool is one per song
(task-082), so orb's free count at a load is that of an engine with no other song: an RBS song the
app refuses because the Untitled song holds nodes can still load in orb. The player and the UI do
not act on a refused load yet (task-088).

## 6. What decides the state at an export [C]

The song's global byte 0 (song mode) decides:

- **Pattern mode:** the device chunks (msg 41) are the live values; tracks are not chased.
- **Song mode:** locate (`0x1001F984`) chases every track: `sub_100151F0` fills a 128-entry state
  with 0xFF, then sets each controller to the value of its last event with `tick <= P`; every entry
  that is not 0xFF is applied through `sub_10012B40` (device setParam). No automatable controller
  (§4.9; for the mixer 1, 2 and `6..9 + 6c`) is left at 0xFF: the song ctor put a tick-0 event
  of each into its track (§4.9), so a controller the song never sets before `P` gets its
  constructor default, not the device chunk (§4), and
  this also holds after the position moved past a later event and came back (loop wrap, locate).
  Only the mixer master and channel enables, which have no controller, come from the chunk. Song
  mode therefore takes every automatable value from TRAK or the constructor.

Every shipped song has an event at tick 0 for every automatable controller of every track (for the
mixer 1, 2 and `6..9 + 6c`; controllers 10, 11, 16, 17, 22 and 23, a channel's unused byte and
enable, get no ctor insert, and setParam ignores them, §5), so for them
the TRAK values at the loop start are the whole automatable state; the chunks still provide the
patterns, the mixer master and enables, and the bytes no controller reaches. The export's locate to
the loop start (msg 35 `{8, start}`) runs this chase.

Load itself: msg 36 sets the mode (song mode chases the still empty tracks) and locates to tick 0;
the device data follows; msg 32 at the end re-locates to the current position (tick 0), which chases
the tracks just loaded.

[C] task-022: the DLL keeps each track as a sorted list (`sub_10014750`, investigation §5, "The
track list"). Two consequences for the stream: the events of one tick are applied in the reverse of
the order msg 37 delivered them, and a later event with the same tick and controller replaces the
earlier one; an event whose tick exceeds 767231 (bar 999's last tick; `cmp 0BB4FFh` at 0x1001477B)
is dropped there. The exe's own check (`sub_42FD60`, §5 "Validation") accepts multiples of 24 below
768000, so it can send ticks up to 767976; the DLL drops those above 767231 (inferred from the two
limits; no corpus song has such an event, the largest corpus tick is 417792; [OB-3](original_bugs.md)). The batch boundaries
do not change the result: each batch starts its insert hint at null (`mov edi, 0` at 0x10021061),
and `sub_10014750` uses a hint only when its tick is below the new tick, so its walk ends at the
same node as a walk from the head of the sorted list. Every export's first stop
also prunes the tracks (`sub_100149F0`): an event repeating its controller's previous value is
removed unless it is at tick 0. `crates/orb-engine/tests/corpus_seq.rs` checks the load's resulting
sequencer and mixer state over the corpus.

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
  PID), so each page holds constants that cancel them. [C] task-030 checked one family on the real
  `RB20FUL.DAT`: the parent-window page `unk_4DAD78` is read at
  `256 + ((((GetWindowLongA(hwnd,-8)+1) & 0x3FFF) + 0x4000) << 11)`; for each of its 16384
  candidates `k`, the word at +0xE0 (`dword_4DAE58`) gives `((g & 0xFFFFF000) >> 14) - k = 18` and
  the word at +0x3AC (`dword_4DB124`) gives `((g & 0xFFFFF000) >> 12) - k = 8`. So the expression
  `X = ((dword_4DAE58 & 0xFFFFF000) >> 14) - ((api + 1) & 0x3FFF)` is 18 in every untampered run
  (the RBM key stride `X - 14 = 4`, the dist index `X / 18 = 1`, docs/rbm.md section 5.2;
  reproduce with `tools/rbm.py check-dat RB20FUL.DAT`);
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

Every engine-side point of task-003 is now answered:

- [C] Top-level msg 52/58 handling inside the DLL: msg 52 is queued (`sub_1001D694`) and run by
  `EngineIdle` (`sub_1001D840`), which builds a new song with new device objects; 58 starts the
  startup-sound preview (task-004, `docs/export_state.md` §3, §8).
- [C] The constructor default of every automatable controller, and the ctor's tick-0 track
  events that make it the song-mode value of a controller the song does not set (§4.9,
  task-034).
- [C] The msg-41 song-mode rule for all nine devices (§4: the mixer and the 303 task-003/022, the
  808 task-024, the 909 task-025, the delay and the distortion task-026, the PCF and the
  compressor task-027).
- [C] The 4.0 mixer-track converter `sub_437380` (§5.1, task-018).
- [C] Global byte 2 (msg 35 id 2, `0x10021194`) is record: it makes the loop wrap merge the
  recorded events (`sub_1001F520`) and locate apply events forward instead of chasing (task-022,
  investigation §5). Nothing in the load or the export sets it. The recording itself: task-057,
  investigation §5.3. The exe's save writes the tracks it reads back from the DLL with song msg
  22 (`sub_437CFF`, 0x437DE6), so a saved song carries the recording and the ctor's tick-0
  events (every shipped song has exactly those: 18, 9, 9, 29, 30, 5, 3, 7, 3 events at position
  0 in tracks 0..8).
- [C] Byte 22 (`+0x16E`) has no effect on the engine (§3, task-034).

Still open:
- [G] task-061: two 4.0 songs (RB-SEMx, TB-909e mkII b7x7) rendered by the original DLL through
  rbhost (messages from `tools/rbhost/rbsmsg.py`, a second port of 0x4352C0 and `sub_437380`
  written from this section) equal `orb render` byte for byte: the Rust and Python ports agree
  and the DLL accepts their messages. The app's own export of a 4.0 song was not run, so the
  conversion is not checked against the exe itself.
- [G] What the exe uses byte 22 for (it always sends 1, and the load sends no msg 21 that would
  read it back, §2).

## 12. ReBirth 1.x songs: the MIDI-file import (task-060) [C]

ReBirth 1.x saved its songs as a Standard MIDI File with the `.rbs` extension (`MThd`). 2.01
opens them through a separate reader, traced in `Rebirth.exe.i64` (a read-only copy,
2026-10-27). orb: `crates/rbs/src/rb1x.rs` (the file), `crates/orb-engine/src/load/rb1x.rs`
(`load_1x`, the import), `tools/rb1x.py` (the Python twin, which `tools/rbhost/rbsmsg.py`,
`run.py`, `fit.py` and `tools/rb_export.py` use for these songs).

### 12.1 Detection and version

- `sub_425C4D` calls `sub_43037D` (0x425DE4) before anything else: it opens the file as a MIDI
  file (`sub_4A0B38`, the tags `MThd` / `MTrk` at 0x4D2AB0 / 0x4D2AB8) and reads the version with
  `sub_431341`. That walks track 0 up to the first meta text event `FF 02` (or the end of track),
  takes at most 200 bytes, `sscanf(text, "File version %s")` (0x431481) and parses the word with
  `sub_4718F6`: the number before the first `.` is the major, the one after it the minor (both
  must be below 100, else `XBadFormat`). With no text the version is `CVersion(2, 0)`.
- 0x425E01: `sub_47DA1C(file, &unk_4DAD60)` is `key(file) >= key(3.1)` (`unk_4DAD60` is
  `CVersion(3, 1)`, built by `sub_4257F3`); an older file throws `XUnsupportedFormat` (0x42613B).
- The song gets the built-in mod `dword_4DA784` (0x425E17; no Mods lookup, no dialog), then the
  same messages as an RBS load around the reader: top-level 52 (0x425F98), the optional 58
  (`sub_426C67`, 0x425FAB), the import `sub_430BA0` (0x425FCE), and msg 32 (0x426610).
- The collection (`~/rb_stuff/songs`, task-058): 129 distinct 1.x songs, 124 with "File version
  3.1.0 (c)1997 Propellerhead Software, all rights reserved" and 5 with 3.2.0. All have MIDI
  format 1, division 32 and six tracks: 0 the version text, tempo and time signature metas; 1 the
  song sysex; 2..5 controller changes on channels 0..3.

### 12.2 The track walk (`sub_430BA0`)

- `sub_4A0C12` reads the header; the format must be 1 (0x430CBC, else `XBadFormat`).
- Nine track buffers, one per engine track 0..8 (`sub_43005E`, `0x282C` bytes each: 1284 events of
  `{u32 tick, ctrl, value}`). `sub_4300E0(tick, ctrl, value)` appends one and, when the buffer holds
  1284, sends it as one msg 37 (`sub_430163`: `[track, u32 first, u32 last]`, then `[99, u32 tick,
  ctrl, value]` per event, the reply read and `'K'` thrown as out of memory). **No tick, controller
  or value check**: unlike section 5, every event goes to the DLL, whose insert drops ticks above
  767231 (section 6). At the end the buffers are sent in order 0..8 (`sub_43035E`, 0x4310E9).
- The first event, before any track is read, is `(0, 0, 0)` in the 909 buffer (0x430CFA, `ecx` =
  `var_2C` = buffer 4): the 909 is switched off at tick 0. 1.x has no 909.
- MIDI tracks 1 .. n-1 are walked in order (track 0 is not; its tempo meta is ignored). Times are
  absolute (`sub_4A0E8A` accumulates the deltas; the running status is kept, also across a track
  seek). The header's division is overwritten with 8 (0x430D72) and `tick = 192 / 8 * time`
  (`idiv`, `imul`, u32), so **tick = 24 × time** whatever the division says.
- An event whose `status & 0xF0` is 0xB0 (a control change on **any channel**; the channel is
  ignored, the MIDI track number decides) is read as `(cc, value)` (`sub_4A0F24`):
  - On track 5, cc 17 (0x430E47) is the distortion's channel: four mixer events at the tick,
    controller 9 = (v == 0), 15 = (v == 1), 21 = (v == 2), 27 = 0.
  - On track 5, cc 31 (0x430ED2, switch at 0x430EFF) is the PCF's channel: mixer controller 2 =
    1, 2, 3, 0 for v = 0, 1, 2, 3, and 0 for any other v.
  - Any other cc goes to device `sub_43067F(track, cc)` as controller `sub_430847(track, cc)`:

    | MIDI track | cc | Engine track, controller |
    |---|---|---|
    | 2, 3, 4 (303 #1, 303 #2, 808; channel c = track - 2) | 7, 10, 12 | mixer `6 + 6c`, `7 + 6c`, `8 + 6c` (level, pan, delay send) |
    | 2, 3, 4 | 14 | the device, 1 (pattern) |
    | 2, 3, 4 | 15 | the device, 0 (enabled) |
    | 2, 3 | 16..22 | the 303, 2..8 (tune, cutoff, resonance, envmod, decay, accent, waveform) |
    | 4 | 16..42 | the 808, 2..28 |
    | 5 | 16, 18 | distortion 1 (amount), 0 (on) |
    | 5 | 19, 20, 21, 22, 23 | delay 1 (steps), 3 (feedback), 4 (pan), 0 (on), 2 (mode) |
    | 5 | 24..29, 30 | PCF 1..6 (frequency, resonance, amount, wave, decay, mode), 0 (on) |

    On the 808, controllers 10, 13 and 16 (the tom tunings) get `sub_43132C` = **`v/2 + 64`**
    (0x430FCB; bytes `99 2b c2 d1 f8 83 c0 40`: a signed halving, then + 0x40), not the `v/2 + 32`
    of the pre-4.2 RBS files (section 5).
  - A cc that neither table maps gives device -1, and 0x430FEF loads the buffer pointer
    `[ebp + 0xFF*4 + var_3C]`, far outside the nine-pointer array: [OB-15](original_bugs.md).
    No song of the collection has one; orb refuses such a song (`LoadError::Rb1x`).
- A sysex (`F0`) goes to `sub_4304C5` (below); `FF 2F` ends the track; every other event is
  skipped by its length (`sub_4A0D77`). The collection has only control changes on tracks 2..5.

### 12.3 The song sysex (`sub_4304C5`)

The data (`F0` and the bytes after it, length from the VLQ) must start `F0 00 00 66`
(`sub_4A14AD`) with byte 5 non-zero, byte 6 = 1 and byte 7 = 0 (0x430546..0x430571); the
collection's files have `F0 00 00 66 00 15 01 00`. From byte 8, `sub_4665CD` decodes groups of 8
bytes into 7 (`sub_4663F0`: byte k gets bit 7 from bit `6 - k` of the group's 8th byte) until the
block is full. The block size is `0x4000 - (((dword_4DA3F0 & 0xFFFFF0) >> 5) - (X & 0x123))`
(0x430577..0x4305A3, `X` from `sub_48CD4A`): copy-protection arithmetic like section 9, whose
intended value is 0x4000; the decode reads `8 × ceil(0x4000 / 7)` = 18728 bytes whatever the
sysex length ([OB-16](original_bugs.md)). The collection's sysex data is 18729 bytes (the groups and
`F7`). A sysex that fails the header check is skipped by its length although `sub_4A0F79` has
already read it ([OB-16](original_bugs.md)).

### 12.4 The decoded block (`sub_431747`)

`sub_47DB95` reads a version from bytes 0..3 (it returns 0 unless byte 0 is 0xBC; the result is
not checked; `BC 03 01 00` in the collection); bytes 5..8 must be `TinY` (else `XBadFormat`).
Multi-byte values are big-endian (`sub_465FCA`). The messages are sent in this order, each with
the packers of section 4 (same layouts):

1. **Msg 36** (from `sub_420A81`'s defaults): byte 0 song mode = `[0x12] != 0`, 3 loop on =
   `[0x2E] != 0`, 4..7 tempo = u32 `[0x2A]`, 12..15 loop start = `768 × u32[10]`, 16..19 loop end
   = `768 × (u32[10] + u32[14])` (a start bar and a length in bars; `imul ecx, 300h` at
   0x431855 / 0x431861), 20 shuffle = 64 (the default; the 1.x song has none), **21 = 1**
   (0x43186A, `mov [ebp+var_2B], 1`: the **Vintage** sound model, section 3), 22 = 1.
2. **Mixer** `0/143`: master `[9]`, compressor id 0, PCF id from `[0x40]` (0..2 → 1..3, 3 → 0,
   others leave 0; switch at 0x431920); channel j = 0..2 from the device record `r = 0x100 + 32j`:
   enabled `r[0x17]`, level `r[2]`, pan `r[3]`, delay send `r[4]`, dist = (`[0x37]` == j);
   channel 3 (the 909) keeps `sub_423560`'s defaults (1, 100, 64, 0, 0).
3. **Delay** `5/86`: on `[0x35]`, steps `[0x2F] & 0x7F`, mode = bit 7 of `[0x2F]`, feedback
   `[0x33]`, pan `[0x34]`.
4. **Distortion** `6/94`: on `[0x38]`, amount `[0x36]`, shape 0.
5. **PCF** `7/116`: on `[0x3F]`, frequency .. mode `[0x39..0x3E]`.
6. **TB-303 #1, #2** `1/39`, `2/39` (j = 0, 1): enabled = (`r[5]` == 0), pattern `r[1]`, tune ..
   waveform = `p[16..22]` with `p = 352 + 4096j`. Pattern lengths from `r[7..22]`: pattern 2k is
   the high nibble of `r[7 + k]` plus 1 (`sub_466B80`), pattern 2k + 1 the low nibble plus 1
   (`sub_466B93`). Shuffle flags 0 (`sub_420EB4`). Step s of pattern q is the 4 bytes at
   `p + 30 + 68q + 4s`: note = byte 0, slide / accent / up / down = bits 0..3 of byte 2
   (`sub_4671F2`), rest = byte 3; a note above 12 becomes `note - 12` with up set.
7. **TR-808** `3/56` (record `0x140`, `p = 8544`): enabled = (`r[5]` == 0), pattern `r[1]`,
   controllers 2..28 = `p[16..42]`, the tom tunings 10, 13, 16 (`p[24]`, `p[27]`, `p[30]`) through
   `v/2 + 64`. Lengths as the 303's. Pattern q is 64 bytes at `p + 48 + 64q`, unpacked by
   `sub_43285C` into 256 two-bit values (high bits first); step s of instrument column i is value
   `s + 16i`, sent as it is (0..3; the collection has only 0 and 1).
8. **TR-909** `4/74 {0, 0}` and no data block: DLL sub 74 (`sub_1002B8E0`'s table at 0x101542F0,
   entry 0x1002BF12) stores the live byte `+0x1891 + ctrl` and calls the record hook
   `sub_10012BC0`, so with record off it is slot 11 with controller 0: the live 909 enabled byte
   is 0. Checked in the installed `ReBirth.dll` with `llvm-objdump` and the file bytes: the guard
   0x1002B932..0x1002B942 (`movl $0xffffffbf, %ecx; addl %edx, %ecx; cmpl $0xf, %ecx; ja
   0x1002C69C`) lets only sub bytes 0x41..0x50 (65..80) reach the table; then 0x1002B948
   `movl 0x101542f0(,%edx,4), %edx; jmpl *%edx` indexes it with the raw sub byte; the
   `.data1` dwords at 0x10154414 and 0x10154418 (entries 73 and 74) are 0x1002BE90 (the data
   block, section 1) and 0x1002BF12; there `movb 0x1(%eax), %dl` takes the controller (0..31,
   table 0x101544AC, whose entry 0 is 0x1002BF46), `movb 0x2(%edx), %al` the value, and
   0x1002BF49 `movb %al, 0x1891(%ebx)` stores it before `calll 0x10012bc0`. The 909's patterns and pattern-mode block stay the constructor's (16 empty steps, the
   defaults of section 4.9).

There is **no compressor block**: the compressor keeps its constructor state, which equals the
exe's defaults (section 4.9). Bytes from 12640 (`0x3160`) hold the song information,
`sub_42FFEC`: the title (40 characters at +0), the text (200 at +41), the web page (100 at +242)
and the show-on-open byte (+343), which `sub_430BA0` copies into the document's USRI fields
(0x431039..0x431078); no message.

### 12.5 What orb does and how it was checked

`orb_engine::song::LoadForm::Rb1x` marks such a song: `Engine::load` sends the seven blocks in the
order above, then the 909's controller 0 = 0, then the tracks (in the buffers' order, which
`Loaded::order` keeps for the message stream). A song the exe would refuse (format other than 1,
version below 3.1, a bad sysex, no sysex) or one that reaches OB-15 is an error.

- The env-gated `crates/orb-cli/tests/thirdparty.rs` renders every 1.x song of the collection and
  compares `load_1x`'s message stream with `tools/rb1x.py`'s: 129 of 129 render, 129 equal
  (`crates/orb-engine/tests/rb1x_messages.rs` does the same on synthetic songs).
- rbhost (the original DLL fed `tools/rb1x.py`'s messages, idle 0) against `orb render
  --idle-blocks 0`: **129 of 129 equal** byte for byte (`tools/rbhost/hashcmp.py`, the WAV
  sha256 taken on the machine, 2026-10-27; the per-song hashes are in the task-060 notes), the
  two pattern-mode songs included (rbhost exports them; the app cannot).
- The app itself (a copy of the installation, `tools/rb_export.py --app-dir --restore-prf
  --startup-sound off`) exported 11 song-mode 1.x songs (Razor Topz, Eternal Darkness, i feel
  love, Virus 303, MagneticFields, Galaxy Defender, pre2, DISFUNKT (3.2.0), Venom Surf, speed,
  Gizmo). The exports of Eternal Darkness and DISFUNKT are silent, so they equal a render at
  any N and confirm little. Of the 9 audible ones, **8 match at a unique N** (`fit.py --all` over
  200 candidates: 3536..3616, `N - N_t` = -42 .. -246), and `orb compare` at that N is EQUAL for
  each. Venom Surf matches too, but not uniquely: at 80 of the 200 candidates (2000..5184), so
  its fit pins no N and is weak evidence. The 8 unique fits, with the byte-exact 129 of 129
  rbhost result above, show that the message stream of this section is what the app sends, end
  to end, for those 8 songs.
- [G] The app showed no song-information box on opening `cut this air.rbs`, whose show-on-open
  byte is 1 but whose title, text and web page are empty (so `rb_export.py`, which waits for the
  box when the byte is set, timed out on it); the rule the app uses was not traced.

### 12.6 The track order, and the save of a 1.x song [C] (task-125)

**The import's buffers are not in tick order.** Each buffer of section 12.2 is filled in the
order of the MIDI tracks: the 909's starts with `(0, 0, 0)` (0x430CFA), and the mixer's takes the
level, pan and send events of MIDI tracks 2, 3 and 4 and then the distortion and PCF channel
events of MIDI track 5 (0x430E47, 0x430ED2), each MIDI track in its own time order. So a mixer
event of MIDI track 5 at an early tick follows one of MIDI track 2 at a later tick, in the buffer
and in the msg-37 batch.

**The DLL sorts as it inserts.** Msg 37 (`sub_10021010`) inserts each event of a batch with
`sub_10014750`, in the batch's order:
- The tick is quantised (`sub_100146E4`), and dropped above 0xBB4FF (0x1001477B).
- The walk 0x10014794..0x100147A2 (`cmp eax, [next+4]; jg`) moves on while the new tick is greater
  than the next node's, so it stops before the first node whose tick is not below the new one.
- Among the nodes of that tick it looks for the same controller (+8, 0x100147B4..0x100147E3); one
  found takes the value (+9, 0x10014820).
- Otherwise a node from `sub_10014434` (0x1001480F) is linked in at that place (0x10014818..0x1001481C).

So every list is sorted by tick whatever the order of the batches. Among the events of one tick, a
new controller goes before those already there, so a later insert comes first, and a repeated
(tick, controller) keeps its place and takes the last value. orb's engine stores them the same way:
`seq::insert_into` (`partition_point(|e| t > e.tick)`, the replace on tick and controller), and the
1.x renders equal the DLL's (section 12.5, 129 of 129 through rbhost).

**What the original saves.** The TRAK writer `sub_437CFF` asks the DLL for each track in batches
of 1284 (song msg 22 `{track, from, from + 0x504}`, 0x437DE6) until a reply is shorter
(0x437EF4). The DLL's reply `sub_10020EC0` walks the list in order (`ebp = [ebp]`, 0x10020FB3),
with the tick at +4 (`sub_1000DEE0`), the controller at +8 and the value at +9. For each event the
writer computes `pos = (tick << 3) / 0xC0` (`shl eax, 3; cdq; idiv`, 0x437E93..0x437E9E). It writes
`pos` less the position before it (`var_20`, 0x437EA4) as a VLQ (`sub_4A10CC`), then the
controller and value. The event count (`var_18`) is written at the chunk's start afterwards
(0x437F25..0x437F49). The lists are sorted, so no delta is negative. A 1.x song saved by the
original is a valid 4.2 file with the DLL's lists: sorted by tick, the song constructor's tick-0
events of every controller included (section 4.9), and the DLL's order within a tick. This is not a
defect.

**The reopened file.** An RBS load inserts the file's events in file order (section 5), so each
tick's events come out reversed, except where an event replaces a node already there. At tick 0
every event of a 1.x import replaces a constructor node, because the import's controllers are all
among the constructor's. So a save and a reopen give the import's lists with each tick's events
reversed, tick 0 excepted. The original does the same.

**orb.** orb-ui gives a 1.x song's document the tracks of the engine it built
(`Link::loaded_tracks`, `document::set_engine_tracks`, `App::adopt_as`), which is what msg 22
would read back right after the load. It does not copy `load_1x`'s buffers, which made
`rbs::Track::build` refuse the save ("events not sorted by position"). The mod does not matter:
`compose` takes only the mod's name, and the save failed without a mod change too. Checked by
`crates/orb-ui/tests/menu.rs`:
- `a_1x_song_with_unsorted_imports_saves_and_reopens_the_same`: a synthetic song.
- `every_1x_song_of_the_collection_saves`, gated on `ORB_THIRDPARTY_SONGS_DIR`: 131 files (129
  distinct) save, and reopen with the import's lists in the order above. Razor Topz.RBS plays the
  same, 200 engine calls, after the save, and saves after a change to Pitch Black Edition 2.0.

**The song information is saved [C] (task-123).** `sub_430BA0` builds a record from the decoded
block's bytes at 0x3160 (`sub_42FED6`, `sub_4304C5`, `sub_42FFEC`) and `strcpy`s it into the
document's USRI fields: the title to +0 (0x431039), the text to +0x29 (0x43104F), the web page to
+0x1BA (0x431067) and the show-on-open byte to +0x21F (0x431078, `neg/sbb/neg` in `sub_42FFEC`
0x430043..0x43004D makes it 0 or 1). `sub_42FFEC` copies each string with `sub_465D8D(src, max,
dst)` with max 0x28, 0xC8 and 0x64 (block offsets +0, +0x29, +0xF2): bytes up to the first zero or
until `max` bytes are written; after a 0x0d byte it writes a 0x0a as well (0x465DDA..0x465DED)
unless `max` is reached, and that byte counts. So a saved 1.x song keeps its title, text (with
CR LF where the file had CR) and web page in its USRI chunk, and it has no copyright (the HEAD is
the constructor's).

**orb.** `orb_cli::info::Usri1x::of` does that copy (`usri_copy`); `document::source_of_1x` puts it
into the stand-in song's USRI, so Save writes it, and `orb info` and File > Info show it
(`SongFacts::of_1x`). Checked by `file_info_of_a_1x_song_shows_its_own_facts` and
`song_information_of_a_1x_song_is_copied_like_the_original` in `crates/orb-ui/tests/menu.rs`.

[G] The original's Save of a 1.x song was not run on the test machine. What it writes is read
from the disassembly above. That orb's lists equal the DLL's after a load rests on the msg-22
comparison of task-057 (RBS songs) and on the 1.x renders.

## 13. The song information and the copyright (task-117)

Read from the bytes of Rebirth.exe with `llvm-objdump` (the idalib MCP tools were not available
for this task): a scan of the file for every copy of each string and every 4-byte copy of its
address, then the disassembly of each place found.

**The copyright is not checked, and not kept [C, the read and write paths].**
- The exe holds the string at 0x4BC010 (`(c)1997 Propellerhead Software, all rights reserved`, no
  period), at 0x4BB8C8 and 0x4BC058 (the same with a period) and twice in `.rsrc` (version
  resources, no reference). ReBirth.dll has no copyright string.
- The six references to the `.rdata` strings are each the copyright argument of the HEAD
  object constructor `sub_465AF6(magic, version, copyright)`, which copies the string to the
  object's +0x10: 0x4109D1 and 0x41110E (magic 0x3B445B64, the RBRT file), 0x4380AE (the RBS
  writer `sub_438025`: magic `[T[T`, CVersion(4,2) at 0x4DB918, string 0x4BC010), 0x43AA8A (magic
  from 0x4CF1A4, string 0x4BC058), 0x41C22E and 0x41CAC2 (magic 0xFFEFEFE5, string 0x4BB8C8).
  None is a comparison.
- HEAD's serializer `sub_465B9D` writes the copyright at +9 with `sub_465D0E(dst, 0x80, src)`
  (0x465BEE); its parser `sub_465BFC` reads it with `sub_465D8D(src, 0x80, dst)` (0x465C39) into
  the object's +0x10.
- A song open reads HEAD twice, each time through `sub_465AA2` (construct and parse) into a
  stack object that is destroyed (`sub_465B87`) after the checks: the peek `sub_4349C7` at
  0x434B26 (object at `ebp-0x170`) checks the magic (0x434B2F) and copies the version bytes
  (0x434B77..0x434BB0, destroyed at 0x434BBD); `sub_434D05` at 0x434ECC (object at `ebp-0x188`)
  checks the magic (0x434ED5) and the major version (0x434F11..0x434F49, `sub_47DB0B`), destroyed
  at 0x434F93. Neither reads the object's copyright (`ebp-0x160`, `ebp-0x178`): no instruction of
  0x4349C7..0x436000 addresses either.
- So a song with any copyright opens without a message, and the original's Save writes its own
  string (0x4380AE) whatever the file had.
- [G] That no other code reads a song's copyright: without IDA's cross references, a pointer made
  at run time cannot be ruled out; no other 4-byte copy of any of the string addresses is in the
  file.

**The limits of the USRI strings [C].** The USRI reader `sub_434955` copies the title with
`sub_465D8D` max 0x28 (0x434960), the text with max 0x190 (0x434975) and the web page with max
0x64 (0x434993); the writer uses `sub_465D0E` with the same 0x28, 0x190 and 0x64
(0x4348E4..0x434911). `sub_465D8D` copies up to the zero or `max` bytes, and after a 0x0D it
writes a 0x0A as well, which counts toward `max` (0x465DDA..0x465DED); `sub_465D0E` zeroes
`max + 1` bytes and copies up to `max` bytes, skipping 0x0A (0x465D55). So the original holds the
text with CR LF in at most 400 bytes, and a file's text whose bytes plus line breaks pass 400 loses
its tail when the original opens it (and a Save of it writes the shortened text;
`docs/original_bugs.md` OB-21).

**orb** (File > Info, `crates/orb-ui/src/info.rs`, `docs/ui_spec.md` 18): the title, text, web
page and copyright are editable with the limits 40, 400 (a line break counting two, so the
original reads the text whole), 100 and 128 bytes. orb's Save writes the copyright the document
holds; a note next to the field says that the original ignores it on open and writes its own on
Save.

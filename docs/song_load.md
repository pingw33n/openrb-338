# Song load: from `.rbs` to engine messages

How Rebirth.exe turns an `.rbs` file into messages to the engine in ReBirth.dll, and what the engine
does with them. Traced for task-003 in `ida/Rebirth.exe.i64` and `ida/ReBirth.dll.i64` (2026-10-26).
`0x4xxxxx` is the exe, `0x10xxxxxx` the DLL. **[C]** is checked in the disassembly; **[G]** is a
guess or only partly traced. `docs/RBS42.txt` is the file format; its errors are listed in §10.

Everything was also checked against the 18 songs in `tests/songs/shipped/` (17 are file version 4.1,
`ReBirth 1.0 Default Song.rbs` is 4.2).

Two implementations follow this document: `tools/rbhost/rbsmsg.py` (for rbhost) and
`crates/rb-engine/src/load.rs` (the engine, task-018), which also covers the 4.0 paths. The test
`crates/rb-engine/tests/rbsmsg_corpus.rs` checks that both give the same messages for every
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
(`llvm-objdump` of `original/ReBirth.dll`) finds no other `+0x16E` displacement and no other
byte access at `+0x16` outside the globals struct's own functions (`sub_10018AC0`,
`sub_10018B00`, `sub_10018BA0`) and the parameter-block functions of other structs; msg 20's
query answers only ids 0..2 (switch at 0x10011FC2). The exe always sends 1. What the exe means by
it is not traced [G]; for rb it is a stored byte (`Globals::byte22`).

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
comp flag, PCF id = `c+1` for a channel with the PCF flag. No 4.0 song was available to check.
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

rb: `MixerParams::default`, `tb303::DEFAULT_PARAMS`, `tr808::DEFAULT_PARAMS`,
`tr909::DEFAULT_PARAMS`, `delay::DEFAULTS`, `dist::DEFAULTS`, `pcf::DEFAULTS`, `comp::DEFAULTS`
hold these values, and every device's `SongDevice::init_track` models its slot 8; the mixer's
(0x100160F0) was missing until task-034 (`frame::tests::constructor_tracks_hold_the_defaults`,
`song_mode_chase_restores_the_mixer_ctor_values`).

### 4.10 The panel's live device messages, as far as read (task-043)

Besides the data block, each device's host-message handler (slot 10) takes the panel's live
changes. Read so far (the rest, and the exe's senders: task-055):

- **TB-303**, `sub_100256E0`, jump table `flt_10153BD8 + 4*sub` (subs 31..46, entries from
  0x10153C54) [C]. Sub 39 (0x10025CB0) is the data block. Sub 40 `{ctrl, v}` (0x10025CFC): for
  ctrl 0..8 it stores the live byte `+0xC7C + ctrl` and calls `sub_10012BC0` → `sub_10021340`,
  the record hook, which acts only with globals byte 0 (song mode) and byte 2 (record) set: it
  sets the track's hold byte `+4 + ctrl` and inserts the event (`sub_10014750`); ctrl 9 stores
  nothing, ctrl 10 stores `+0xC86`; every ctrl then clears its changed flag `+0xC88 + ctrl`. So
  with record off, sub 40 is slot 11 (0x10026180) for ctrls 0..8. Sub 45 `{pattern, step, field,
  v}` (0x10025FC4, field 0..5): the byte `+0x3E + 98*pattern + 6*step + field`, i.e. a step byte
  of the pattern block of §4.6. Sub 42 `{pattern, 0|1, v}` (0x10025F25): the pattern's byte
  `+0x3C` or `+0x3D + 98*pattern`. Subs 43, 44 and 46 (`sub_100191E0`, `sub_10018DC0`,
  `sub_10019B00`) not read.
- **Mixer**, `sub_10015520`, table at `0x10151ECC + 4*sub` (subs 137..148, entries from
  0x101520F0) [C]. Sub 143 (0x1001588F) is the data block; sub 144 (0x100158E7) puts a block
  into the pattern copy `+0x58` only; sub 145 `{id, v}` (0x10015900): id 0 the live master
  `+0x30` (0x1001592C, no record hook), 1 the compressor id `+0x34` and 2 the PCF id `+0x38`
  (each clears a changed flag, `+0x81` or `+0x82`, and calls the record hook with controller 1
  or 2), 3 and 4 nothing; sub 147 `{c, k, v}` (0x100159DB): k 0 the live enabled byte
  `+0x40 + 6c` (0x10015A1F, no record hook: the mute is not automatable), k 1..4 the live level,
  pan, send and dist `+0x41..+0x44 + 6c` with the record hook on controller `5 + 6c + k`.

rb: `Engine::set_param` (slot 11) stands for the controller messages with record off,
`MixerDevice::set_master` and `MixerDevice::set_enabled` for sub 145 id 0 and sub 147 field 0;
`rb_player::Command` sends them.

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
removed unless it is at tick 0. `crates/rb-engine/tests/corpus_seq.rs` checks the load's resulting
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
  investigation §5). Nothing in the load or the export sets it.
- [C] Byte 22 (`+0x16E`) has no effect on the engine (§3, task-034).

Still open:
- [G] No 4.0 song is available to test `sub_437380` against the app.
- [G] What the exe uses byte 22 for (it always sends 1, and the load sends no msg 21 that would
  read it back, §2).

# RBM mods: the container, the key table and the sample lookup

How ReBirth RB-338 2.01 reads a mod (`.rbm`) and hands its samples to the engine, found out in
task-030 (2026-10-27). Addresses `0x4xxxxx` are `Rebirth.exe` (the IDA database of `original/`),
`0x10xxxxxx` are `ReBirth.dll`. [C] marks what was read in the disassembly and checked on real
files; [G] marks an inference. The reference implementation is `tools/rbm.py` (tests:
`tools/rbm_test.py`); `tools/rbhost` loads a mod into the original DLL (`run.py --mod`).

RB20FUL.DAT, its pages, any key table and the mods themselves are Propellerhead material: they stay
on the user's machine and are never committed. Tests read them from `RB_RB20FUL` and `RB_RBM_DIR`.

## 1. Summary

- An `.rbm` is an IFF `CAT ` of type `PRBM`: `HEAD` (256 bytes), the `EMBF` chunks (one embedded
  file each: a NUL-terminated name, then the file), `INFO` (1280 bytes). [C]
- The exe looks up 46 sample names in it (case-insensitive, first match); a missing name keeps the
  built-in sample of that slot. [C]
- Each EMBF payload is stored in the clear up to byte 1024 and scrambled from byte 1024 on:
  `clear[n] = stored[n] + table[(key&1)*2048 + (n-1024) % ((17*key) % 1925 + 123)]` (u8 add), with
  `key` the low byte of the chunk's index in the CAT. [C]
- The 4096-byte `table` is two 2048-byte pages of `RB20FUL.DAT` (the ReBirth CD), at offsets
  `256 + sum_j % (0x8000000 - 2048)`, where `sum_j` are two u32 sums over the first 1024 bytes of
  every EMBF payload of the mod. The pages depend only on the mod's bytes: **each mod needs exactly
  4096 bytes of key material**. [C]
- Shown correct on the four shipped mods: every embedded file decodes (JPEGs, the demo songs, the
  AIFFs), and the samples the original DLL loads from them equal `rbm.py`'s (section 8). [C]

## 2. The container [C]

The exe's IFF reader (`sub_49F477`) reads a 4-byte id and a big-endian u32 size; `FORM`, `CAT ` and
`LIST` (kinds 1, 2, 3) carry a 4-byte type after the size, any other id is a plain chunk (kind 0).
An odd-sized chunk is followed by a pad byte, as in plain IFF: the shipped mods have many odd EMBF
sizes, and only a walk that skips the pad byte ends exactly at the file end (the exe's iterator,
`sub_49F63C`, was not read for this; `rbm.py` pads).

```
"CAT " u32 size "PRBM"            size + 8 == file size (sub_49F3B4); the type is not checked
                                  (rbm.py accepts any type, with a warning)
  "HEAD" u32 256  ...             first chunk in every shipped mod
  "EMBF" u32 n    name\0 file     105 or 106 of them in the shipped mods
  ...
  "INFO" u32 1280 ...             last chunk in every shipped mod
```

- `sub_414D18` (the mod object, built for each file in the `Mods` folder by `sub_4158A9`) requires
  kind 2 (CAT) at 0x414FF4, finds `INFO` by id (`sub_49F6F2`, 0x415066), and throws `XBadFormat`
  unless INFO is 1280 bytes (0x4150BC), its menu name is non-empty (0x415126) and its version's
  major equals the app's (`sub_47DB0B`, 0x415178).
- `HEAD`: no reader. The string `HEAD` at 0x4BB690 (next to `EMBF` at 0x4BB688) has no reference.
  In the shipped mods it holds `5B 35 B3 5B BC 01 00 00 00` and the copyright string
  `(c)1998 Propellerhead Software, all rights reserved.`, zero-padded. The CAT walk counts it, so it
  shifts the EMBF keys (section 4).
- `EMBF`: the payload is a file name, NUL, then the file. The shipped mods embed the panel graphics
  (`12017.JPG` ..., `splash.jpg`), `Demo.rbs` and the samples. `sub_41813E` (the open mod,
  `COpenMod`) keeps per EMBF a 276-byte entry: the name (`strcpy` of at most 261 bytes, 0x418786),
  +264 the absolute file offset of the payload (the file stream's position after the chunk header,
  0x418717), +268 the payload size, +272 a u16 ordinal (section 4).
- Nothing in the exe makes the file name unique; the lookup takes the first (section 3).

### INFO, 1280 bytes (`sub_419343`, copied into the mod object at +8) [C]

| Offset | Size | Contents |
|---|---|---|
| 0 | 600 | description text, NUL-terminated (CR LF lines) |
| 601 | 200 | ftp URL |
| 802 | 200 | web URL |
| 1003 | 64 | the name in the Mods menu (non-empty; a mod whose name is already in the list is dropped with a message, `sub_4155D1` at 0x415A90) |
| 1073 | 4 | version: tag 0xBC, major, minor, revision (`sub_47DB95`); a zero byte at 1073 means "the app's version" (bytes `byte_4DA794..`) |

The shipped mods: PBE2, MSM2, Orbit2 carry `BC 01 00 00` (1.0.0); Metallicon has zeros. The app's
version major is compared but is set at run time (`unk_4DA790`); it is 1 on the test machine [G:
inferred from all four mods loading].

## 3. The sample lookup [C]

`sub_41693C` builds the mod message (section 6). For each slot it calls `sub_416710(name)`, which
walks the EMBF entries in file order and returns the first whose name equals `name` under
`sub_4713D4` (a character-by-character compare of both strings through `sub_4A9280`, a case fold,
including the terminating NUL). The names:

- 808, slots 1..24 (0x4BB388, 17-byte stride): `TR808BD.aif`, `TR808SD1.aif` .. `TR808SD5.aif`,
  `TR808N1.aif` .. `TR808N4.aif`, `TR808CB.aif`, `TR808CH.aif`, `TR808CL.aif`, `TR808CP.aif`,
  `TR808CY.aif`, `TR808HC.aif`, `TR808HT.aif`, `TR808LC.aif`, `TR808LT.aif`, `TR808MA.aif`,
  `TR808MC.aif`, `TR808MT.aif`, `TR808OH.aif`, `TR808RS.aif`;
- 909, slots 25..45 (0x4BB520): `TR909bd1.aif` .. `TR909bd4.aif`, `TR909sd1.aif` .. `TR909sd5.aif`,
  `TR909sdn1.aif` .. `TR909sdn3.aif`, `TR909lt.aif`, `TR909mt.aif`, `TR909ht.aif`, `TR909rs.aif`,
  `TR909hc.aif`, `TR909chh.aif`, `TR909ohh.aif`, `TR909ccy.aif`, `TR909rcy.aif`;
- slot 46, the startup sound: `Startup.aif` (0x4BB378).

This is the slot order of the DLL's sample table (`sub_1001A030`, docs/initial_investigation.md
7.4). A name that is not found sends a record with `present = 0`, and the DLL loads that slot's
built-in resource (`sub_1001A3E0` with ID 1000 + i, 1025 + i or 1024). The four shipped mods
replace all 46 slots.

## 4. The scrambling (DLL) [C]

`sub_1001A500` loads one present slot: it reads `size - name_len` bytes from the mod file at
`offset + name_len` into a memory buffer (vtable +0x30 of the open file, 0x1001A5A1), then, if
`size > 1024` (0x1001A5A9), calls `sub_100163D0(table, buffer + 1024 - name_len, size - 1024, key)`,
then parses the buffer as AIFF (`sub_10003300`). In payload coordinates (the name included):

```
P    = (17 * key) % 0x785 + 123          // 0x100163EF..0x10016404; key is 0..255, P is 123..2047
base = (key & 1) << 11                    // 0x10016412..0x1001641B
for n in 1024 .. size-1:                  // j counts 0..P-1, then wraps (0x10016425..0x10016442)
    payload[n] = (payload[n] + table[base + (n - 1024) % P]) & 0xFF
```

- `key` is the low byte of the chunk's ordinal among **all** chunks of the CAT, counted from 0 at
  `HEAD` (`sub_41813E` increments `v59` for every chunk, 0x418832, and stores it as a word at entry
  +272; `sub_41693C` sends the byte `Str[272]`, 0x416A9B). In the shipped mods the first EMBF is
  key 1.
- A payload of at most 1024 bytes is not scrambled; nor are the first 1024 bytes of any payload,
  which hold the name and the AIFF header.
- Only byte `base + j` with `j < P <= 2047` is used: the half of the table is chosen by the key's
  parity.

## 5. The key table (exe) [C]

### 5.1 The key sums

At start-up `sub_40DDFB` scans the `Mods` folder (`sub_4158A9`, `this` = the mod list, argument =
the app's field `this[325]` (`[ecx+514h]`, 0x40DED2), the file spec of `RB20FUL.DAT`). For each
mod, `sub_414D18` creates an 8-byte object of two u32 sums initialized from `dword_4BB290` =
`{0x2000, 0x3000}` (`sub_414987`), and for each EMBF chunk, in file order (0x4151B7..0x4152EA),
reads `min(size, 1024)` bytes from the start of its payload (the name included) and calls
`sub_4149EC(buf, n)`:

```
b = 1024 bytes of 0x65; b[0:n] = payload[0:n]          // memset 0x65, memcpy (0x414A01..0x414A26)
stride = ((dword_4DAE58 & 0xFFFFF000) >> 14) - (((api & 0xFFFF) + 1) & 0x3FFF) - 14   // 0x414A5F..0x414A86
idx = 0
for j in 0, 1:
    s = 0
    repeat 128: s += be32(b + idx * stride); idx += 1      // sub_465FCA = byteswap
    sum[j] += s                                             // u32 wrapping
```

The 0x65 padding is exercised by real data: Metallicon.rbm has eight EMBF payloads under 1024 bytes
(`12019.jpg` 917, `12020.jpg` 931, `12031.jpg` 843, `12501.jpg` 850, `12505.jpg` 833, `12506.jpg`
810, `12509.jpg` 845, `12510.jpg` 831), and its pages decode the mod (section 8); the other three
shipped mods have none (smallest payloads 1203, 2083, 2335 bytes).

`api` is `[dword_4DFB24 + 0x174](hwnd)` of the main window (`sub_48126D` -> `sub_481A59`), an entry
of the app's USER32 function table [G: GetParent, as docs/song_load.md section 9 has it].

### 5.2 The stride is 4 [C]

`dword_4DAE58` is never written by code: it lies at +0xE0 of the 2048-byte page `unk_4DAD78` that
`sub_4065F2` fills at start-up from `RB20FUL.DAT` offset
`256 + ((((GetWindowLongA(hwnd, -8) + 1) & 0x3FFF) + 0x4000) << 11)` (0x406947..0x40697D;
docs/song_load.md section 9). With `k = (parent + 1) & 0x3FFF` the page is page `k + 0x4000` of the
body, and the stride is `((g_k & 0xFFFFF000) >> 14) - k - 14`, provided the two API values agree
in their low 14 bits.

Checked on the real `RB20FUL.DAT` (task-030, all 16384 pages `k`; `tools/rbm.py check-dat DAT`,
and the test `RealDat` of `tools/rbm_test.py` with `RB_RB20FUL` set): `((g_k & 0xFFFFF000) >> 14)
- k == 18` for every `k`, so **stride = 4**, and the 256 words are the 1024 bytes in order. Two more
uses of the same expression agree: `sub_41FFBC`'s anti-debug branch (an IDT check, 0x41FFE9) sets
it to 0x12 = 18 directly, and `sub_423048` (the distortion packer) uses it as `X / 18` = index 1.
In the same page `dword_4DB124` (+0x3AC) satisfies `((g_k & 0xFFFFF000) >> 12) - k == 8` for every
`k`, which makes `sub_420568`'s 808 record count `3 * 8 = 24`.

So, in an untampered run:

```
sum = [0x2000, 0x3000]
for each EMBF payload p, in file order:
    b = (p[:1024] + 0x65 * 1024)[:1024]
    sum[0] += Σ be32 words 0..127 of b
    sum[1] += Σ be32 words 128..255 of b
```

### 5.3 The pages

After the scan (`sub_4158A9` 0x415E09..0x415F6A), for each mod and `j = 0, 1`:

```
table[2048*j : 2048*j + 2048] = RB20FUL.DAT[off_j : off_j + 2048],
off_j = sub_404C35() + sum[j] % (0x8000000 - 0x800)       // 256 + ...; sub_414B65 (div, edx)
```

The file is opened from `this[325]` (`sub_484151`), read with seek + read (`sub_46A80C`). The
offsets are not 2048-aligned. The table goes into the mod's 4096-byte buffer (`sub_4144D0`), then is
copied into the open mod (`sub_414546`, 0x41858C) and into message 55 (section 6).

`RB20FUL.DAT` (134,217,984 bytes): a 256-byte header (`03 E1 F1 1E`, `BC 01 01 00 00`,
`(c)1997 Propellerhead Software, all rights reserved.`, zeros; checked by `sub_4065F2`, magic
65138974 = 0x03E1F11E at 0x406662), then 2^27 bytes of key material.

The key pages of the shipped mods (`rbm.py info`; the numbers are offsets, not material):

| Mod | sum[0] | sum[1] | off_0 | off_1 |
|---|---|---|---|---|
| PBE2.rbm (Pitch Black Edition 2.0) | 0x9F0C36E0 | 0x6AA397BE | 118280160 | 44302526 |
| MSM2.rbm (Massen's MSM 2.0) | 0x8886EF4A | 0x908F2A1C | 8878154 | 9419548 |
| Metallicon.rbm (Metallicon+) | 0x1909239F | 0x1AEF9F26 | 17382559 | 49264678 |
| Orbit2.rbm (Orbit 2.0) | 0x12621363 | 0x5C1F52D1 | 39986275 | 69184465 |

Consequences for rb (task-032, task-037): the table of a mod is a pure function of the mod file and
`RB20FUL.DAT`. Descrambling a mod needs 4096 bytes of the DAT, the two pages above; a mod changes
pages whenever any of its EMBF headers (first 1024 bytes) changes.

## 6. Message 55: the mod to the engine [C]

`sub_41693C` (the open mod's "send to engine") sends top-level message 55 with 9542 bytes
(0x2546), packed by `sub_420568` and unpacked by the DLL's `sub_10018870` (handler
`sub_1001DA70` -> `sub_1001A030`, reply 'C' = 67). Message 56 `{u16 id}` is the built-in mod
(`sub_4166E5` true, id 411). All numbers big-endian:

| Offset | Size | Contents |
|---|---|---|
| 0 | 2 | mod id (`sub_4166D0`: the open mod's word at +0x694, taken from the counter `word_4CEB70` at 0x4182D6) |
| 2 | n | the mod's file spec (`sub_483143`): u32 tag 0xF11E54EC, the full path, NUL; the DLL checks the tag against `dword_10150388` (`sub_1000C604`; 0xF11E54EC at run time) |
| 4802 | 4096 | the key table |
| 8898 | 24 × 14 | 808 records, slots 1..24 |
| 9234 | 21 × 14 | 909 records, slots 25..45 |
| 9528 | 14 | the startup record, slot 46 |

Record (14 bytes, `sub_41FFBC`): u8 present, u8 key, u32 payload offset (absolute, in the mod
file), u32 name length (`strlen(name) + 1`, where the file starts in the payload), u32 payload size.
The DLL widens it to 16 bytes (`+0` present, `+1` key, `+4` offset, `+8` name length, `+12` size).

The 909 records' stride and the record count are computed through the anti-tamper pages
(`v11 = ((dword_4D9AE0 & 0xFFFFF0) >> 4) - (PID & 0x222) + 3`, intended 14; the 808 count
`3 * (...)`, intended 24): the layout above is the untampered one (docs/song_load.md section 9).
`tools/rbm.py msg` writes this message, and `tools/rbhost --mod-msg` sends it.

## 7. Defects

None found in this path. The key wraps every 256 chunks (a mod with more than 255 chunks reuses
keys), and two EMBF of the same name leave the second unreachable; both are format limits, not
defects.

## 8. Verification (task-030, 2026-10-27)

- Static: `rbm.py extract MOD RB20FUL.DAT -o DIR` on the four shipped mods: every AIFF of the 46
  slots parses (16-bit mono, 44100 Hz); all 59 JPEGs per mod end in `FF D9` and `splash.jpg`
  decodes to a clean 256 × 256 image; each embedded `Demo.rbs` passes `tools/rbs.py check`
  (read and write back byte-identical). The mean |x[n] - x[n-1]| of the AIFF audio is 310..1202
  after descrambling against 21252..21898 without it.
- Dynamic: `tools/rbhost/run.py --mod MOD.rbm --rb20ful RB20FUL.DAT --dump-samples` loads the mod
  into the original `ReBirth.dll` with message 55 (reply 'C') and dumps the DLL's sample table
  (list `dword_10118280`; CWaveSample frames at +0x4C, data at `[[+0x50]+0x0C]`). For all four mods
  all 46 slots equal channel 0 of `rbm.py`'s descrambled AIFF, sample for sample. The DLL runs its
  own descrambler and AIFF reader on the records and table that `rbm.py` built; the table's
  derivation (section 5) is what the decoded JPEGs and songs confirm, since a wrong page would
  garble every payload past byte 1024.
- The table that `Rebirth.exe` itself computes was never dumped; task-031 closes that by output
  (2026-10-27): the original app (File > Export Loop as Audio File, the song's mod found in its
  own `Mods` folder) exported RSI (PBE2), Fonda Workout (MSM2) and Metallicon City (Metallicon+),
  and `tools/rbhost/fit.py --all` with `--mod` and `rbm.py`'s key table (from the user's
  RB20FUL.DAT) found an `N` at which rbhost's render has the same SHA-256 as the app's WAV, byte
  for byte: N = 3536, 3536 and 3504 (the only match among 200 candidates each). A wrong table
  would garble the samples, so the app's own table and `rbm.py`'s agree for these three mods
  (Orbit2 has no shipped song). The references are in `tests/refs` (sets `mod` and `modapp`).
  The fitted `N - N_t` is -28, -40 and -5, against +48 +- 15 for the mod-less songs (§13 of
  `docs/export_state.md`): N sits below the timeline's estimate for these mod songs; not
  investigated [G: 3 exports].

## 9. The `rbm` crate (task-032, 2026-10-27)

`crates/rbm` implements sections 2 to 5 (`Rbm::parse`, `KeyTable::read` from a reader over
`RB20FUL.DAT`, `Rbm::file`, `Rbm::load_samples`). Its tests (`RB_RB20FUL`, `RB_RBM_DIR`) compare
every embedded file and the 46 samples of the four shipped mods with `tools/rbm.py extract`.

One fact found on the way: 19 of the 46 AIFFs of Metallicon.rbm have a **12-byte COMM chunk** (the
rate field is 4 bytes, not 10; the SSND chunk follows at once); the other 27 have 18 bytes. [C]
The DLL reads the COMM fields in `sub_10001674` (0x10001674) at +0 channels, +2 frames, +6 bits and
+8 the 80-bit rate, from the chunk start with no size argument; the chunk walk only checks that
the 8-byte chunk header fits (0x100050CD..0x100050D6); `sub_100016F4` then validates the values
(non-zero channels, frames and rate, bits 8..32). So the rate is read through the chunk end into
the SSND header. `rbm.py` does the same, and the engine's AIFF reader (`samples/aiff.rs`) takes
COMM from the chunk start to the end of the file rather than rejecting a chunk under 18 bytes.
The audio is read the same way (section 11.2): `sub_10002FC4` reads `2 * frames` bytes after the
SSND header whatever the SSND size says, and so does the engine's reader since task-058.

## 10. Mods in the engine and the CLI (task-033, 2026-10-27)

- **Name lookup** [C]: `sub_4155D1` (0x4155D1), called with the GLOB mod name, walks the mod list
  and returns the first mod whose C string at +1011 equals the name under plain `strcmp`. +1011 is the
  INFO menu name (INFO offset 1003, the object copies INFO at +8): the match is on the INFO
  name, exact and case-sensitive, not on the file name. A name that is not in the list gets the
  Download / Standard / Cancel dialog (docs/song_load.md section 2). `rb` matches the INFO name
  the same way and, as a convenience the original lacks, a file name (`PBE2` or `pbe2.rbm`, case-
  insensitive) when no INFO name matches. `--mod FILE` whose INFO name is not the song's name
  warns, as the original would not load it for that song. [G: the value `strcmp`'s result is
  compared with comes from the anti-tamper pages and is 0; not traced.]
- **Message 55 and rendering**: of the 9542 bytes, only the 46 sample records (and the key table
  and file spec that locate and descramble them) reach the sample table; the mod id only selects
  the mod for message 52 (and 58, the preview player). So the engine's input from a mod is its
  46-slot `SampleSet` (a slot the mod lacks keeps the built-in sample). Confirmed by output: with
  only the samples modeled, all 8 `mod` and 3 `modapp` references render byte-exact.
- **CLI**: `rb render` / `rb compare` take `--mod-dir DIR` (`RB_MOD_DIR`; `RB_RBM_DIR` is a
  deprecated alias), `--mod FILE` (overrides the song's mod), `--rb20ful PATH` (`RB_RB20FUL`, then
  an `RB20FUL.DAT` in the mod folder or its parent or next to the binary, or the Windows install
  folders) and `--standard-kit` (a missing mod renders with the built-in kit and a warning).
  Without it a song naming a mod that is not found is an error. A mod needs the key file only
  when a drum machine plays (the mod holds nothing else). `rb compare --manifest` includes the
  sets `mod` and `modapp` by default only when a mod folder is given.
- **AIFF rate** [C]: `sub_10001674` (0x100016B2..0x100016DE) converts the 80-bit rate with `fistp`
  under a control word with rounding set to truncate (`| 0xC00`) into a 64-bit slot and keeps the low
  dword; `sub_100016F4` rejects 0. The engine's reader now does the same (a rate below 1 Hz, or
  with zero low 32 bits, such as 2^32, or one that does not fit in 63 bits, is rejected).
  [G: the slot is a qword, from the 8-byte stack variable; a dword `fistp` would differ only for
  rates of 2^31 and up, which no mod has.]

## 11. A third-party collection (task-058, 2026-10-27)

The user's collection (`~/rb_stuff`: `Mods/`, 172 `.rbm`; `songs/`, 193 distinct songs; and a large
song archive, `CaveysRebirthStuff/`) is third-party material: it is read in place and nothing of it
is in the repository. Mod names and counts only below.

**The check.** `tools/rbm.py check-dir DIR RB20FUL.DAT [--json]` (the reference: every mod's
parse, key pages, every embedded file descrambled, the 46 slots' AIFFs, JPEG end markers,
duplicate INFO names in the original's folder order) and the env-gated cargo test
`crates/rb-cli/tests/thirdparty.rs` (`RB_THIRDPARTY_DIR`, `RB_RB20FUL`; run it with
`cargo test --release -p rb-cli --test thirdparty -- --nocapture --test-threads 1`), which compares
`crates/rbm` with `check-dir --json` (the SHA-256 of every embedded file, the channel-0 PCM of every
slot) and reads, writes back and renders every distinct song.

### 11.1 Mods

- **All 172 load**, in `rbm.py` and in `crates/rbm`, with the same 16680 embedded files and the
  same 7285 slot samples. Every one is a `CAT ` of type `PRBM` with `HEAD` first and a 1280-byte
  `INFO` last, only `EMBF` between, under 256 chunks, version major 1 (or zero), no duplicate EMBF
  name. Every slot AIFF is mono, 16-bit, 44100 Hz. [C]
- **The key table is right for all of them**: the mean |x[n] - x[n-1]| of the descrambled slot
  audio past byte 1024 is 180..1710 per mod, against 20328..22071 for the stored bytes; and the
  rbhost renders of section 11.4 load the samples through the DLL's own descrambler. [C]
- **54 mods leave slots built in** (1 to 46 of the 46): the original sends `present = 0` and the
  DLL loads the built-in sample (section 3); `rb` does the same (`Rbm::load_samples`). One mod,
  ReBirth 666, holds no sample at all (panel graphics only): it plays the built-in kit.
- **Stale AIFF sizes** (3 mods: 8tRack, DJ Turn's 8-Track, a byte-identical copy of it, and DJ
  Turn's MOD): their `TR808BD.aif` has FORM and SSND sizes smaller than the audio that follows
  (COMM 21064 frames, SSND 15798; 18052 and 13539). `crates/rbm` rejected the sample; the DLL plays
  it whole (section 11.2). Fixed in the engine's AIFF reader.
- **JPEGs without an end marker** (1 mod, YellowBirth): its 59 panel JPEGs end without `FF D9`
  (progressive JPEGs cut short by its packer [G]); `splash.jpg` decodes with a garbled edge. The
  samples are fine, and the engine uses none of the JPEGs.
- **Duplicate INFO names** (5 pairs): 8tRack (the two identical files), AXIOM (`AXIOM.rbm`,
  `axiom(b3).rbm`), dejenerate (`dejenerate.rbm`, `dejenerate(b3).rbm`), Alpha 2.0 (`Alpha2.rbm`,
  `alpha3.rbm`), AlpJunUV (`AlpJunUV.rbm`, `UVJ.rbm`). Which file the original keeps is section
  11.3.

### 11.2 The AIFF audio is read past a stale SSND size [C for the mono read]

`sub_10002FC4` (the SSND reader of `CAIFFReader`) finds SSND and reads its 8-byte offset and block
size (`sub_10004F00`, `sub_10001774`), skips the offset (vtable +0x2C), and for a mono file reads
`2 * frames` bytes (0x10003060 `lea eax,[ebx+ebx]`, the read at vtable +4, 0x1000306F; `frames`
from COMM) into the sample buffer; nothing compares that with the SSND size. The stream is the
whole descrambled payload (section 4), so the audio past the stale size is there and is played.
Confirmed with `rbhost --dump-samples` and 8tRack.rbm / DJ Turn's MOD.rbm: slot 1 is 42128 and
36104 bytes, equal to the file's audio sample for sample. `samples/aiff.rs` now reads the audio
through to the end of the file, as it already did COMM (section 9). `docs/original_bugs.md` Q-3.

Open (not checked against the DLL):
- **A file that really ends early.** The read's result is ignored (0x10003071 goes straight on to
  `sub_10009990`), and what the memory stream does when asked for more than it holds (a short
  read, zeros, stale bytes or an exception) was not traced; no mod in the collection reaches it.
  rb's policy is an error, and `Rbm::load_samples` propagates it, so the **whole mod** is refused
  rather than that one slot falling back to its built-in sample.
- **Stereo.** Only the mono path was read; the multichannel path (0x10003087, `sub_10009DD0`) was
  not examined. Every slot AIFF of the collection is mono.

### 11.3 The Mods folder order and duplicate names

`sub_4158A9` lists the `Mods` folder with `FindFirstFile`/`FindNextFile` over `*.*`
(`sub_4862C9`, the pattern at 0x4D11C4; `sub_486355`, the app's function table +164 and +112),
builds a mod object for each file (`sub_414D18`) and, when `sub_4155D1` finds its INFO name already
in the list (0x415A90), deletes it and shows message 500128050 (`sub_475C12`). [C: the disassembly.]

- **The first file listed wins** [C]: on the test machine (NTFS), a copy of the app with both
  AXIOM files in its `Mods` showed **"The MOD "AXIOM.rbm" was not added because another Mod with
  the same name is already installed."**, and its export of AXIOM.rbs matches the `axiom(b3).rbm`
  render, not the `AXIOM.rbm` one (section 11.4). So there `axiom(b3).rbm` was listed first.
- **The listing order is the volume's, not the app's** [G: general file-system behavior, not
  verified here beyond the AXIOM case]: NTFS lists a folder by the names' UTF-16 units after its
  per-unit `$UpCase` table, which puts `axiom(b3).rbm` before `AXIOM.rbm` (`(` is 0x28, `.` 0x2E)
  and `dejenerate(b3).rbm` before `dejenerate.rbm`; FAT or a network share may list in directory
  order, so which duplicate wins depends on the user's volume.
- **rb models NTFS**: it sorted bytewise, which put `AXIOM.rbm` first; `mods.rs` (`folder_order`)
  now sorts by the name upcased per character (only where the uppercase is one character, so `ß`
  stays `ß` as in `$UpCase`), then compares UTF-16 units. This approximates `$UpCase` (it agrees
  for ASCII, which is every name of the collection).

### 11.4 Songs and the reference check

- `songs/`: 193 distinct songs by SHA-256 (208 files whose name contains `.rbs` when the task
  started, with "... copy" duplicates; 195 after the user's cleanup during it). At the review
  (same day) the folder had grown to 206 distinct songs, 77 RBS 4.2 and the same 129 1.x: the
  check passed again, every mod resolving by its exact INFO name (51 Standard, 14 PBE2, 5 MSM2,
  2 Poison-Box, 1 each of Regulation Issue, Orbit 2.0, Nord Beat, Metallicon+, RedTop 2.0). The
  counts below are those of the first run. **64 are RBS 4.2**: every one reads and writes back byte-identical through
  `crates/rbs` and renders with `rb render --mod-dir` without a fault. Their mods: 46 Standard
  ReBirth, 12 Pitch Black Edition 2.0, 2 Massen's MSM 2.0, 2 Poison-Box, 1 each Orbit 2.0, Massen
  RedTop 2.0, Regulation Issue, Nord Beat; all resolve by the exact INFO name (none missing, none
  by case only).
- **129 are ReBirth 1.x songs**: a Standard MIDI File (`MThd`, a meta text "File version 3.1.0")
  named `.rbs`. The original imports them (`sub_425C4D` tries `sub_43037D` first: the MIDI file and
  its "File version %s" text, `sub_431341` at 0x431481; then `sub_430BA0` loads it with the built-in
  mod, 0x425FCE); `rb` refuses them with an error. Follow-up: task-060.
- **rbhost**: 49 songs from `~/rb_stuff` (mostly the archive, where the mod songs are) over 29
  third-party mods, each with the mod its GLOB names and where that mod is audible (the render
  differs from the built-in kit's). 47 rendered with `run.py --mod --rb20ful --idle-blocks 9088`
  and **all 47 equal `rb render` byte for byte** (27 mods). The other 2 (the RB-SEMx and TB-909e
  mkII songs) are RBS 4.0 files, which `tools/rbhost/rbsmsg.py` cannot convert (no 4.0 mixer
  converter); `rb` renders them. Also exact: AXIOM.rbs with `AXIOM.rbm` (it differs from the
  `axiom(b3).rbm` render), and two songs each with 8tRack and DJ Turn's MOD (the stale-size
  samples; GLOB renamed in scratch copies).
- **The app** (`tools/rb_export.py --app-dir`, a copy of the installation in `C:\re\rb` with its
  own `Mods`; `--startup-boxes 2` for the duplicate-name box; `--restore-prf --startup-sound
  off`) exported AXIOM.rbs (AXIOM, both AXIOM files present), Commutation (Trancer) and Meth X
  State (Poison-Box), each mod found by the app's own scan. `fit.py --all` (200 candidates) found
  one N each, 3616, 3600 and 3632 (N - N_t = -71, -5, -9), and `rb compare --mod-dir` at that N is
  **EQUAL** for all three. AXIOM.rbs against `AXIOM.rbm` matches at no N: the app used
  `axiom(b3).rbm`, as section 11.3 says.
- `run.py` and `fit.py` take `--remote-dir NAME` (work in `C:\re\rb\NAME`), `rb_export.py`
  `--app-dir` and `--startup-boxes`.

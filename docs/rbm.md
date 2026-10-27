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

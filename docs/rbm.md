# RBM mods: the container, the key table and the sample lookup

How ReBirth RB-338 2.01 reads a mod (`.rbm`) and hands its samples to the engine, found out in
task-030 (2026-10-27). Addresses `0x4xxxxx` are `Rebirth.exe` (the IDA database of `original/`),
`0x10xxxxxx` are `ReBirth.dll`. [C] marks what was read in the disassembly and checked on real
files; [G] marks an inference. The reference implementation is `tools/rbm.py` (tests:
`tools/rbm_test.py`); `tools/rbhost` loads a mod into the original DLL (`run.py --mod`).

RB20FUL.DAT, its pages, any key table and the mods themselves are Propellerhead material: they stay
on the user's machine and are never committed. The `rbm` crate's and `rbm.py`'s tests read them from
`RB_RB20FUL` and `RB_RBM_DIR`; the rb CLI and its tests read only recrypted mods (`RB_MOD_DIR`,
section 12.4).
The one exception, decided by the user on 2026-10-27 (task-059): the 2047-byte window of section
12 is embedded in rb (`crates/rbm/src/rb20ful_window.bin`, `embedded.rs`).

## 1. Summary

- An `.rbm` is an IFF `CAT ` of type `PRBM`: `HEAD` (256 bytes), the `EMBF` chunks (one embedded
  file each: a NUL-terminated name, then the file), `INFO` (1280 bytes). [C]
- The exe looks up 46 sample names in it (case-insensitive, first match); a missing name keeps the
  built-in sample of that slot. [C]
- Each EMBF payload is stored in the clear up to byte 1024 and scrambled from byte 1024 on:
  `clear[n] = stored[n] + table[(key&1)*2048 + (n-1024) % ((17*key) % 1925 + 123)]` (u8 add), with
  `key` the low byte of the chunk's index in the CAT for the 46 samples (the DLL reads them), the
  whole u16 index for the files the exe reads itself (the panel JPEGs, the demo song); the two
  agree below index 256, which every known mod is (section 12). [C]
- The 4096-byte `table` is two 2048-byte pages of `RB20FUL.DAT` (the ReBirth CD), at offsets
  `256 + sum_j % (0x8000000 - 2048)`, where `sum_j` are two u32 sums over the first 1024 bytes of
  every EMBF payload of the mod. The pages depend only on the mod's bytes: **each mod needs exactly
  4096 bytes of key material**. [C]
- `rb recrypt` rewrites a mod so that it needs only the 2047 bytes of `RB20FUL.DAT` that rb
  embeds; the original loads the result unchanged (section 12).
- `rb standard-mod` builds the built-in kit (ReBirth.dll's 46 AIFF resources) and panel
  (Rebirth.exe's JPEG resources) into such a mod, "ReBirth Standard Kit", from which rb takes the
  built-in samples (section 13).
- The Mods menu changes the mod of an open song in place: song msg 19 `{id}`, which silences the
  808's and the 909's voices and replaces their sample table; the song plays on (section 14).
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
- The exe has its own copy of the descrambler for the files it reads (section 12.1):
  `sub_4148DF`, the same formula with the key taken as the whole u16 ordinal.

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

- More than 200 EMBF chunks overrun the open mod's entry table (section 12.1;
  `docs/original_bugs.md` OB-14).
- The DLL's key wraps every 256 chunks and the exe's every 65536 (section 12.1), and two EMBF of
  the same name leave the second unreachable; these are format limits, not defects.

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
  deprecated alias), `--mod FILE` (overrides the song's mod) and `--standard-kit` (a missing mod
  renders with the built-in kit and a warning). Without it a song naming a mod that is not found
  is an error. A mod is read only when a drum machine plays (the mod holds nothing else).
  `rb compare --manifest` includes the sets `mod` and `modapp` by default only when a mod folder
  is given. Since task-063 these commands take no `--rb20ful` and read no `RB20FUL.DAT`: they
  load recrypted mods only (section 12.4). Task-033 had `--rb20ful PATH` / `RB_RB20FUL` here.
  Since task-036 the built-in kit comes from the standard mod (`--standard-mod`, section 13.4),
  not from `--rebirth-dll`.
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
`crates/rb-cli/tests/thirdparty.rs` (`RB_THIRDPARTY_DIR` with `Mods/` recrypted, `Mods.orig/` and
`RB_RB20FUL` for the reference since task-063; run it with
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
  mod, 0x425FCE); `rb` refused them with an error until task-060, which imports them as the exe
  does (`docs/song_load.md` section 12).
- **rbhost**: 49 songs from `~/rb_stuff` (mostly the archive, where the mod songs are) over 29
  third-party mods, each with the mod its GLOB names and where that mod is audible (the render
  differs from the built-in kit's). 47 rendered with `run.py --mod --rb20ful --idle-blocks 9088`
  and **all 47 equal `rb render` byte for byte** (27 mods). The other 2 (the RB-SEMx and TB-909e
  mkII songs) are RBS 4.0 files, which `tools/rbhost/rbsmsg.py` could not convert at the time;
  task-061 added the 4.0 mixer converter and both then render with rbhost equal to `rb render`
  byte for byte (49 of 49). Also exact: AXIOM.rbs with `AXIOM.rbm` (it differs from the
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

## 12. Recrypted mods: one 2047-byte embedded key window (task-059, 2026-10-27)

The user decided on 2026-10-27 (superseding task-053's "nothing bundled" for this only) that rb
embeds a minimum of `RB20FUL.DAT` that lets a mod load without the CD, and that
`rb recrypt IN.rbm -o OUT.rbm [--rb20ful PATH]` rewrites a mod to need only that. Shown the
trade-off of 12.2 (123 bytes with 113663-chunk mods against 2 KB with compact ones), the user
chose, the same day, **one 2 KB window and compact mods**: rb embeds `RB20FUL.DAT[W .. W + 2047]`,
and a recrypted mod keeps its chunk count and size (plus one 1024-byte chunk). A mod that is not
recrypted is not loaded by rb at all (section 12.4); `rb recrypt` is the one place `RB20FUL.DAT`
is read.

### 12.1 Facts the scheme rests on [C]

- **Two descramblers, two keys.** The DLL descrambles the 46 slot samples with the byte key of
  message 55 (section 4, the ordinal's low byte, `Str[272]` at 0x416A9B). The exe reads every
  other file itself: `sub_41678A` (the open mod's reader; called by `sub_41703A` for the panel
  JPEGs `%d.jpg` and `Splash.jpg`, id 12999, and by `sub_417022` for the demo song that
  `sub_419E92` saves) reads the payload and, if it is over 1024 bytes, calls `sub_4145FE` ->
  `sub_4148DF(buf + 1024 - name_len, size - 1024, word entry+272)`, which computes
  `P = (17 * (key & 0xFFFF)) % 0x785 + 123` (0x4148EE..0x414905) and `base = (key & 0xFFFF) % 2
  << 11`, then `buf[i] += table[base + j]` (`add dl,[eax+ecx]` at 0x41494D). So for the exe the
  key is the **whole u16 ordinal** (the word at entry +272, `v59`, an int16 counter of all
  chunks, 0x418832). Below ordinal 256 the two keys are equal, which is why no shipped mod shows
  it; the parity, hence the page, is always the same. Confirmed by output: in the app, the
  PBE2 and Metallicon+ panels of the 123-byte recrypt (12.2; JPEGs at ordinals past 255, scrambled
  with the u16 key) display clean (12.3). `sub_414620` (0x4145F3 is its only reference, outside any function) is
  the inverse (`sub dl`), the packer's scrambler [G: dead code].
- **200 EMBF at most.** `COpenMod` has 200 entries of 276 bytes from +0x69C and writes entry
  `count` with no bound check (0x418758..0x4187DA): `docs/original_bugs.md` OB-14. Chunks of other
  ids take no entry and are only counted.
- **Any other chunk id is skipped.** The CAT walk (`sub_49F63C`) moves to `start + size`, plus 1 if
  that is odd; `sub_49F477` gives every id but `FORM`, `CAT ` and `LIST` kind 0, and a size of 0
  is fine. The sums (section 5.1) read EMBF chunks only.
- **The pages depend only on the first 1024 bytes of each EMBF**, which are never scrambled
  (sections 4 and 5), and a file's table bytes depend only on its reader's key, i.e. on its
  ordinal: a file with key `k` and payload size `s > 1024` uses bytes `0 .. min(P(k), s - 1024)`
  of page `k & 1`.

### 12.2 The scheme and the window size

With both pages at one window `W` of `L` bytes, a file with key `k` is covered when `P(k) <= L`
(or its payload is short). A filler chunk between files moves the next file to another key, so
`L` trades against the chunk count:

- **The analysis: 123 bytes is the floor.** `P(k) = (17k) % 1925 + 123` is 123 only at
  `k = 0` (mod 1925): ordinal `o` with `o % 256 == 0` for the DLL, `o % 65536 % 1925 == 0` for
  the exe (35 ordinals in each 65536). A mod whose every scrambled file sits at such an ordinal
  needs exactly `RB20FUL.DAT[W .. W + 123]`, and no smaller amount works for a mod with a payload
  over 1147 bytes (all 176 known mods). The cost is the fillers: about 255 per sample and 1925 per
  exe file, 8 bytes each; a shipped mod grows from 108 to 113663 chunks (+909 KB). The curve over
  the 176 mods of sections 8 and 11 (most chunks any of them needs): 123 bytes, 113662 chunks; 132,
  11551; 136, 8154; 140, 6343; 174, 2154; 293, 630; 2047, 107 (scratch analysis in task-059's
  notes). The original accepted the 123-byte form (12.3).
- **The choice (the user, 2026-10-27): 2047 bytes, no fillers.** The longest period is 2047
  (`17k % 1925` reaches 1924 for the exe's u16 keys; 2044 at the DLL's `k = 113`), and the table
  index is `base + j` with `j < P`, so byte 2047 of a page is never read: 2047 bytes cover every
  key, and 2048 would add a byte no key uses. Every file then keeps its place.

`rbm::recrypt` (crates/rbm/src/recrypt.rs):
1. Decrypts every EMBF with the mod's real table and each file's reader key.
2. Lays the CAT out again: the non-EMBF chunks before the first EMBF (HEAD); a 1024-byte EMBF
   `rb-recrypt.pad` (never scrambled; its name is none of the files anything reads); then,
   ordinal by ordinal, the first remaining file in file order that is not scrambled or whose
   reader's period there is at most `L`, else a filler chunk (id `"    "`, the IFF 85 filler id,
   size 0; with `L = 2047` there is none); files of the same name keep their order (the lookup
   takes the first); the remaining non-EMBF chunks (INFO) last. So a recrypted mod is the original
   with the pad after HEAD: 108 -> 109 chunks, +1032 bytes for every known mod.
3. Sets the pad's words at payload offsets 508 (in sum[0]'s half) and 1020 (sum[1]'s) so that both
   sums are `W - 256`: both pages are `W`.
4. Scrambles every file against the window under its new key, and checks the result (pages,
   coverage, at most 200 EMBF).

**Dropped input chunks.** An earlier pad (an EMBF `rb-recrypt.pad` of 1024 bytes) and every
zero-size chunk of the id `"    "` are taken for an earlier recrypt's and dropped, so recrypting a
recrypted mod gives the same bytes. Nothing reads such a chunk; no known mod has one.

The window is `W = 118280160`, PBE2.rbm's page 0 (section 5.3), a page the original already reads
for a shipped mod; `crates/rbm/src/rb20ful_window.bin` holds the 2047 bytes and
`crates/rbm/src/embedded.rs` `W`, their SHA-256 (`2c292d92...56bc`) and the SHA-256 of the DAT
they came from (`62071e99...2797`), checked against `RB_RB20FUL` by the test
`embedded_window_is_the_dats`. rb's loader (`KeyTable::embedded`, `Rbm::slots_covered`) takes a
mod whose page offsets are `W` and whose files need no more than the embedded bytes; any other mod
needs `RB20FUL.DAT`.

### 12.3 What the original accepted

On the test machine (a copy of the installation with only the four recrypted mods in its `Mods`,
`RB20FUL.DAT` from the CD as always), both forms were run the same way: the app's Mods scan raised
no box (`rb_export.py` expects the MIDI box only), the three songs opened their recrypted mods with
no Download / Standard dialog, the exports of RSI (PBE2), Fonda Workout (MSM2) and Metallicon City
(Metallicon+) each matched rbhost with the **original** mod at exactly one N among 200
(`fit.py --all`), `rb compare` with the recrypted mod and no `RB20FUL.DAT` at that N is EQUAL for
all three, and the panels display clean:

| Form | Chunks | RSI N (N - N_t) | Fonda Workout | Metallicon City |
|---|---|---|---|---|
| 123 bytes, fillers | 113663 / 98263 | 3584 (-503) | 3728 (-176) | 3648 (-444) |
| 2047 bytes (rb's) | 109 | 3600 (-33) | 3552 (+10) | 3568 (-6) |

With the original mods N - N_t was -28, -40 and -5 (section 8): the 2 KB form is back in that
range, and the 123-byte form's gap came with its 113663 chunks [G: the open mod's chunk walk
delays the export; not measured further].

All 172 third-party mods of section 11 and the four shipped ones recrypt, and every file and slot
sample of the result equals the original's (`crates/rbm/tests/rbm.rs`, `thirdparty_mods_recrypt`
and `real_mods_recrypt`); the corpus mod songs render byte-exact with recrypted mods loaded without
the DAT (`crates/rb-cli/tests/corpus_mods.rs`). `tools/rbm.py` decodes with each reader's key.

### 12.4 rb reads only recrypted mods (task-063, 2026-10-27)

At the user's request, once mods can be recrypted `RB20FUL.DAT` goes to `rb recrypt` alone:

- `rb render`, `rb compare` and `rb play` have no `--rb20ful` and do not read `RB_RB20FUL`; only
  `rb recrypt` has them (it searches: the option, `RB_RB20FUL`, an `RB20FUL.DAT` in the input mod's
  folder or its parent, next to the binary, the Windows install folders). `crates/rbm` keeps the
  general descrambler (`KeyTable::read`) for recrypt's own use.
- A mod that is not recrypted is found like any other by `--mod-dir` / `RB_MOD_DIR` (its INFO is in
  the clear, it keeps its place in the original's folder order, so the duplicate-name rule of
  task-058 is unchanged) and `--mod FILE` takes it; the error comes when its samples are loaded
  (only when a drum machine plays): `FILE: the song plays a drum machine with this mod, which is
  not recrypted. Run rb recrypt --rb20ful RB20FUL.DAT FILE -o OUT.rbm once and use OUT.rbm`
  (exit 1). The decision is "found, fails at load", not "skipped": a skipped mod would turn into
  "mod not found" and hide the fix. The future UI (m-3, task-047's mod chooser) lists such mods the
  same way, marked "needs rb recrypt".
- The env-gated tests take recrypted mods: `corpus_mods.rs` and `rb compare` read `RB_MOD_DIR`
  (recrypted); `thirdparty.rs` reads `RB_THIRDPARTY_DIR/Mods` (recrypted) with `Mods.orig` and
  `RB_RB20FUL` as the reference; `corpus_mods.rs`'s `recrypt_of_the_originals_matches_the_references`
  still runs the recrypt (`RB_MOD_ORIG_DIR`, `RB_RB20FUL`), requires its bytes to equal the files of
  `RB_MOD_DIR` and renders the 8 songs from them.

**Done on 2026-10-27.** The user's 172 third-party mods: `~/rb_stuff/Mods.orig` holds the originals,
`~/rb_stuff/Mods` the recrypted ones under the same names; all 172 recrypted (0 failures), and
`thirdparty.rs` `mods` found 16680 embedded files and 7285 slots equal to `rbm.py check-dir` of the
originals. The four shipped mods on the test machine: the originals are backed up in
`C:\re\rb\mods_orig`, `C:\ReBirth RB-338\Mods` holds the recrypted ones (hashes in
docs/test_machine.md section 7), and an app export of Fonda Workout from the installation equals
`rb compare` with the recrypted MSM2 at one N of 3400..3707 (3600).

## 13. The standard mod: the built-in kit and panel as a mod (task-036, 2026-10-27)

`rb standard-mod --rebirth-dll ReBirth.dll [--rebirth-exe Rebirth.exe] [-o FILE]`
(`crates/rbm/src/standard.rs`) builds, from the user's own files, a mod that holds what the
original has built in. rb's other commands take the built-in drum samples from it (13.4); the
original loads it like any mod. Nothing of it is committed: the tests build it from `original/`.

### 13.1 Where the original keeps its standard skin [C]

The panel graphics are **Rebirth.exe** resources of the named type `"JPEG"` (the type string at
0x4D0990, read by `sub_46E900` through `sub_484EE1`); ReBirth.dll has none (only `"AIFF"` 1000..1045
and a version resource). Rebirth.exe 2.01 has 64 JPEG resources, all language 1053, all baseline
JFIF already, i.e. the format the mods embed:

| IDs | Count | What |
|---|---|---|
| 12015 .. 12571 (not contiguous) | 58 | the panel bitmaps a mod replaces: exactly the 58 numbered JPEGs of every shipped mod |
| 903 | 1 | the built-in kit's splash (256 x 256) |
| 905 | 1 | the splash of a mod that has no `Splash.jpg` (256 x 256) |
| 900, 901 | 2 | 480 x 344, the about / start-up screens [G: not traced] |
| 904, 906 | 2 | 171 x 900 and 94 x 17 [G: not traced] |

The mapping is `sub_41703A` (the panel's bitmap loader, called with a bitmap id):
- when the open mod is not the built-in kit (`sub_4166E5` false, the branch at 0x4170C5), it asks the
  mod for `Splash.jpg` when the id is 12999 (`cmp [ebp+var_24], 32C7h` at 0x4170CB, the name at
  0x4BB368) and otherwise for `sprintf("%d.jpg", id)` (0x4170FA), through `sub_41678A` (the
  case-insensitive first-match lookup of section 3, then the exe's descrambler); a file found is
  decoded from the mod (`sub_46E818`, 0x41712C);
- otherwise (the built-in kit, or the mod lacks the file) it loads the exe's JPEG resource: id 12999
  is first changed to 905 for a mod without a splash (`mov [ebp+var_24], 389h` at 0x417188), and the
  built-in kit's 12999 loads 903 (`push 387h` at 0x417198); every other id loads the resource of
  that id.

So the mods' `<id>.jpg` is the exe's JPEG resource `<id>`, `splash.jpg` is resource 903, and a file
a mod lacks falls back to the built-in bitmap one by one.

**The CD's Rebirth.exe** (the ReBirth CD's installer, SHA-256 `e69fef41...7004`, 1,606,656 bytes;
extracted from the CAB in `Install ReBirth RB-338.exe` with `bsdtar`) and the installed one in
`original/` (`6a651e81...f876`, 1,594,880 bytes) have the same 58 panel bitmaps byte for byte; the
four splash and about screens 900, 901, 903 and 905 differ: `original/`'s carry a Steinberg logo
next to Propellerhead's, the CD's do not. So the standard mod's panel is the same from either exe,
and its `splash.jpg` is the one of the exe it was built from (the CD's gives a 2,572,792-byte mod).
Nothing of it reaches the audio.

**Demo.rbs.** Every shipped mod carries one; the standard mod does not. The demo song is optional:
`sub_417000` (`sub_416710("Demo.rbs") != 0`) only enables a button of the mod info dialog
(`sub_41A0C5`, 0x41A3E0) that saves it (`sub_419E92` -> `sub_417022`), and the built-in kit has
none. The exe's two `"SONG"` resources are not demo songs: 128 (15,027 bytes, read by
`sub_4098EB`) and 129 (141,008 bytes, an RB40 CAT, read by `sub_401EE0`, the new-song template); the
ReBirth 1.0 Default Song is a file of the installation, not a resource.

### 13.2 The mod's layout

```
"CAT " PRBM
  HEAD  256    5B 35 B3 5B BC 01 00 00 00, "(c)1998 Propellerhead Software, all rights reserved.",
               zeros (as every shipped mod)
  EMBF         rb-recrypt.pad (1024 bytes, section 12.2)
  EMBF x 58    12015.jpg .. 12571.jpg: Rebirth.exe's JPEG resources in ID order, unchanged
  EMBF         splash.jpg: JPEG resource 903
  EMBF x 46    TR808BD.aif .. TR808RS.aif, TR909bd1.aif .. TR909rcy.aif, Startup.aif: ReBirth.dll's
               AIFF resources 1000..1023, 1025..1045, 1024 (the slot order), unchanged
  INFO  1280   name "ReBirth Standard Kit", version BC 01 00 00 (1.0.0, as PBE2, MSM2, Orbit2),
               a description, no ftp URL, web http://www.propellerheads.se/
```

108 chunks, 106 EMBF (under the 200 of OB-14, every ordinal under 256). It is laid out in the clear
and passed through `rbm::recrypt` with an all-zero table (the identity), so it is scrambled against
the embedded window exactly as `rb recrypt` output: both key pages are `W`, rb loads it without
`RB20FUL.DAT`, the original with its own. From `original/`: 2,574,060 bytes, SHA-256
`2d732978...f3e5` (the build is deterministic).

The menu name is **"ReBirth Standard Kit"**: "Standard ReBirth" is the name the original and rb give
the built-in kit itself (`sub_4166E5`; GLOB name of a song without a mod), and a mod of that name
would be confused with it (a song naming "Standard ReBirth" never opens a mod).

### 13.3 What the original does with it (test machine, 2026-10-27)

A copy of the installation in `C:\re\rb\t036` with only the standard mod in its `Mods`: the app
started with no extra box, the Mods menu listed `Standard ReBirth` and `ReBirth Standard Kit`, and a
song whose GLOB names "ReBirth Standard Kit" opened with no Download / Standard dialog, the mod
checked in the menu and the panel drawn clean. `rb_export.py --app-dir` exported `808_snare` and
`909_snare` (diag) with GLOB "ReBirth Standard Kit" and, the same session, unchanged ("Standard
ReBirth"): `rb compare` (the built-in kit from the standard mod) is EQUAL for each of the four WAVs
at one N of 2000..12000 (2704 and 2704 with the mod, 2768 and 2784 with the built-in kit; the N is
the app's start state, docs/export_state.md section 13). So each export with the standard mod
equals rb's render with the built-in kit at its own fitted N; the two app WAVs of one song (mod and
built-in) differ from each other, as two exports of one song do at different start states. What
was shown is that the samples the app plays from the standard mod give the built-in kit's output.

### 13.4 rb takes the built-in kit from it

`rb render`, `rb compare` and `rb play` read the 46 built-in samples from the standard mod
(`--standard-mod PATH`, then `RB_STANDARD_MOD`, then `Standard.rbm` next to the rb binary, then the
first mod named "ReBirth Standard Kit" in the mod folder), never from ReBirth.dll: `--rebirth-dll`
and `RB_REBIRTH_DLL` are gone from them, and `rb standard-mod` is the one command that reads the DLL
(and the exe). The mod must be named "ReBirth Standard Kit", be recrypted and have all 46 slots
(`Rbm::load_all_samples`). A third-party mod's missing slots and `--standard-kit` take the standard
mod's samples. The corpus is unchanged: 164 of 164 EQUAL, 175 of 175 with `RB_MOD_DIR`.

## 14. Changing the mod of an open song: the Mods menu (task-056, 2026-10-27)

The Mods menu does not make a new song: it sends the song **message 19** `{u16 mod id}`, and the
DLL swaps the sample table of the song's TR-808 and TR-909 in place, after silencing their
voices. The song plays on. [C: read in the disassembly; run in the DLL by rbhost, section 14.4.]

### 14.1 The exe [C]

- **The commands.** "Select Mod..." (403) is handled by the song window's command handler
  `sub_42C204` (`cmp ..., 193h` at 0x42C294 → `sub_41A482`, which runs the modal "Mod Selector"
  dialog `CModSelectorWindow`: ctor `sub_419460`, vtable 0x4BB69C, command handler `sub_41A0C5`).
  The menu's one item per mod is command `5000 + i` (0x1388..0x144F), handled by the main frame's
  `sub_40C163` (0x40C427..0x40C489): the active song window (`sub_40B4A6`), the mod object
  `sub_415578(i)`, then `sub_42D80D(window, mod)` and `dword_4DA788 = mod`.
- **The dialog.** It lists the mods and preselects the one whose INFO name equals the song's
  mod name (`strcmp` at 0x4199A2; the name from 0x4197A8..0x4197D0: the window's document, its
  open mod `[doc+4]` (`unknown_libname_8` at 0x426F72 is `mov eax,[ecx+4]`), and `sub_41691A`,
  which copies the open mod's name at `+1595`, `add eax, 63Bh` at 0x416924).
  Selecting an entry already opens that mod (`sub_416300` at 0x41A16A, releasing the one opened
  before, `sub_4165BE` at 0x41A13E) and plays its startup sound (`sub_426C67` at 0x41A180). OK
  (0x41A2B6..0x41A305) calls `sub_42D80D(window, mod)`, stores `dword_4DA788` and closes the
  dialog (`sub_47350E`).
- **Opening a mod** (`sub_416300`): the mod object's word `+1088` counts its users; the first
  one builds the open mod (`sub_41813E` for a file mod, `sub_417AE9` for "Standard ReBirth"),
  whose `sub_41693C` sends top-level 55 (the samples, section 6) or 56 (id 411). Releasing
  (`sub_4165BE`) decrements it and, for the last user, keeps the open mod in a one-deep cache
  (`this+8`), deleting the one cached before (`sub_4162BC`); `sub_416300` takes a mod back from
  that cache when its name matches (0x41634F). The file mod's destructor `sub_418A87` sends
  top-level 57 `{id}` (`sub_416EF9`, called at 0x418EA9), which the DLL's `sub_1001DB60` answers by
  freeing the mod (`sub_1001A6B0`) and clearing the startup-sound preview (`engine+0x428`).
- **`sub_42D80D(window, mod)`**: when the song document's mod (`sub_426F83`, the open mod at
  `doc+4`) is another one, it calls `sub_426FB3(doc, mod)`, then detaches the window from the
  document (`sub_42D7F7(0)`), destroys it (`sub_47350E`) and builds a new song window for the
  document (`sub_42DA72(doc, 0, rect)`, at the old window's place), which draws the panel with the
  new mod's bitmaps (section 13.1). The new window's activation (`sub_40D163` at 0x42E5AE) sends
  top-level 53 only when the document is not already the current one (0x40D178), and it is, so no
  msg 53 is sent. The same mod again does nothing.
- **`sub_426FB3(doc, mod)`**, in this order: the document is marked modified (`sub_42590B`);
  `doc+4` = the new mod opened (`sub_416300`, which sends 55 or 56 if it was not open); **song
  msg 19 `{u16 BE id}`** to the document's song (`sub_4259FF`: `sub_465F5E` packs the id,
  `push 13h` at 0x425A31, sender `sub_40B1E3`, then the reply wait `sub_40ABE5`); the old mod
  released (`sub_4165BE`); the startup sound (`sub_426C67`: top-level 58 when the mod's `+1088`
  is 1 and "Play Mod Startup Sound" is on, docs/export_state.md section 8).
- **The save writes the new name** [C for the source of the name]: the save `sub_4270F7` copies
  the name of the document's open mod (`mov ecx,[ecx+4]; call sub_41691A`, 0x427522..0x42752B;
  `sub_41691A` copies from `+1595`, 0x416924)
  and the mod's INFO URLs (`+0x4A9`, `+1394`) and passes them to the RBS writer `sub_438025`
  (called at 0x42758A), which writes the GLOB chunk (0x43817C). [G: the GLOB field the name
  lands in was not traced through `sub_438025`'s copies.]

### 14.2 The DLL [C]

- The song-message handler `sub_10011ED0` (table `0x10151D6C + 4*msg`) takes msg 19 at
  0x10011F67: it does not act, it forwards the message as top-level **78** `{slot, 19, id}` (the
  bytes 'N', the song's slot `+0x178`, then the message, built at `byte_100DC7B0`) to the queue the
  idle loop drains (`sub_1001D690`, which falls into `sub_1001D694`, the queue of msg 52).
- The idle dispatcher `sub_1001D6D4` (table `0x101531F8 + 4*msg`) runs 78 at 0x1001D7B9:
  `sub_10012940(song)` with the song `engine+0x14 + 4*slot`, whose case 19 (0x1001296D) reads the
  BE id (`sub_10004060`), finds the loaded mod by id (`sub_1001A860`, the list `dword_10118280`;
  none found: nothing happens) and calls **`sub_10011D84(song, mod)`**, then replies 39.
- `sub_10011D84`, under the song's lock (`song+0x154`, `sub_10009480` / `sub_100094A0`): the
  song's mod pointer `+0x174` = mod (0x10011DA6); the first device of type 3 in the song's list
  (`+0x18C`, count `+0x1C4`; the 808) gets `sub_10028290(mod)`, the first of type 4 (the 909)
  `sub_1002B8C0(mod)`.
- `sub_10028290`: vtable slot 6 (`mov eax,[eax+18h]; call eax`, 0x10028297: the 808's slot 6,
  every voice silenced), then `+0x18D8` = mod (0x100282A0), the 808's sample table. `sub_1002B8C0`
  is the same for the 909: slot 6, then `+0x18DC` (0x1002B8D0).

So: **no new song, no msg 52, no msg 53.** The transport, the position, the play flag, the
patterns, the 303s, the effects, the mixer and the process globals (the shared delay line, which
only the delay's slot 6 clears; the drum counters) are untouched; voices that sounded stop at once
(no release), and the next step plays from the new samples. A song that plays keeps playing.

[G, not traced] With more than one song window open, destroying the old window may let Windows
activate another song's window (`sub_42D7BE` → msg 53 for that song) before the new window
activates this one again (msg 53 back), which on a stopped song runs slot 6 on both songs
(docs/export_state.md section 3). rb's UI has one song, so this is not modelled (task-075).

### 14.3 rb

`Engine::set_samples` (crates/rb-engine/src/frame/mod.rs) is `sub_10011D84`: the 808, then the
909, each `ChannelDevice::set_samples` = slot 6, then the table. The real-time player sends it as
`Command::Mod` (`Player::set_mod`, crates/rb-player), to the newest song as every command. The
audio callback frees nothing: the command's box (`ModSwap`) carries a fixed-capacity
`samples::Retired` that takes the old tables and the samples the silenced voices still held, and
goes back to the UI thread through the garbage queue. The original's voices keep their old sample
pointer; rb's voices let go of it, which changes no output (an inactive voice reads nothing, and
every 808 and 909 trigger passes a sample). Tests: the engine's
`set_samples_silences_then_replaces_the_table` (808, 909); the player's `mod_change.rs` (a mod
set on a stopped new song equals building the song with it; on a playing song the tick runs on as
without the change, the drums go silent at once with a silent kit and sound again with the
built-in one); `no_alloc.rs` (the change twice while playing, the first kit freed on the UI side
before the voices retrigger). The startup sound (msg 58, the preview player) is not modelled by
rb-player (task-074). The check against the DLL itself is section 14.4.

### 14.4 Against the DLL (task-076, 2026-10-27) [C]

rbhost (`tools/rbhost`, `--live`) ran three sessions through `ReBirth.dll` 2.01 on the test
machine with song msg 19 `{78, slot, 19, id_hi, id_lo}` written to the port before given
1024-frame calls (dispatched by the 0-frame `SafeSoundCallback`, which forwards it as top-level 78,
then `EngineIdle`, which runs `sub_10012940` case 19 → `sub_10011D84`, before the call renders).
The mods were loaded first: the song's with `--mod-msg` (msg 55) or the built-in 56 {411}, the
others with the new `--load-msg` (msg 55 of the original `.rbm` made by `tools/rbm.py msg` with
the id, or 56 {411}, hex `38 01 9B`). Each session: idle 0, pattern mode and play at call 0, 400 calls.

| Session | Song | Built with | Changes (call: id) | WAV sha256 |
|---|---|---|---|---|
| `mod_kit` | `drums_kit` (808 and 909) | 411 | 100: 412 PBE2, 200: 413 Orbit2, stop at 270, 290: 411 (stopped), play at 300 | `0ded17da…` |
| `mod_808` | `808_cymbal` | 412 Metallicon | 150: 413 MSM2, 250: 411 | `3954a36b…` |
| `mod_909` | `909_crash` | 412 Orbit2 | 150: 411, 250: 413 PBE2 | `cf414eb6…` |

rb-player's render of the same sessions (`Player::set_mod` with the recrypted mod's samples
before the same call, `crates/rb-player/tests/live_messages.rs`) has the same hashes: byte for
byte equal, on the first run, with no change to rb. The same sessions without the changes render
otherwise in rb (the test checks it), so the changes are audible: the mod swap while playing
(slot 6 on the 808 and the 909, then the new table, the transport untouched), while stopped, to
and from the built-in kit and between two file mods all do in rb what they do in the DLL. Not
covered: the startup sound (msg 58, task-074) and more than one open song (14.2, task-075).

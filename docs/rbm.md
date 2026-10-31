# RBM mods: the container, the key table and the sample lookup

How ReBirth RB-338 2.01 reads a mod (`.rbm`) and hands its samples to the engine, found out in
task-030 (2026-10-27). Addresses `0x4xxxxx` are `Rebirth.exe` (the IDA database `Rebirth.exe.i64`),
`0x10xxxxxx` are `ReBirth.dll`. [C] marks what was read in the disassembly and checked on real
files; [G] marks an inference. The reference implementation is `tools/rbm.py` (tests:
`tools/rbm_test.py`); `tools/rbhost` loads a mod into the original DLL (`run.py --mod`).

RB20FUL.DAT, its pages, any key table and the mods themselves are Propellerhead material: they stay
on the user's machine and are never committed. The tests read them from the CD image (`ORB_CD_ISO`:
the crate `orb-testkit` extracts the DAT and the four shipped mods and recrypts them, each checked
against a pinned SHA-256; task-121); the orb CLI reads only recrypted mods (`ORB_MOD_DIR`,
section 12.4).
The one exception, decided by the user on 2026-10-27 (task-059): the 2047-byte window of section
12 is embedded in orb (`crates/rbm/src/rb20ful_window.bin`, `embedded.rs`).

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
- `orb mod recrypt` rewrites a mod so that it needs only the 2047 bytes of `RB20FUL.DAT` that orb
  embeds; the original loads the result unchanged (section 12).
- `orb mod pack-standard` builds the built-in kit (ReBirth.dll's 46 AIFF resources) and panel
  (Rebirth.exe's JPEG resources) into such a mod, "Standard ReBirth"; orb embeds the one committed
  as `assets/Standard.rbm` and takes the built-in samples from it (section 13).
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
and the test `RealDat` of `tools/rbm_test.py`, run by the cargo test
`the_python_tool_tests_pass_on_the_real_files` with the DAT of `ORB_CD_ISO`): `((g_k & 0xFFFFF000) >> 14)
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

Consequences for orb (task-032, task-037): the table of a mod is a pure function of the mod file and
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
- The DLL's AIFF chunk walk trusts every chunk size and reads its memory stream with no bound
  check (section 11.2; `docs/original_bugs.md` OB-22).
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
`RB20FUL.DAT`, `Rbm::file`, `Rbm::load_samples`). Its tests (gated on `ORB_CD_ISO`)
compare every embedded file and the 46 samples of the four shipped mods with `tools/rbm.py extract`.

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
  Download / Standard / Cancel dialog (docs/song_load.md section 2). **orb's rule** (the user's
  decision on task-145, 2026-10-29; task-150): a song's mod is the first mod in the folder order
  (section 11.3) whose INFO name equals the song's name **ignoring ASCII case**
  (`eq_ignore_ascii_case`: a non-ASCII letter compares exactly, so `ÇoNaNbox 2.0` does not match
  `çoNaNbox 2.0`), and never a file name (`PBE2` or `pbe2.rbm` find nothing). This is a
  **deliberate difference from the original's `strcmp`**. Every orb user of the rule goes through
  `mods::find_in_dir` (`orb render`, `compare`, `play`, `info`, orb-ui's song open and Mods menu
  check). On the user's collection: 509 song names match an INFO name exactly, the 2 `omega`
  songs match `Omega` only ignoring case, and 0 need the file name. `--mod FILE` stays the
  explicit override; it warns when its INFO name is not the song's name by the same comparison
  (no warning for a case-only difference). Which of several matching mods is used: the first by
  file name bytes (section 11.3, task-151). [G: the value `strcmp`'s result is compared with comes from the anti-tamper pages
  and is 0; not traced.]
- **Message 55 and rendering**: of the 9542 bytes, only the 46 sample records (and the key table
  and file spec that locate and descramble them) reach the sample table; the mod id only selects
  the mod for message 52 (and 58, the preview player). So the engine's input from a mod is its
  46-slot `SampleSet` (a slot the mod lacks keeps the built-in sample). Confirmed by output: with
  only the samples modeled, all 8 `mod` and 3 `modapp` references render byte-exact.
- **CLI**: `orb render` / `orb compare` take `--mod-dir DIR` (`ORB_MOD_DIR`; the old alias `RB_RBM_DIR` was
  dropped in task-068), `--mod FILE` (overrides the song's mod) and `--standard-kit` (a missing mod
  renders with the built-in kit and a warning). Without it a song naming a mod that is not found
  is an error. A mod is read only when a drum machine plays (the mod holds nothing else).
  `orb compare --manifest` includes the sets `mod` and `modapp` by default only when a mod folder
  is given. Since task-063 these commands take no `--rb20ful` and read no `RB20FUL.DAT`: they
  load recrypted mods only (section 12.4). Task-033 had `--rb20ful PATH` / `ORB_RB20FUL` here.
  Since task-036 the built-in kit comes from the standard mod, not from `--rebirth-dll`; since
  task-067 from the one orb embeds, with no option (section 13.4).
- **AIFF rate** [C]: `sub_10001674` (0x100016B2..0x100016DE) converts the 80-bit rate with `fistp`
  under a control word with rounding set to truncate (`| 0xC00`) into a 64-bit slot and keeps the low
  dword; `sub_100016F4` rejects 0. The engine's reader now does the same (a rate below 1 Hz, or
  with zero low 32 bits, such as 2^32, or one that does not fit in 63 bits, is rejected).
  [G: the slot is a qword, from the 8-byte stack variable; a dword `fistp` would differ only for
  rates of 2^31 and up, which no mod has.]

## 11. A third-party collection (task-058, 2026-10-27)

The user's collection of 2026-10-27 (`~/rb_stuff`: `Mods/`, 172 `.rbm`; `songs/`, 193 distinct
songs; and a large song archive, `CaveysRebirthStuff/`) is third-party material: it is read in place
and nothing of it is in the repository. Mod names and counts only below.

**The folder since 2026-10-29.** `~/rb-mods/mods` holds 171 recrypted mods: the user removed
DJ Turn's 8-Track.rbm (a byte-identical copy of 8tRack.rbm, with the same INFO name) on 2026-10-29,
and its entry was dropped from `tests/refs/thirdparty_mods.json` (task-149). The counts in this
section are the folder's now (171), except where a run is dated: the 2026-10-27 and 2026-10-28 runs
of the 172-mod collection, which had that file, are history.

**The folder since 2026-10-30 (task-165).** The user grew `~/rb-mods/mods` to 311 recrypted mods
(142 added; BadRat, DarkStar, digalog and noize replaced; PeffEdit.rbm and `degenerate
oxidized.rbm` deleted; TechnoBox.rbm renamed `TechnoBox mkII.rbm`). The manifest
`tests/refs/thirdparty_mods.json` lists all 311. The originals were not kept, so they were
found again by content: every `.rbm` of `rb_stuff.zip` (`Mods/`) and of `rb-stuff-peff/ReBirth
Mods` (subfolders included, 535 files) was recrypted with the current orb, and each collection
file took the original whose recrypted bytes equal it (all 311 found; 4 of the 535 candidates
fail to recrypt and are not collection mods). `tools/thirdparty_mods_manifest.py --check` over
those originals reports `equal`. The 171-mod counts in the rest of this section are those of
the 2026-10-29 folder.

**The folder since 2026-10-31 (task-168).** The user added 9 mods (`Modyssey`, `PhaseII.2`,
`Polar a1`, `Polar v`, `Polar vx`, `Polar vx3`, `Polar vx3a`, `Polar vx4`, `R-SP 1200.2`): 320
recrypted mods, none removed or changed. The originals were found again by content as in
task-165 (534 candidates, 3 of which fail to recrypt and are not collection mods; all 320
found), the manifest lists all 320, and `--check` over them reports `equal`.

**The check.** `tools/rbm.py check-dir DIR RB20FUL.DAT [--json]` (the reference: every mod's
parse, key pages, every embedded file descrambled, the 46 slots' AIFFs, JPEG end markers,
duplicate INFO names in the original's folder order) and the env-gated cargo test
`crates/orb-cli/tests/thirdparty.rs` (`ORB_THIRDPARTY_MODS_DIR`, the folder of the recrypted mods, and
`ORB_THIRDPARTY_SONGS_DIR`, the folder of songs; since task-121 the test needs no `Mods.orig/` and
no DAT; run it with
`cargo test --release -p orb-cli --test thirdparty -- --nocapture --test-threads 1`), whose `mods`
test compares `crates/rbm` with the committed manifest `tests/refs/thirdparty_mods.json` (the
output of `check-dir --json` reduced to hashes and names, plus the SHA-256 of each recrypted file:
the SHA-256 of every embedded file, the channel-0 PCM of every slot) and whose `songs` test reads,
writes back and renders every distinct song. The manifest is written by
`tools/thirdparty_mods_manifest.py --orig MODS.ORIG --mods MODS --dat RB20FUL.DAT`
(on 2026-10-28: 172 mods, 16680 embedded files, 7285 slots, equal to the 2026-10-27 run of the old
test, which took the originals and the DAT from the environment); regenerate it when the
collection changes (the test fails on a `.rbm` it does not list). The tool first recrypts the
originals with the current orb and requires every result to equal the file of `Mods/` byte for
byte, so the manifest pins what the recryptor makes of the originals (2026-10-28: all 172 equal).
To re-verify the recryptor on the real collection at any time, without the user's `Mods/`:
`tools/thirdparty_mods_manifest.py --check --orig MODS.ORIG --dat RB20FUL.DAT` recrypts the
originals and compares every result with the manifest (exit 1 on a difference). No file content
is in the manifest.

### 11.1 Mods

- **All 171 load**, in `rbm.py` and in `crates/rbm`, with the same 16591 embedded files and the
  same 7255 slot samples (on 2026-10-28, 172 mods with 16680 and 7285, before the removal). Every
  one is a `CAT ` of type `PRBM` with `HEAD` first and a 1280-byte `INFO` last, only `EMBF` between,
  under 256 chunks, version major 1 (or zero), no duplicate EMBF name. Every slot AIFF is mono,
  16-bit, 44100 Hz. [C]
- **The key table is right for all of them**: the mean |x[n] - x[n-1]| of the descrambled slot
  audio past byte 1024 is 180..1710 per mod, against 20328..22071 for the stored bytes; and the
  rbhost renders of section 11.4 load the samples through the DLL's own descrambler. [C]
- **53 mods leave slots built in** (1 to 46 of the 46): the original sends `present = 0` and the
  DLL loads the built-in sample (section 3); `orb` does the same (`Rbm::load_samples`). One mod,
  ReBirth 666, holds no sample at all (panel graphics only): it plays the built-in kit.
- **Stale AIFF sizes** (2 mods in the folder: 8tRack and DJ Turn's MOD; on 2026-10-28 a third,
  DJ Turn's 8-Track, a byte-identical copy of 8tRack, had it too, and the user removed it on
  2026-10-29): their `TR808BD.aif` has FORM and SSND sizes smaller than the audio that follows
  (COMM 21064 frames, SSND 15798; 18052 and 13539). `crates/rbm` rejected the sample; the DLL plays
  it whole (section 11.2). Fixed in the engine's AIFF reader.
- **JPEGs without an end marker** (1 mod, YellowBirth): its 59 panel JPEGs end without `FF D9`
  (progressive JPEGs cut short by its packer [G]); `splash.jpg` decodes with a garbled edge. The
  samples are fine, and the engine uses none of the JPEGs.
- **Duplicate INFO names** (5 pairs in the 2026-10-28 collection; 4 in the folder since 2026-10-29,
  when the user removed DJ Turn's 8-Track.rbm, the copy of 8tRack.rbm, so 8tRack has no duplicate
  left): AXIOM (`AXIOM.rbm`, `axiom(b3).rbm`), dejenerate (`dejenerate.rbm`,
  `dejenerate(b3).rbm`), Alpha 2.0 (`Alpha2.rbm`, `alpha3.rbm`), AlpJunUV (`AlpJunUV.rbm`,
  `UVJ.rbm`). The removed pair was 8tRack (the two identical files). Which file the original keeps
  is section 11.3.

### 11.2 The AIFF audio is read past a stale SSND size [C for the mono read]

`sub_10002FC4` (the SSND reader of `CAIFFReader`) finds SSND and reads its 8-byte offset and block
size (`sub_10004F00`, `sub_10001774`), skips the offset (vtable +0x2C), and for a mono file reads
`2 * frames` bytes (0x10003060 `lea eax,[ebx+ebx]`, the read at vtable +4, 0x1000306F; `frames`
from COMM) into the sample buffer; nothing compares that with the SSND size. The stream is the
whole descrambled payload (section 4), so the audio past the stale size is there and is played.
Confirmed with `rbhost --dump-samples` and 8tRack.rbm / DJ Turn's MOD.rbm: slot 1 is 42128 and
36104 bytes, equal to the file's audio sample for sample. `samples/aiff.rs` now reads the audio
through to the end of the file, as it already did COMM (section 9). `docs/original_bugs.md` Q-3.

**The chunk walk stops at the wanted chunk (task-155, 2026-10-30) [C].** On 2026-10-30, four
mods of a 317-mod folder of the user's (`~/rb-stuff-peff/ReBirth Mods`) failed `orb mod recrypt`
because the AIFF reader walked every chunk to the FORM's end: in Black_Steel.rbm (Startup.aif),
Black_Steel_2.rbm (TR808MT.aif) and O.S.C.rbm (Startup.AIF) the SSND size is 8 too small (it leaves
out the 8-byte offset and block-size header; Black_Steel: 226799 frames, 453598 bytes of audio =
the SSND size, ending at the file's end, 453652), so the last 8 bytes of audio read as a chunk header
with a huge size; mellow.rbm (Startup.aif) has COMM, INST (20), a correct SSND, then 20 stray bytes
inside the FORM. The DLL never reads those bytes. From the disassembly of the CD's ReBirth.dll:
- `sub_10004F94` (find a chunk; called through 0x10004F40 / 0x10004F44 by the COMM reader
  `sub_10003170`, 18 bytes of id 0x1014F590 "COMM", and through 0x10004F00 by the SSND reader
  `sub_10002FC4`, 8 bytes of id 0x1014F5A0 "SSND") **starts every search at the file's start**: it
  seeks to 0 (vtable +0x14, 0x10004FD2), reads the 12-byte header (vtable +4, 0x10004FF2) and checks
  it with `sub_100054D4`: the id equals "FORM" (the reader's this+6, copied by the constructor
  0x10004EB0 from 0x1014F600) and the FORM size is at least 4 (unsigned, 0x1000550C `jae`). The
  FORM type is decoded (0x10004A84) but never compared, so AIFC or any other type passes. A failed
  check throws (0x1000504F).
- The first chunk header is read at 12 with no bound check (0x10005071, `sub_10005474`: 8 bytes,
  the id with a NUL by 0x10003FD0, the size big-endian by 0x100040B0).
- **It stops at the first chunk with the id** (`strcmp` 0x10005084..0x100050AB, `je 0x100051AD`):
  it stores the size, skips the requested offset (vtable +0x2C) and reads the requested bytes
  (vtable +4, 0x100051C6..0x100051FB). Nothing past that chunk is read, so junk after SSND (or after
  COMM, when COMM is the later one) is never seen.
- On another id (0x100050B1..0x100050D6): `pos = pos + size + 8`, +1 when the pad flag this+4 is set
  and the size is odd, in 32-bit wrapping arithmetic; then `lea eax,[ebp+8]; cmp eax,[end]; jge`
  with `end` = FORM size + 8 (0x1000506A): a header with `pos + 8 >= end` (signed) is not read and
  the search fails, returning 0; else seek(pos), read 8 bytes, compare again. A chunk size is
  never compared with the stream.
- So **a damaged chunk before the wanted one** sends the walk to `pos + size + 8`: past the FORM's
  end, the chunk is "missing" and the reader throws (`_CxxThrowException` 0x1002E522 with type
  0x10030420: COMM 0x10003209..0x1000321B, SSND 0x10002FFE..0x10003018); inside the FORM but past
  the buffer (a FORM size larger than the file), or at a wrapped negative position, it reads
  outside the buffer (below); a size of 0xFFFFFFF8, or 0xFFFFFFF7 with the pad, leaves `pos`
  unchanged and the loop never ends. `docs/original_bugs.md` OB-22.
- The SSND offset (0x1000302D..0x10003033 `test eax,eax; jle`) is skipped only when it is positive
  as a signed 32-bit value.
- The stream of a mod sample is a memory stream (constructed at 0x1001A563 by 0x10006FD0, vtable
  0x10030824, a buffer of the payload's size): read 0x10007180 is a bare `memcpy` from buffer +
  position (`rep movsd` / `rep movsb`), seek 0x10007200 stores the position, tell 0x1000DEE0
  returns it, skip 0x10006E60 is seek(tell + n). **No bound check anywhere**.

`samples/aiff.rs` (`find`) follows the walk: one search per id from the start, the FORM check
(the type ignored), the first header at 12, the step and the signed `pos + 8 >= end` stop, the
signed SSND offset. Where the DLL would read outside the file (a header, the 18 COMM bytes or the
audio) orb rejects the sample, and a walk that would not end is an error too. All four mods
recrypt, and the 317-mod folder gives 317 recrypted, 0 failed.

Open (not checked against the DLL):
- **A file that really ends early.** The read's result is ignored (0x10003071 goes straight on to
  `sub_10009990`), and the memory stream copies past its buffer with no check (above), so the
  DLL reads whatever heap bytes follow the payload, or faults [G: the effect]; no mod in the
  collection reaches it. orb's policy is an error, and `Rbm::load_samples` propagates it, so the
  **whole mod** is refused rather than that one slot falling back to its built-in sample.
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
- **orb's rule: file name bytes** (the user's decision on task-146, 2026-10-30; task-151). orb
  does not model the volume. Of the mods that share an INFO name (ASCII case ignored, section
  10), the one whose **file name sorts first by bytes** (`strcmp` over the name's bytes:
  `Path`'s `as_encoded_bytes`, no Unicode folding, no locale) is the default, in the CLI and in
  the UI. `mods.rs` `byte_order` is that order; `scan` returns the folder in it, so the UI's Mods
  menu lists its entries in it too (one order everywhere). `AXIOM.rbm` wins over `axiom(b3).rbm`
  (`A` 0x41 < `a` 0x61), `Alpha2.rbm` over `alpha3.rbm`, `AlpJunUV.rbm` over `UVJ.rbm`,
  `dejenerate(b3).rbm` over `dejenerate.rbm` (`(` 0x28 < `.` 0x2E). **This is a deliberate
  difference from the original**, where the volume's listing order decides (on NTFS
  `axiom(b3).rbm` wins). The earlier NTFS model (`folder_order`, names upcased per character then
  UTF-16 units) is gone. `--mod FILE` still picks any file, and the CLI reads no UI setting, so a
  render is deterministic. The reference check of task-129 keeps the original's file: its
  manifest records the mod file rbhost used (`"mod"`), and the gated songs test renders each
  such song with `--mod <folder>/<that file>` (`axiom(b3).rbm`, not the new default
  `AXIOM.rbm`). The UI's variants and preference: docs/ui_spec.md section 18.
- The user's collection (171 mods) has 4 shared names: AXIOM (`AXIOM.rbm`, `axiom(b3).rbm`),
  dejenerate, Alpha 2.0 (`Alpha2.rbm`, `alpha3.rbm`) and AlpJunUV (`AlpJunUV.rbm`, `UVJ.rbm`).

### 11.4 Songs and the reference check

- `songs/`: 193 distinct songs by SHA-256 (208 files whose name contains `.rbs` when the task
  started, with "... copy" duplicates; 195 after the user's cleanup during it). At the review
  (same day) the folder had grown to 206 distinct songs, 77 RBS 4.2 and the same 129 1.x: the
  check passed again, every mod resolving by its exact INFO name (51 Standard, 14 PBE2, 5 MSM2,
  2 Poison-Box, 1 each of Regulation Issue, Orbit 2.0, Nord Beat, Metallicon+, RedTop 2.0). The
  counts below are those of the first run. **64 are RBS 4.2**: every one reads and writes back byte-identical through
  `crates/rbs` and renders with `orb render --mod-dir` without a fault. Their mods: 46 Standard
  ReBirth, 12 Pitch Black Edition 2.0, 2 Massen's MSM 2.0, 2 Poison-Box, 1 each Orbit 2.0, Massen
  RedTop 2.0, Regulation Issue, Nord Beat; all resolve by the exact INFO name (none missing, none
  by case only).
- **129 are ReBirth 1.x songs**: a Standard MIDI File (`MThd`, a meta text "File version 3.1.0")
  named `.rbs`. The original imports them (`sub_425C4D` tries `sub_43037D` first: the MIDI file and
  its "File version %s" text, `sub_431341` at 0x431481; then `sub_430BA0` loads it with the built-in
  mod, 0x425FCE); `orb` refused them with an error until task-060, which imports them as the exe
  does (`docs/song_load.md` section 12).
- **rbhost**: 49 songs from `~/rb_stuff` (mostly the archive, where the mod songs are) over 29
  third-party mods, each with the mod its GLOB names and where that mod is audible (the render
  differs from the built-in kit's). 47 rendered with `run.py --mod --rb20ful --idle-blocks 9088`
  and **all 47 equal `orb render` byte for byte** (27 mods). The other 2 (the RB-SEMx and TB-909e
  mkII songs) are RBS 4.0 files, which `tools/rbhost/rbsmsg.py` could not convert at the time;
  task-061 added the 4.0 mixer converter and both then render with rbhost equal to `orb render`
  byte for byte (49 of 49). Also exact: AXIOM.rbs with `AXIOM.rbm` (it differs from the
  `axiom(b3).rbm` render), and two songs each with 8tRack and DJ Turn's MOD (the stale-size
  samples; GLOB renamed in scratch copies).
- **The app** (`tools/rb_export.py --app-dir`, a copy of the installation in `C:\re\rb` with its
  own `Mods`; `--startup-boxes 2` for the duplicate-name box; `--restore-prf --startup-sound
  off`) exported AXIOM.rbs (AXIOM, both AXIOM files present), Commutation (Trancer) and Meth X
  State (Poison-Box), each mod found by the app's own scan. `fit.py --all` (200 candidates) found
  one N each, 3616, 3600 and 3632 (N - N_t = -71, -5, -9), and `orb compare --mod-dir` at that N is
  **EQUAL** for all three. AXIOM.rbs against `AXIOM.rbm` matches at no N: the app used
  `axiom(b3).rbm`, as section 11.3 says.
- `run.py` and `fit.py` take `--remote-dir NAME` (work in `C:\re\rb\NAME`), `rb_export.py`
  `--app-dir` and `--startup-boxes`.

### 11.5 Every song against rbhost: the songs manifest (task-129, 2026-10-29)

The collection moved to `~/rb-songs/songs` (songs, read recursively: the top level and the
`CaveysRebirthStuff` archive) and `~/rb-mods/mods` (320 recrypted mods since 2026-10-31; 311 on
2026-10-30; 171 on 2026-10-29, after the user removed DJ Turn's 8-Track.rbm; 172 before). The
user asked (2026-10-29) that all songs be handled recursively.

**The manifest.** `tests/refs/thirdparty_songs.json` holds, per distinct song (by SHA-256), its
paths under the folder, its form and version, the mod name and the mod file the original
resolves (null: the built-in kit), the idle blocks and the SHA-256 of rbhost's render (the WAV,
hashed on the machine); then the skipped files with the reason. Hashes and names only.
`tools/rbhost/thirdparty_songs.py --songs SONGS --mods MODS --rb20ful RB20FUL.DAT` writes it:
- RBS songs: `rbsmsg.py`'s messages, `--idle-blocks 9088`, the mod's message 55 as `run.py --mod
  --rb20ful` builds it (the recrypted mods load in the DLL with the CD's DAT);
- 1.x songs: `tools/rb1x.py`'s messages, idle 0, the built-in kit.
- The mod is the first file in NTFS order (rbhost's volume; `tools/rbhost/thirdparty_songs.py`
  `ntfs_key`) whose INFO name is the GLOB's, exactly (`strcmp`, section 10); orb's own default
  differs for AXIOM (`AXIOM.rbm` by bytes, `axiom(b3).rbm` on NTFS; for dejenerate both orders
  give `dejenerate(b3).rbm`; section 11.3), so the test passes the recorded file with `--mod`; `Standard ReBirth` or a name no INFO has is the built-in kit.
- It works in `C:\re\rb\t129` (deleted after each batch) in batches of 100 songs, each under one
  permit with no Rebirth.exe running. The manifest is rewritten after every song, and a song it
  already records is not rendered again, so a stopped run resumes (`--fresh` starts over,
  `--only PATH...` renders songs again). The DAT comes from the CD image (`bsdtar -xf CD.iso
  RB20FUL.DAT`).

The gated `songs` test (`crates/orb-cli/tests/thirdparty.rs`) renders every listed song with
`orb render --idle-blocks N --mod-dir ORB_THIRDPARTY_MODS_DIR --standard-kit` and requires the
WAV's SHA-256 to equal the manifest's; a song file of the folder the manifest does not list fails
it, and so does a run without `ORB_THIRDPARTY_MODS_DIR` when the manifest lists a song with a
third-party mod (531 of them), so a green run has compared all 1268. Regenerate the manifest when
the collection changes.

**Positions that wrap.** The reader adds the deltas in u32 (`docs/song_load.md` section 10), so
huge deltas can wrap a track's positions past 2^32. The writers (`crates/rbs` `Track::build`,
`tools/rbs.py`) write each delta as `pos - prev` modulo 2^32 and refuse nothing: positions come
only from u32 deltas, so every backward step of a parsed track is a wrap (whatever its delta), and
every parsed track writes back byte for byte. The order is checked where tracks are built from a
model instead: orb-ui's `document::set_engine_tracks` (the engine's lists, a 1.x import included)
debug-asserts that its positions never go back ("events not sorted by position"), and
`tools/rbs.py`'s `validate` refuses an unsorted spec. No song of the collection wraps. Today
orb-ui's Save writes a song's own events through this writer; task-143 (open) makes it write the
engine's tracks instead, whose positions are below 32000 and sorted, so they never wrap.

**The run of 2026-10-30 (task-165).** The user added 29 songs and moved 7 (mostly into
`peff/`); `~/rb-songs/songs` now has 1271 song files, 1269 distinct. Three songs of the
2026-10-29 manifest (`Acid Werks.rbs`, `Phatmannen.rbs`, `block_out.rbs`, all in
`CaveysRebirthStuff/ReBirth 2.x Songs`) are gone from the folder and were dropped from the
manifest. 47 songs were rendered on the machine again with `thirdparty_songs.py --only`: the 30
new ones and the 17 whose mod changed (the pinned file `TechnoBox.rbm` / `degenerate
oxidized.rbm` gone, `DarkStar.rbm` replaced, or a mod now found for a name that had none,
`omega` -> `omega777.rbm`); the other entries kept their renders (a mod of the old manifest that
did not change keeps its hash, so the tool needs no change, but it does not notice a replaced
mod by itself: pass the songs of a changed mod to `--only`). Result: **1266 rendered by rbhost,
all equal `orb render`**, 3 skipped (the three below); 426 ReBirth 1.x and 840 RBS (461 of 4.2,
371 of 4.1, 8 of 4.0); 529 RBS play a mod, 42 mods in all; 311 play the built-in kit (310
`Standard ReBirth`, 1 `senctionbirth`, which no mod has).

**The run of 2026-10-31 (task-168).** The user's repairs closed task-148: `thefur~1.rbs` was
deleted, and `Top 25/005/Wax and Rain.RBS` and `Pack34/yawn.rbs` were repaired (both now RBS
4.2), and the mod name of `peff/pistolwh.rbs` was fixed (`senction birth`, found as `senction
birth.rbm`). `~/rb-songs/songs` now has 1270 song files, 1268 distinct. The 9 new mods are
named by no song in the folder that did not already resolve (the stale check below found no
other song whose resolution changed), so 3 songs were rendered on the machine again with
`thirdparty_songs.py --only`: the two repaired ones and `pistolwh.rbs`. Result: **1268 rendered
by rbhost, 0 skipped** (the three refused songs of section 11.4 are repaired or gone); 426
ReBirth 1.x and 842 RBS (464 of 4.2, 370 of 4.1, 8 of 4.0); 531 RBS play a mod, 43 mods in all
(`yawn.rbs` plays `AXIOMpeace.rbm`); 311 play the built-in kit, all `Standard ReBirth`.

**The run of 2026-10-29.** 1242 song files (a name containing `.rbs`), 1242 distinct:
- 426 ReBirth 1.x (built-in kit) and 813 RBS: 436 of 4.2, 369 of 4.1, 8 of 4.0. 510 RBS play a
  mod, 33 mods in all (each name matches one file). 303 play the built-in kit: 301 name `Standard ReBirth`, and 2
  name `omega`, which no INFO name matches (`Omega.rbm` is `Omega`).
- All songs are in song mode. Five pattern-mode songs (four 1.x) were resaved by the user during
  the run; their new versions are the ones pinned.
- **1239 rendered by rbhost, and all 1239 equal `orb render` byte for byte** (the WAV's
  SHA-256), the 4.0 and 4.1 songs, the 1.x songs and all 33 mods included.
- 3 skipped: the original refuses them (`docs/song_load.md` 2.1, `sub_49F3B4`), and so does orb:
  `Pack34/yawn.rbs` (a CAT of 18018 bytes in a file of 18020), `Top 25/005/Wax and Rain.RBS` (a
  BinHex 4.0 text) and `ChemicalPack/thefur~1.rbs` (no chunk header).
- One mismatch was found and fixed. `Mod Demo Songs/AXIOM peace.rbs` has a 5-byte delta in a
  TRAK. orb refused it, but the original reads deltas of any length (`docs/song_load.md` 10).
  `crates/rbs` and `tools/rbs.py` now read it so, and its render equals rbhost's.
- **The `omega` songs and orb's ASCII-case rule.** orb matches the INFO name ignoring ASCII case
  and never a file name (section 10, task-150). So `--mod-dir` makes orb load `Omega.rbm` (INFO
  name `Omega`) for the two `omega` songs, where the original, comparing exactly, would ask and,
  on Standard, play the built-in kit. The test renders such a song without the mod folder, as
  rbhost rendered it, and also checks with `orb info --json --mod-dir` that orb resolves it to
  the mod that differs in case only.
- `CaveysRebirthStuff/Battle of the Best/Top 25/006/SaturdayMorning.zip` (a song, a ReadMe and a
  sample) was out of scope by the user's decision. The user replaced it with its song, `Saturday
  morning.RBS` (1.x), during the run, and that song is in the manifest like the others.

## 12. Recrypted mods: one 2047-byte embedded key window (task-059, 2026-10-27)

The user decided on 2026-10-27 (superseding task-053's "nothing bundled" for this only) that orb
embeds a minimum of `RB20FUL.DAT` that lets a mod load without the CD, and that
`orb mod recrypt IN.rbm -o OUT.rbm [--rb20ful PATH]` rewrites a mod to need only that. Shown the
trade-off of 12.2 (123 bytes with 113663-chunk mods against 2 KB with compact ones), the user
chose, the same day, **one 2 KB window and compact mods**: orb embeds `RB20FUL.DAT[W .. W + 2047]`,
and a recrypted mod keeps its chunk count and size (plus one 1024-byte chunk). A mod that is not
recrypted is not loaded by orb at all (section 12.4); `orb mod recrypt` is the one place `RB20FUL.DAT`
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
they came from (`62071e99...2797`), checked against the DAT of `ORB_CD_ISO` by the test
`embedded_window_is_the_dats`. orb's loader (`KeyTable::embedded`, `Rbm::slots_covered`) takes a
mod whose page offsets are `W` and whose files need no more than the embedded bytes; any other mod
needs `RB20FUL.DAT`.

### 12.3 What the original accepted

On the test machine (a copy of the installation with only the four recrypted mods in its `Mods`,
`RB20FUL.DAT` from the CD as always), both forms were run the same way: the app's Mods scan raised
no box (`rb_export.py` expects the MIDI box only), the three songs opened their recrypted mods with
no Download / Standard dialog, the exports of RSI (PBE2), Fonda Workout (MSM2) and Metallicon City
(Metallicon+) each matched rbhost with the **original** mod at exactly one N among 200
(`fit.py --all`), `orb compare` with the recrypted mod and no `RB20FUL.DAT` at that N is EQUAL for
all three, and the panels display clean:

| Form | Chunks | RSI N (N - N_t) | Fonda Workout | Metallicon City |
|---|---|---|---|---|
| 123 bytes, fillers | 113663 / 98263 | 3584 (-503) | 3728 (-176) | 3648 (-444) |
| 2047 bytes (orb's) | 109 | 3600 (-33) | 3552 (+10) | 3568 (-6) |

With the original mods N - N_t was -28, -40 and -5 (section 8): the 2 KB form is back in that
range, and the 123-byte form's gap came with its 113663 chunks [G: the open mod's chunk walk
delays the export; not measured further].

All 172 third-party mods of section 11 and the four shipped ones recrypt, and every file and slot
sample of the result equals the original's (`crates/rbm/tests/rbm.rs`, `thirdparty_mods_recrypt`
and `real_mods_recrypt`); the corpus mod songs render byte-exact with recrypted mods loaded without
the DAT (`crates/orb-cli/tests/corpus_mods.rs`). `tools/rbm.py` decodes with each reader's key.

### 12.4 orb reads only recrypted mods (task-063, 2026-10-27)

At the user's request, once mods can be recrypted `RB20FUL.DAT` goes to `orb mod recrypt` alone:

- `orb render`, `orb compare` and `orb play` have no `--rb20ful` and do not read `ORB_RB20FUL`; only
  `orb mod recrypt` has them (it searches: the option, `ORB_RB20FUL`, an `RB20FUL.DAT` in the input
  mod's folder or its parent, next to the binary, the Windows install folders). `crates/rbm` keeps the
  general descrambler (`KeyTable::read`) for recrypt's own use.
- A mod that is not recrypted is found like any other by `--mod-dir` / `ORB_MOD_DIR` (its INFO is in
  the clear, it keeps its place in the folder order, so the duplicate-name rule of section 11.3
  applies to it as to any mod) and `--mod FILE` takes it; the error comes when its samples are loaded
  (only when a drum machine plays): `FILE: the song plays a drum machine with this mod, which is
  not recrypted. Run orb mod recrypt --rb20ful RB20FUL.DAT FILE -o OUT.rbm once and use OUT.rbm`
  (exit 1). The decision is "found, fails at load", not "skipped": a skipped mod would turn into
  "mod not found" and hide the fix. The future UI (m-3, task-047's mod chooser) lists such mods the
  same way, marked "needs orb mod recrypt".
- The env-gated tests take recrypted mods. Since task-121 the tests read three variables only
  (`ORB_CD_ISO`, `ORB_THIRDPARTY_MODS_DIR`, `ORB_THIRDPARTY_SONGS_DIR`): `corpus_mods.rs` and the
  other tests that need the shipped mods use the four mods that `orb-testkit` recrypts from the CD
  image (pinned SHA-256s, docs/test_machine.md 7); `orb compare` still takes `--mod-dir`
  (`orb mod recrypt --iso "$ORB_CD_ISO" -o DIR` makes the folder); `thirdparty.rs` reads
  `ORB_THIRDPARTY_MODS_DIR` (recrypted) against the committed manifest (section 11);
  `corpus_mods.rs`'s `recrypt_of_the_originals_matches_the_references` still runs the recrypt (the
  originals and the DAT from the image), requires its bytes to equal the recrypted mods and renders
  the 8 songs from them.

**Done on 2026-10-27.** The user's 172 third-party mods: `~/rb_stuff/Mods.orig` holds the originals,
`~/rb_stuff/Mods` the recrypted ones under the same names; all 172 recrypted (0 failures), and
`thirdparty.rs` `mods` found 16680 embedded files and 7285 slots equal to `rbm.py check-dir` of the
originals. The four shipped mods on the test machine: the originals are backed up in
`C:\re\rb\mods_orig`, `C:\ReBirth RB-338\Mods` holds the recrypted ones (hashes in
docs/test_machine.md section 7), and an app export of Fonda Workout from the installation equals
`orb compare` with the recrypted MSM2 at one N of 3400..3707 (3600).

### 12.5 `orb mod recrypt`: the key file's SHA-256, the file checks, bulk mode (task-064, 2026-10-28)

Until task-064 `orb mod recrypt` checked only the DAT's magic and length. A different or corrupt
`RB20FUL.DAT` with the right header decrypted to garbage, which was scrambled again against the
embedded window; the internal round trip passed because it used the same wrong key, and the mod
played noise. Now (`crates/rbm/src/dat.rs`, `crates/orb-cli/src/recrypt.rs`):

- **The key file is verified.** Before any mod is written, the SHA-256 of the whole file is
  compared with `embedded::DAT_SHA256` (`62071e99...2797`, 134,217,984 bytes: the 2001 reissue
  CD's file). A mismatch is an error naming the expected and the actual hash, and saying whether
  the 2047 bytes at `embedded::WINDOW` equal the embedded ones (they differ: another edition of
  the CD; they equal: the file differs elsewhere, modified or corrupt). **Only that one edition's
  `RB20FUL.DAT` is supported yet; another edition is not**, because no other is known and the
  embedded window comes from this one. The size and magic are checked first, with their own
  messages. The hash is part of a 0.8 s wall-clock run of one mod (release build, macOS, the
  128 MB file in the page cache); the file is found, hashed and opened once per run, when
  the first mod that is not recrypted needs it, and each mod then reads only its two key pages
  (2 x 2048 bytes). A run in which every mod is recrypted never reads the file.
- **The files are checked.** Each embedded file of the input, decrypted, must be well formed: a
  slot sample or any `.aif` / `.aiff` must parse with the engine's AIFF reader
  (`orb_engine::samples::Sample::from_aiff`, the DLL's reader rules of sections 7.4 of
  `initial_investigation.md` and 11.2 above for the stale SSND size), and a `.jpg` must
  start with `FF D8`; other files (the demo song) are not checked. A failure is an error naming the
  mod file and the embedded name, and nothing is written for that mod. These header checks find
  damaged files; they are **not** a wrong-key detector: the first 1024 bytes of every file are
  never scrambled and the AIFF and JPEG headers lie there (only a sample whose chunks follow the
  audio would fail with a wrong key). The SHA-256 is the key file's check, and the checks past
  byte 1024 (next item) find a mod scrambled with another key file. The result of the recrypt is
  checked the same way with the embedded bytes. Measured: the 172 third-party originals recrypt
  in one run with the real file (0 failed, byte-identical to the recrypted set of 12.4), and the
  172 recrypted ones and the four recrypted shipped mods (already recrypted) pass the checks too
  (no false positives).
- **The checks past byte 1024** (task-160, 2026-10-30; `rbm::dat::check_file`,
  `rbm::dat::validate_files`). The scrambling adds key bytes to every byte from 1024 on, so a mod
  scrambled with another edition's key file, or stored unscrambled (a key half of zeros, section
  4), passes every header check and recrypts into garbage that plays as noise.
  - *JPEG.* A `.jpg` longer than its clear part (1024 bytes of the payload minus the name and
    its NUL, so 1014 for `12016.jpg`; see *Rescue*) must end with `FF D9` followed only by zero bytes,
    or else hold only JPEG data past byte 1024: every `FF` there followed by `00`, `01`, `FF` or a
    marker `C0..FF`. The second clause exists for YellowBirth (in `~/rb-mods/mods`, from a
    collection of 2026-10-28): its 59 JPEGs were cut short by its packer (section 11.1) and lack
    `FF D9`, but are intact JPEG data. The set is 66 of 256 values (`FF` lies inside `C0..FF`),
    so in noise a `FF` is followed by a byte outside it 190 times in 256 (about 3 in 4). A noise
    JPEG of `size` bytes therefore passes with a probability of about exp(-0.0029 * (size - 1024)):
    0.80 at 1.1 KB, 0.23 at 1.5 KB, 0.06 at 2 KB, 0.003 at 3 KB, about 0 from 5 KB; any one
    failing JPEG refuses the mod, and the audio test backs it up. Measured over the 317 originals and 171 recrypted mods: of the JPEGs without an end
    marker, YellowBirth's 59 have no such byte (and no other good mod has one without an end
    marker); in the five bad mods 0 to 8 of the 32 to 59 JPEGs have none (short files with few
    bytes past 1024), so every bad mod is refused by its first JPEG with one.
  - *Audio.* The share of jumps over 16384 between neighbouring 16-bit words past byte 1024
    (first 100,000 words) of each sample file: 0 to 0.2 for ordinary samples, about 0.56 for
    noise. One file proves nothing, because real noise drums exist (Heavy Rhythm Machine's
    TR808SD5.aif reaches 0.696, Magica's TR808HC 0.644, Neurotek and Inner mod 0.524), so the
    mod is judged: at least 4 sample files and more than half above 0.3. Over the 312 good
    originals the share of files above 0.3 is at most 0.087 per mod (the minimum over a mod's
    files at most 0.0007; the 171 recrypted mods have at most 0.524 in a single file); in each of the five bad mods it is 1.000
    (every file 0.497 to 0.61, median 0.557 to 0.577). The margin is 0.087 against 1.0.
  - *The five* (the user's folder of 317 originals). Scrambled with a key table that is not the
    2001 CD's: `mellow.rbm` (keys 72963593 and 134110473), `House of
    love Juam1 .rbm` (3876875 and 90141265), `TurnMod.rbm` (10752749 and 50644384),
    `AloNaNbox 2.0.rbm` (118557743 and 114850228), `MaShMoD.rbm` (20373127 and 84469203). One or
    both halves are zeros (mellow, House of love: both; TurnMod: the even half; AloNaNbox: the odd
    half) or foreign bytes found nowhere in the 2001 `RB20FUL.DAT` (MaShMoD: both; TurnMod, the
    odd half; AloNaNbox, the even half). With only the checks above (task-160) `orb mod
    recrypt` over the 317 originals wrote 312 and refused exactly these five, each by the first
    JPEG (since task-161 the first two are recovered, see *Rescue*): "the JPEG 12015.jpg has no end
    marker FF D9 and its data is not JPEG data; the mod was probably scrambled
    with the key file of another edition of ReBirth (or not scrambled at all)". Bulk mode goes on
    with the other mods. `~/rb-stuff-peff/mods_recrypted` holds three of them already recrypted
    (mellow, TurnMod, AloNaNbox): recrypt refuses them too (it checks an already recrypted mod).
  - *Loading.* `Rbm::load_samples` (so `orb render`, `compare`, `play` and the UI) applies the
    audio test to the slot sample files past byte 1024, as recrypt does, and returns the same error (`RbmError::WrongKey`)
    instead of playing noise. **Decision: refuse, not warn**: a mod whose audio is noise is never
    what a user means to play, and the check only fires when most of at least 4 files are noise.
    It does not look at JPEGs (the engine uses none).
  - *Rescue* (task-161, 2026-10-30; `rbm::dat::check_and_rescue`, `rbm::recrypt_with`). A file
    stored unscrambled (a key half of zeros, mellow, House of love, half of TurnMod and of
    AloNaNbox) is recovered exactly, **per file**; recrypt takes it as stored only when the CD's
    key fails it and it passes as stored:
    - *JPEG:* the stored file starts with FF D8 and ends with FF D9 (trailing zeros allowed), and
      the file descrambled with the CD's key does not. The cut-short leniency of the checks above
      never applies to a rescue, because it also passes keyed noise that has few FF bytes. A bug
      of the first version of the rescue (review round 1): the checks took the bytes up to 1024 of
      the *file* as never scrambled, but the scrambling counts from the start of the *payload*,
      which holds the name and its NUL first, so a file's own clear part is `1024 - (name length +
      1)` (1014 for `12016.jpg`, `rbm::dat::clear_len`). A short JPEG of 1015 to 1024 bytes, or
      the few bytes of a longer one, was taken as "never scrambled" and passed as noise with the
      key, so 15 stored JPEGs (mellow: 12015, 12016, 12068, 12558 to 12562 and House of love:
      12016, 12019, 12506, 12509, 12550, 12567, 12571) were written descrambled with the CD's key,
      differing from the twin after file byte 1014. A guess that those halves are zero only in
      part was **not** the cause: recovering the table bytes with the twins as known plaintext
      gives stored == plaintext for all 104 files of mellow, so the half is zero over every window
      tested (up to 1924 entries); with `clear_len` all 15 are rescued and equal.
    - *AIFF:* the jump share (above) with the CD's key is above 0.45 (RESCUE_CD_NOISY) and as
      stored below 0.3 (RESCUE_STORED_CLEAN). Measured over the 317 originals (the 171 recrypted
      mods of `~/rb-mods/mods` are all recrypted, so they do not exercise the rescue; the evidence
      for good mods is the 312 originals, of which recrypt reports no rescue): 0 files in a good
      mod meet both; the noise drums that exceed 0.45 (Heavy Rhythm Machine 2 files, Inner mod,
      Magica, Neurotek 1 each) are 0.556 to 0.570 as stored. In the files stored unscrambled the
      share as stored is 0.000 to 0.069 (mellow 44 of 44 files, House of love 46 of 46, TurnMod
      21 of 44, AloNaNbox 23 of 46) against 0.49 to 0.61 with the CD's key. MaShMoD: 0 files
      (46 are 0.526 to 0.564 as stored: foreign key, noise either way). The margin is 0.069
      against 0.556 for the stored share, 0.45 against at most 0.30 needed.
    - *A file that cannot say* (noise as stored and with the key, a noise drum, or the demo
      song, which has no check) follows the key half: the two halves of the key table are chosen
      by the parity of the chunk key, and a file uses the entries `0 .. min(period, size -
      1024)` of its half (`key_need`). A file that cannot say is taken as stored only if the
      files of its parity found stored by their own evidence prove that many entries zero (the
      largest `key_need` among them), else it is left to the key. A noise drum alone in its half,
      or with a window past the proven entries, stays descrambled with the CD's key, which is
      the one case the rescue cannot see (the mod's audio check still sees a mod of mostly
      noise).
    - *The mod:* it is written only if every checked file passes one way or the other, then the
      audio of the chosen versions passes the per-mod check. Otherwise it is refused naming the
      JPEGs (or the audio) that failed. The summary says "N files were stored unscrambled (made
      with another edition's key file); recovered (first names)". Run over the 317 originals:
      314 written (the 312 and mellow and House of love, 77 and 106 files recovered: every file
      of the two) and exactly TurnMod (27 JPEGs), AloNaNbox (15) and MaShMoD (45) refused. The
      recovered files of mellow equal the files of its correctly scrambled twin `Mellow
      (SouthPark).rbm`: 104 of 104 identical. House of love Juam1 against its twin `House of love
      Juamø.rbm` (on disk `JuamÃ¸`): 104 of 106 identical; `Demo.rbs` (a valid RBS, "Template
      Mod") and `Splash.jpg` (ends with FF D9) differ, being that mod's own content. TurnMod's odd half, AloNaNbox's even half and all of MaShMoD stay unrecoverable
      (they need the other key file). Not done: a `--force-partial` keeping the built-in sample for a
      lost slot (a later decision).
    - *The original loads the recovered mods* (task-163, 2026-10-30) [C for the test machine's
      rbhost run of 2026-10-30, ReBirth.dll as installed]. Both mods were
      recrypted by `orb mod recrypt --rb20ful RB20FUL.DAT mellow.rbm "House of love Juam1 .rbm"
      -o DIR` (77 and 106 files recovered), each named by a song built with `tools/rbs.py build`
      (`{"glob":{"mod_name":"southpark Mod"}}`, `"Template Mod"`), and loaded by the original
      `ReBirth.dll` with `tools/rbhost/run.py --mod DIR/X.rbm --rb20ful RB20FUL.DAT
      --dump-samples --remote-dir task163 -o OUT SONG.rbs` (no Rebirth.exe was running; one
      permit; the machine folder was deleted). The DLL accepted both (message 55, "mod 412
      loaded") and dumped its sample table. Each dumped `slotNN.bin` (int16) equals channel 0 of
      the AIFF in the correctly scrambled twin (`rbm.py`'s `embedded_file` and `aiff_samples`),
      sample for sample: mellow 44 of 44 slots equal, House of love 46 of 46 equal; 0 slots
      differ. The count is of the 46 sample slots, not of files: the 104/106 comparison above
      includes `Demo.rbs` and `Splash.jpg`, which are outside the DLL's sample table. Mellow's
      twin has no file for slots 21 (`TR808MC.aif`) and 27 (`TR909bd3.aif`); those two were not
      compared (by the rule at the top of this document and in the CLI notes, a missing slot
      keeps the built-in sample; this run did not test that). "77 files recovered" against "104 of
      104 identical": 27 of mellow's 104 files have a payload of at most 1024 bytes, wholly in
      the clear (the 27 short JPEGs, 12017 ... 12571), so they need no rescue; 77 + 27 = 104
      (counted from the file with `rbm.py`).
- **Bulk.** `orb mod recrypt IN.rbm... DIR... -o OUTDIR [--force]` takes any number of mod files
  and folders (every `.rbm` directly in a folder) and writes each mod under its own file name into
  `OUTDIR`. The single-file form `orb mod recrypt IN.rbm -o OUT.rbm` stays: one existing input
  file and an `-o` that is not an existing folder and ends in `.rbm`. An `-o` ending in `.rbm`
  that is not an existing folder is a file name and never becomes a folder: a missing input or
  several inputs with it are a usage error (exit 2). `OUTDIR` is created only when the first mod
  is written, so a run in which nothing passes (a bad key file, missing inputs) leaves nothing
  behind. `OUTDIR` must not be the folder of an input (exit 2). An output that exists is a failure
  of that mod unless `--force`, **in the single-file form too** (before task-064 it was
  overwritten silently); a second input with the same file name fails. A mod that fails does
  not stop the others; the summary (`N recrypted, N already recrypted (copied), N failed`, then
  one line per failure) goes to stderr and the exit code is 1 if any failed. A mod that is
  **already recrypted** is checked with the embedded bytes and **copied unchanged**
  (recrypting it again gives the same bytes, so the copy is the cheaper form of the same
  thing).
- **Tests.** `crates/orb-cli/tests/recrypt.rs`: a stand-in key file (sparse, with the magic and the
  pages of the synthetic mods) and `Options::expected_sha256` for its hash; bulk over a folder
  and over a list equal recrypting each mod alone and hash the file once (`Report::dat_hashes`);
  a wrong-hash file with the right magic and length is rejected before anything is written (also
  through the binary); a damaged AIFF, a JPEG without `FF D8` and a sample whose header is found
  only with the right pages are caught; `--force` and the folder rules; the already-recrypted
  path over the recrypted shipped mods. Gated on the real file (`ORB_CD_ISO`): bulk over the
  shipped originals equals single and the pinned SHA-256s of the recrypted mods.
  Synthetic, with no input (task-121; `crates/rbm/tests/recrypt_python.rs`): mods built with a
  stand-in key table (ordinals past 255 where the DLL's byte key and the exe's u16 key differ,
  payloads of 8 to 3,000,000 bytes around the 1024-byte head, 199 files, duplicate names, foreign
  chunks, fillers and an earlier pad) are recrypted, and `crates/rbm` and `tools/rbm.py` (the
  independent decoder, given a key file that holds only the window) must read the same files from
  the source and from the result.

## 13. The standard mod: the built-in kit and panel as a mod (task-036, 2026-10-27)

`orb mod pack-standard --rebirth-dll ReBirth.dll [--rebirth-exe Rebirth.exe] -o FILE`
(`crates/rbm/src/standard.rs`) builds a mod that holds what the original has built in. By the
user's decision of 2026-10-27 (task-067) one build of it is committed as `assets/Standard.rbm`
(the one file of Propellerhead's in the repository; `assets/README.md`) and embedded in orb as its
built-in kit (13.4).

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
extracted from the CAB in `Install ReBirth RB-338.exe` with `bsdtar`) and the installed one
(SHA-256 `6a651e8164b8444a5f0639cb6a2fef2aa43c5500bd9c5288627c2030b047f876`, 1,594,880 bytes;
the installed ReBirth.dll is 2,472,960 bytes) have the same 58 panel bitmaps byte for byte; the
four splash and about screens 900, 901, 903 and 905 differ: the installed one's carry a Steinberg logo
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
  INFO  1280   name "Standard ReBirth", version BC 01 00 00 (1.0.0, as PBE2, MSM2, Orbit2),
               a description naming `rb mod pack-standard` (the CLI's first name, kept in the committed asset), no ftp URL,
               web http://www.propellerheads.se/
```

108 chunks, 106 EMBF (under the 200 of OB-14, every ordinal under 256). It is laid out in the clear
and passed through `rbm::recrypt` with an all-zero table (the identity), so it is scrambled against
the embedded window exactly as `orb mod recrypt` output: both key pages are `W`, orb loads it without
`RB20FUL.DAT`. The build is deterministic:

| Built from | Size | SHA-256 |
|---|---|---|
| the installed `ReBirth.dll` (`4c3a6441...8b12`) and the CD's Rebirth.exe (`e69fef41...7004`): **`assets/Standard.rbm`** | 2,572,792 | `6355b5f14fc2ebfe7ed9e6e42208f97817f96c5e30ed42fdca4205d3cdab5fbe` |
| the installed `ReBirth.dll` and `Rebirth.exe` | 2,574,060 | `c8752965...9324` |

The two differ only in `splash.jpg` (13.1). The gated test
`crates/orb-cli/tests/standard_mod.rs::the_command_reproduces_the_committed_mod`
(`ORB_CD_ISO`: the CD's exe and the DLL come out of the image) rebuilds the asset byte for
byte.

The menu name is **"Standard ReBirth"** (task-067): the name songs give the built-in kit (the GLOB
mod name of a song without a mod; the original's `sub_4166E5` pushes the string at 0x4BB2D0
and `this + 0x63B` and calls `_strcmp` at 0x4A76E0, 0x4166EC..0x4166FA, returning 1 when it
gives 0 (`test eax, eax` at 0x416702) [C]; 0x4A76E0 is the CRT's _strcmp (equal strings give
0 at 0x4A7720; the first difference gives -1 or 1 at 0x4A7724)). The committed mod is orb's
built-in kit, so a song made with the built-in kit and the mod name the same kit, and orb never
looks the name up in a mod folder. The mod is for orb only and was not tried in the original app
in this form (where a mod of that name would stand beside the built-in entry of the Mods menu).
task-036 built it as "ReBirth Standard Kit", with a description naming `rb standard-mod`
(2,574,060 bytes from the installed files, SHA-256 `2d732978...f3e5`); 13.3 is that form's app check.

### 13.3 What the original does with it (test machine, 2026-10-27)

This was the task-036 form, named "ReBirth Standard Kit" (13.2).
A copy of the installation in `C:\re\rb\t036` with only the standard mod in its `Mods`: the app
started with no extra box, the Mods menu listed `Standard ReBirth` and `ReBirth Standard Kit`, and a
song whose GLOB names "ReBirth Standard Kit" opened with no Download / Standard dialog, the mod
checked in the menu and the panel drawn clean. `rb_export.py --app-dir` exported `808_snare` and
`909_snare` (diag) with GLOB "ReBirth Standard Kit" and, the same session, unchanged ("Standard
ReBirth"): `orb compare` (the built-in kit from the standard mod) is EQUAL for each of the four WAVs
at one N of 2000..12000 (2704 and 2704 with the mod, 2768 and 2784 with the built-in kit; the N is
the app's start state, docs/export_state.md section 13). So each export with the standard mod
equals orb's render with the built-in kit at its own fitted N; the two app WAVs of one song (mod and
built-in) differ from each other, as two exports of one song do at different start states. What
was shown is that the samples the app plays from the standard mod give the built-in kit's output.

### 13.4 orb's built-in kit is the embedded mod (task-067)

`crates/rbm/src/standard.rs` embeds `assets/Standard.rbm` (`include_bytes!`, `EMBEDDED`):
`builtin_samples()` loads its 46 samples once (`Rbm::load_all_samples` with the embedded key
window), `builtin_file(name)` gives any of its files (the UI's splash). `orb render`, `orb compare`,
`orb play` and OpenRB-338 (the panel, the 808's mod window `App::mod_view`, the export) use it and
read no file for the kit: `--standard-mod` and `RB_STANDARD_MOD` (task-036's lookup: the option, the
variable, `Standard.rbm` next to the binary, a "ReBirth Standard Kit" mod in the mod folder) are
gone (a decision of task-067: the kit is part of orb, like its tables; another kit is a song mod).
`orb mod pack-standard` needs `-o` (there is no place it is looked for any more) and is the one
command that reads ReBirth.dll and Rebirth.exe. A song whose GLOB names "Standard ReBirth" plays
the embedded kit; a third-party mod's missing slots and `--standard-kit` take its samples. The
samples are ReBirth.dll's byte for byte, so the corpus is unchanged: 164 of 164 EQUAL with no kit
option, 175 of 175 with the four recrypted shipped mods (`--mod-dir`). The tests (orb-cli,
orb-engine's corpus tests, orb-player, OpenRB-338) use the embedded kit and need no file of the original;
the ones that check the DLL itself or build the mod take the DLL and the exe from the CD image
(`ORB_CD_ISO`, task-121) and skip without it.

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
  (0x41A2B6..0x41A305) calls `sub_42D80D(window, mod)`, stores `dword_4DA788` and marks the
  dialog for deletion (`sub_47350E`, section 14.5).
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
  document (`sub_42D7F7(0)`), marks it for deletion (`sub_47350E`), deletes it (`sub_473401`,
  0x42D892; section 14.5) and builds a new song window for the
  document (`sub_42DA72(doc, 0, rect)`, at the old window's place), which draws the panel with the
  new mod's bitmaps (section 13.1). The new window's activation (`sub_40D163` at 0x42E5AE) sends
  top-level 53 only when the document is not already the current one (0x40D178); with this one
  song window open it is, so no msg 53 is sent. With other song windows open, the Mods menu sends
  msg 53 for another song and back (inferred, section 14.5). The same mod again does nothing.
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

The "no msg 53" holds for one open song. With more than one song window open, the Mods menu (not
the Mod Selector's OK) also sends top-level 53 for another song and then back for this one
(inferred, section 14.5).

### 14.3 orb

`Engine::set_samples` (crates/orb-engine/src/frame/mod.rs) is `sub_10011D84`: the 808, then the
909, each `ChannelDevice::set_samples` = slot 6, then the table. The real-time player sends it as
`Command::Mod` (`Player::set_mod`, crates/orb-player), to the newest song as every command. The
audio callback frees nothing: the command's box (`ModSwap`) carries a fixed-capacity
`samples::Retired` that takes the old tables and the samples the silenced voices still held, and
goes back to the UI thread through the garbage queue. The original's voices keep their old sample
pointer; orb's voices let go of it, which changes no output (an inactive voice reads nothing, and
every 808 and 909 trigger passes a sample). Tests: the engine's
`set_samples_silences_then_replaces_the_table` (808, 909); the player's `mod_change.rs` (a mod
set on a stopped new song equals building the song with it; on a playing song the tick runs on as
without the change, the drums go silent at once with a silent kit and sound again with the
built-in one); `no_alloc.rs` (the change twice while playing, the first kit freed on the UI side
before the voices retrigger). The startup sound after the change (msg 58, the preview player)
is `Player::startup_sound` / `Command::Preview` (task-074, docs/export_state.md section 8),
which OpenRB-338 sends after `Player::set_mod` when its preference is on, and after a song's open
and at the start (docs/ui_spec.md section 18.1, task-087). The check against the DLL
itself is section 14.4.

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

orb-player's render of the same sessions (`Player::set_mod` with the recrypted mod's samples
before the same call, `crates/orb-player/tests/live_messages.rs`) has the same hashes: byte for
byte equal, on the first run, with no change to orb. The same sessions without the changes render
otherwise in orb (the test checks it), so the changes are audible: the mod swap while playing
(slot 6 on the 808 and the 909, then the new table, the transport untouched), while stopped, to
and from the built-in kit and between two file mods all do in orb what they do in the DLL. Not
covered: the startup sound (msg 58, task-074) and more than one open song (14.5).

OpenRB-338's Mods menu (task-047, docs/ui_spec.md section 18) is this change: `Link::set_mod`
forwards the panel's pending events and sends `Player::set_mod`; the song is marked modified, and
a save writes the mod's INFO name (GLOB) and URLs. The mod's panel bitmaps are not applied (orb
draws its own art).

### 14.5 Several song windows open: the msg 53 round trip (task-075, 2026-10-27)

**The Mods menu sends top-level 53 for another open song, then 53 back for the changed song
[G: inferred from the two facts below]. The Mod Selector's OK sends neither [G: inferred, and the
dialog being active is not traced].** The marks are split:
- the exe code is [C], read in the disassembly (and the addresses re-read with llvm-objdump on
  the installed `Rebirth.exe`);
- the order of Windows' MDI messages is [C for the test machine's Windows], measured with a probe
  program that mimics the exe's calls, not with Rebirth.exe;
- the sequence in Rebirth.exe composed from the two is inferred, not observed.

- **The destroy is synchronous and goes through WM_MDIDESTROY [C].** `sub_47350E` only marks the
  window (`mov byte ptr [eax+18h],1`); `sub_42D80D` then calls `sub_473401` (0x42D892), which walks
  the window list `dword_4DFA60` and deletes each marked window through its vtable slot 0 (`push 1;
  call [edx]`, 0x4734D8..0x4734E2). The song window (`CReBirthWindow`, vtable 0x4BBFB4) is deleted
  by `sub_42F2D0` → `sub_42E6CA` → `sub_498085` (`CDocumentWindow`) → `sub_49629E` (`CABCWindow`) →
  `sub_49617E`, which first takes the window off the list (`sub_472A09`, 0x49620F) and then, for
  window type 1 (`this+0x3C`, the MDI child; the type is the first argument of the `CABCWindow`
  ctor `sub_495B0B`, stored at 0x495BC9), sends message 545 = WM_MDIDESTROY `{hwnd, 0}` to the MDI
  client (`sub_481A59`; the call through the user32 table `dword_4DFB24 + 4`, 0x49626B). All of it
  runs before `sub_42DA72` builds the new window (0x42D8BE).
- **WM_NCACTIVATE TRUE is the activation [C].** The window procedure `sub_49395B` sends message 0x86
  to `sub_494DF1` (0x493E2C): with wParam TRUE it finds the window object by hwnd (`sub_473376`)
  and calls its vtable slot 14 (`call [edx+38h]`, 0x494E20; FALSE calls slot 15, 0x494E2D).
  Slot 14 of the song window (0x4BBFEC) is `sub_42D7BE`, which calls `sub_40D163(document)`
  (0x42D7DD): top-level 53 `{the document's slot}` when the document is not app `+0x2544`
  (0x40D172..0x40D1B1). WM_MDIACTIVATE (0x222, `sub_494DCF`) only goes to `DefMDIChildProc`.
- **What Windows sends on WM_MDIDESTROY of the active child [C for the test machine's
  Windows, from a probe, not from Rebirth.exe].** The probe was a 32-bit program built with
  llvm-mingw: an MDI frame, children A and B, B active, then WM_MDIDESTROY of B and a new child
  B2, logging the children's messages. It ran from `C:\re\rb\t075` and was not kept.
  - With the frame being the active window, the client activates A before B is destroyed, and A
    gets `WM_NCACTIVATE` 1. The log order: B NCACTIVATE 0, B MDIACTIVATE, A NCACTIVATE 1, A
    SETFOCUS, A MDIACTIVATE, B WM_DESTROY. B2's creation then gives A NCACTIVATE 0 and B2
    NCACTIVATE 1.
  - With an owned popup being the active window (the frame disabled or not), A becomes the
    MDI-active child but gets only WM_MDIACTIVATE, no WM_NCACTIVATE 1. B2 gets its NCACTIVATE 1
    only when the frame is active again.
- **So, for the Mods menu [G: inferred from the above]** (command `5000 + i` in the frame's
  `sub_40C163`; the frame is the active window): the next MDI child in Z order (another song
  window) gets NCACTIVATE 1 during the destroy → `sub_42D7BE` → msg 53 for its song, then the new
  window's constructor sends msg 53 for the changed song at 0x42E5AE (app `+0x2544` is now the
  other document). The new window's own NCACTIVATE 1 finds its document current and sends
  nothing. The order is msg 19 (in `sub_426FB3`, before the destroy), 53 {other}, 53 {this}.
- **For the Mod Selector's OK [G: inferred]** (`sub_41A2B6..` calling `sub_42D80D` while the
  dialog is up): the dialog is a `CModalDialog` (`sub_49836A`, `sub_495B0B(2, ...)` at
  0x49838F: type 2, a top-level window with `DefWindowProc`) run by the modal loop `sub_498496`,
  and it is the active window, so the other song window gets no NCACTIVATE 1 and no msg 53 is
  sent. When the dialog closes the
  frame is active again and the new window (already the current document) gets NCACTIVATE 1: no
  msg 53. [G: that the dialog is the active window when OK runs is inferred (a click on OK
  activates it), not traced.]
- **The effect in the DLL** (docs/export_state.md section 3, `sub_1001BDA4`): when the changed
  song plays, 53 {other} only makes the other song pending and 53 {this} makes this one pending
  again, so nothing changes. When it is stopped, 53 {other} runs slot 6 on this song's devices and
  makes the other current, and 53 {this} runs slot 6 on the other song's devices and makes this one
  current again: both songs' 303s get the note-off release, their drum voices are silenced, and the
  shared delay line is cleared. [G] Between the two messages an audio callback may render the
  other song (not observed). This is docs/original_bugs.md Q-4.
- **orb.** orb opens one song, so there is no other song to activate and nothing is modelled.

## 15. `orb mod`: unpack and pack (task-097, 2026-10-28)

The `orb mod` group holds every command that works on a mod file: `recrypt` (section 12, the one
command that reads `RB20FUL.DAT`), `pack-standard` (section 13, `orb mod pack-standard`), `unpack`
and `pack`. The code is `crates/rbm/src/pack.rs` (the CLI, `crates/orb-cli/src/modcmd.rs`, only
reads and writes the files).

### 15.1 `orb mod unpack MOD.rbm -o DIR`

Takes a recrypted mod (one that loads with `KeyTable::embedded`, i.e. orb's own; any other fails
with the `orb mod recrypt` hint and `RB20FUL.DAT` is never read). It writes every embedded file
decrypted with the key of its reader (12.1: the DLL's byte key for the first file of a slot name,
the exe's u16 key for the others) under its own name, skips the key pad `rb-recrypt.pad`, and
writes `DIR/mod.toml`. `DIR` must be new or empty unless `--force` is given. The mod must be
HEAD, EMBF chunks, INFO (what every known mod is, recrypted); another chunk is an error.

**Naming rule** (`rbm::pack::safe_path`, the one check for what unpack writes and for every path
pack reads from `mod.toml`). A file name is used as it is only when it is a plain name on every
system: not empty, not `.` or `..`, at most 200 bytes, none of `/ \ : * ? " < > |`, no control
character (NUL included), no trailing dot or space, not `mod.toml` in any case, and not a
Windows device name (`CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`, superscript digits
too, in any case, with or without an extension, and with spaces or dots before the extension:
`CON.aif`, `nul .txt`). Any other name, and any name whose lower case (`char::to_lowercase`, the
fold `rbm::pack::fold` both unpack and pack use) was taken by an earlier file, gets the file name
`NNN_name`: NNN its index, the unsafe characters turned into `_`, cut to 100 characters; if that
is taken too, or is unsafe, the next index (or `file` as the name) is tried until the path is safe
and unused. The file then has a `{ name, path }` entry in `files`. A name with a NUL cannot exist
in a mod (the NUL ends the name) and `pack` refuses one.

**Writing.** Each file is created with `create_new`: a clash the file system sees and the fold
does not (macOS normalizes Unicode, for one) is an error, never a silent overwrite of one file
of the mod by another. `--force` allows a folder that is not empty and deletes nothing: a file
that was in the folder before the run and has the exact name of one of the mod's files is
removed first and then created; every other name must be new.

### 15.2 `mod.toml`

```toml
name = "Pitch Black Edition 2.0"      # INFO, the Mods menu name (required, at most 63 bytes)
version = "1.0.0"                     # INFO; absent = zeros, "the app's version" (section 2)
description = """..."""               # INFO text (at most 599 bytes; CR LF lines)
ftp = "ftp://..."                     # at most 199 bytes
web = "http://..."                    # at most 199 bytes
head = "5b35b3..."                    # hex, only when HEAD is not the standard 256 bytes
info_raw = "..."                      # hex, only when the fields above cannot reproduce INFO
files = [                             # the EMBF order; the first of a name is the one looked up
    "12017.JPG",
    { name = "x", path = "001_x" },
]
```

Strings are Latin-1 (the original's), each field NUL-terminated in its INFO space (section 2).
`info_raw`, when present, is the whole 1280 bytes of INFO and wins over the fields (a mod with
bytes between or after its fields); unpack writes it only then. None of the four shipped mods
(recrypted) has one or a non-standard HEAD.

### 15.3 `orb mod pack DIR -o MOD.rbm`

Lays the CAT out in the clear (HEAD, the files in `files` order, INFO), then passes it through
`rbm::recrypt` with an all-zero table (the identity), exactly as the standard mod is built
(`standard.rs` uses the same `pack_parts`): the key pad is added after HEAD, every file is
scrambled against the embedded window under its ordinal, and the result is parsed and checked to
load without `RB20FUL.DAT` (`slots_covered`, `all_covered`). Errors (exit 1):
- a listed file that cannot be read, a path that fails the naming rule of 15.1, or two paths
  that are equal after the fold (case);
- **a file in DIR that `files` does not list** (a decision: an error and not a skipped warning,
  so a file is never left out unnoticed; `mod.toml` itself is the one exception). Names are
  compared exactly, so `a.bin` next to a listed `A.bin` on a case-sensitive file system is
  reported;
- more than 199 files (the pad is the 200th EMBF of OB-14), or a file named `rb-recrypt.pad`;
- an INFO field over its length, an empty name, a character over Latin-1, a bad `version`.

### 15.4 The round trip

`pack(unpack(M)) == M` byte for byte for every recrypted mod: the order is the file order, the
ordinals and so the keys are the same, the pad is rebuilt from the same files, and INFO and HEAD
are either rebuilt from the fields or carried raw. Checked (`crates/orb-cli/tests/mod_pack.rs`) on
the standard mod (`assets/Standard.rbm`) and on the four shipped mods, recrypted, in
`ORB_MOD_DIR`. To edit a mod: unpack, change a file or a field, pack. A mod made so loads in orb
and, like a recrypted one, in the original (section 12.3). Since task-067 the standard mod's INFO
description names `rb mod pack-standard` (13.2; the CLI's first name, kept in the committed asset).

## 16. The CD image: `--iso` (task-065, 2026-10-28)

`orb mod recrypt --iso CD.iso` and `orb mod pack-standard --iso CD.iso` read what they need from the
ReBirth CD as an ISO 9660 image, in place: nothing is extracted to disk and no Windows code runs.
The code is `crates/orb-cli/src/cd.rs`. The image is the user's (the 2001 reissue, 570,923,008
bytes); it and anything read from it are never committed.

### 16.1 What is on the CD [C, read from the user's image]

- **The image.** 2048-byte sectors, a primary volume descriptor and a Joliet one (UCS-2BE long
  names; the primary names are 8.3 with `;1`). orb's own reader (about 150 lines) takes the Joliet
  tree when there is one, else the primary one; names are compared without case; a multi-extent
  file,
  a block size other than 2048 or a raw 2352-byte image is refused. The root holds `Autorun.inf`,
  `Install ModPacker.exe`, `Install ReBirth RB-338.exe` (20,151,808), `RB20FUL.DAT` (134,217,984),
  `RB20UPD.DAT`, `rebirth.ico`, `ReadMe.wri` and the folders `Acrobat Reader 5.0`, `DirectX`, `HLD`,
  `Sounds Good promo`.
- **The installer** is a Wise for Windows package: three CAB archives are appended to the exe, at
  file offsets 0x12BE4 (13 files, the Windows Installer redistributable, 1,422,335 bytes),
  0x17E4E8 (the same 13 files again) and **0x30266C** (16,773,032 bytes, LZX window 21, one
  folder, **32 files**).
  `Install ModPacker.exe` has its own CABs (unpacked mod parts, not used). (The exe also holds
  `MSCF` bytes at 0x893E and 0x8D70 and their copies: not cabinets, so the scan checks
  reserved1 = 0,
  version 1.3, a file count and that `cbCabinet` fits the file.)
- **The 32 files of the third CAB:** `Rebirth.exe`, `ReBirth.dll`, `ReWire.dll`, `MSVCRT.DLL`, help
  and manuals, the songs (`ReBirth_1.0_Default_Song.rbs`, `Silent_Default_Song.rbs`, the demo
  songs) and **the four shipped mods `Metallicon.rbm`, `MSM2.rbm`, `Orbit2.rbm`, `PBE2.rbm`**, which
  are the originals of the pinned SHA-256s of docs/test_machine.md 7 (the test compares them). The mods are not plain
  files on the CD.

### 16.2 The commands

- **`orb mod recrypt --iso CD.iso [IN... ] -o OUT`**: `RB20FUL.DAT` is read through a window of the
  image (the SHA-256 of task-064 applies, 0.9 s for the four mods); `--iso` and `--rb20ful` exclude
  each other. With **no input mod** the four shipped mods of the third CAB are the inputs and `-o`
  must be a folder (a name ending in `.rbm` is a usage error). With input mods only the key file
  comes from the image. The result is byte for byte the `--rb20ful` form over the original mods
  (and the `recrypted-mods` set of 12.4).
- **`orb mod pack-standard --iso CD.iso -o FILE`**: `ReBirth.dll` and `Rebirth.exe` are unpacked from
  the third CAB in memory (0.5 s) and **checked by SHA-256** (`4c3a6441...8b12`, `e69fef41...7004`);
  another hash is an error that says it is another edition of the CD. The result is
  `assets/Standard.rbm` byte for byte (`6355b5f1...5fbe`). `--iso` excludes `--rebirth-dll` and
  `--rebirth-exe`.
- **Unknown images** are reported: not an ISO 9660 image (no `CD001` at sector 16, too short, a
  block size); an ISO without `RB20FUL.DAT` or the installer at its root ("not the ReBirth 2.01
  CD", with the names the root holds); a CAB set without the file (another edition of the CD?);
  a hash that
  differs (the `RB20FUL.DAT` message of 12.5, the two hashes above). Only the 2001 reissue is
  known; the 1998 original CD has not been seen, so whether its layout differs is **not confirmed**.
- **CAB reader.** `cab` 0.6.0 (pure Rust, MIT, 2024-04-05; its `unsafe` is in test code only; the
  LZX decoder is `lzxd` 0.2.7, 2026-06-25, MIT or Apache-2.0), pinned exactly (`cab = "=0.6.0"`).
  It was chosen over writing an LZX decoder: it reads the CAB with its header reserve and the
  LZX folder of the real installer: the files it gives have the SHA-256 that `bsdtar` and Windows
  `expand` gave (ReBirth.dll, Rebirth.exe) and the mods are the originals.
  Its other new dependencies (`time`, `deranged`, `num-conv`, `powerfmt`, `time-core`) are pinned
  by `Cargo.lock`, all published more than two weeks before this task.
- **Names from the image** that become output files (the CD's mods) must pass
  `rbm::pack::safe_path` (no separator, `..`, drive, device name); any other name is an error and
  nothing is written. The `cab` and `lzxd` crates are not hardened against malformed CAB bytes (a
  panic is possible, not memory unsafety); the image is the user's own.
- **Tests** (`crates/orb-cli/tests/cd_image.rs`): always on, a synthetic Joliet and a primary-only
  ISO and an MSZIP CAB (`cab` cannot write LZX) in memory: names, paths, windows, the decoy
  `MSCF`, unsafe mod names, a truncated installer, the errors, the option conflicts; gated on
  the real image (`ORB_CD_ISO`): the DAT's hash, the two
  files' hashes (the DLL's is the hash of the installed `ReBirth.dll`), the shipped mods equal to their
  pinned SHA-256s, `pack-standard --iso` equal to `assets/Standard.rbm` (and to the file form),
  `recrypt --iso` equal to the `--rb20ful` form and to the pinned SHA-256s of the recrypted mods.
  `standard_mod.rs`'s reproduction test takes the exe and the DLL from `ORB_CD_ISO`. The LZX path is
  covered only by the gated tests.

# Defects of the original (ReBirth 2.01)

rb reproduces the original's behavior, bugs included. This file lists every known defect of
`Rebirth.exe` (addresses `0x4xxxxx`) and `ReBirth.dll` (`0x10xxxxxx`) in one place. Each entry has
its home document, where the mechanism is written in full; **a newly found defect of the original
goes into this file as well as into its home document**, and the home document links here.

Fields: **What**, **Where** (addresses), **Trigger** (the song data or action that reaches it),
**Effect**, **Corpus** (the songs in `tests/songs` that reach it), **rb** (the model: file and
function, and any deliberate deviation), and a mark: [C] confirmed in the disassembly (every
address here was re-read with `llvm-objdump` on `original/`), [G] guessed or inferred.

Entries are numbered OB-n. "Quirks" at the end are behaviors that are odd but not defects, or whose
status is a judgment call.

## 1. Song loading (Rebirth.exe)

### OB-1: the 909 loader copies RBS byte 13 into slot 16 [C]

- **What.** The pattern-mode 909 parse copies LowTomDecay (RBS byte 13) into slot 16 (MidTomDecay)
  and drops RBS byte 16.
- **Where.** 0x433558: `mov dl,[ecx+0Dh]` then `mov [eax+10h],dl`.
- **Trigger.** Any RBS 909 chunk read into the pattern-mode block; the TRAK controller 0x10 is not
  affected.
- **Effect.** The pattern-mode mid tom decay equals the low tom decay, whatever the file says.
- **Corpus.** Every 909 song loads through it; it shows only where the two bytes differ [G: not
  checked per song].
- **rb.** `crates/rb-engine/src/load.rs` (comment "0x433558"), the same copy, unit-tested.
- **Home.** `docs/song_load.md` section 4.8 ("Quirk").

### OB-2: the old (4.0/4.1) 808 track tunes the tom value before the range check [C]

- **What.** For a pre-4.2 file the toms' tune `v/2 + 32` (`sub_432E1B`) is applied before the
  value is range-checked, so the check sees the tuned byte.
- **Where.** `sub_432E1B` called at 0x437139 (controllers 10, 13, 16); the check `sub_42FD00` is
  called at 0x4371A3 with the tuned byte.
- **Trigger.** A 4.0/4.1 file with a tom tune event of 190 (becomes 127, accepted) or 192 (becomes
  128, dropped).
- **Effect.** A value outside the documented range is accepted; the accepted range is shifted.
- **Corpus.** None known (the corpus songs are written as RBS 4.2) [G]. Unit test only.
- **rb.** `load.rs`, `old_tom_tune` applied before the range check.
- **Home.** `docs/song_load.md` section 5 ("The 808 track before 4.2").

### OB-3: the exe and the DLL disagree on the last tick of a track event [C for both limits, G for the effect]

- **What.** The exe accepts event ticks that are a multiple of 24 and below 768000, so up to
  767976. The DLL drops an event whose (quantized) tick is above 767231, and the sequencer ends
  the song at `pos > 767231`.
- **Where.** Exe `sub_42FD60` (`cmp [ebp+8], 0BB800h` at 0x42FD69); DLL 0x1001477B (`cmp eax,
  0BB4FFh`, `jg` drops).
- **Trigger.** A TRAK event in the last bar of bar 1000 (ticks 767232..767976).
- **Effect.** The exe counts the event as accepted and sends it; it is silently dropped in the DLL.
- **Corpus.** None (the largest corpus tick is 417792).
- **rb.** `seq.rs` (`SONG_END`) and the track insert drop events above 767231 like the DLL [G for
  the sent-and-dropped path: inferred from the two limits, not run].
- **Home.** `docs/song_load.md` sections 5 and 6,
  `docs/initial_investigation.md` section 5.

### OB-4: an event exactly at the loop end never fires in a loop [G: from task-022's trace of the wrap and chase, not re-read here]

- **What.** The loop-end wrap applies the events up to `loopEnd - 1`, and the chase starts the next
  pass at `loopStart`, so an event whose tick equals the loop end is applied by neither.
- **Where.** The clock `sub_1001EF20` (wrap), chase `sub_100151F0`.
- **Trigger.** A track event at the tick of the loop end (for example a pattern change at the first
  tick after the last bar).
- **Effect.** The event is lost in a looping song and in the export.
- **Corpus.** None known.
- **rb.** `crates/rb-engine/src/seq.rs`, the same wrap and chase.
- **Home.** `docs/initial_investigation.md` section 5.

### OB-15: a 1.x control change with no mapping indexes the track buffers with -1 [C: the index; G: the effect]

- **What.** The 1.x import maps a control change to an engine track with `sub_43067F`, whose
  switch returns -1 (0xFF) for an unknown (track, cc); the result is used as the index of the
  nine-pointer buffer array without a check.
- **Where.** `sub_430BA0`: `sub_43067F` at 0x430F5A, then 0x430FE3..0x430FEF `mov ecx,
  [ebp+var_A0]; and ecx, 0FFh; mov ecx, [ebp+ecx*4+var_3C]` and `call sub_4300E0` at 0x430FF3.
- **Trigger.** A 1.x song with a control change on MIDI track 2..5 outside the table of
  `docs/song_load.md` 12.2 (for example cc 23 on a 303 track, any cc on a track 6 or later).
- **Effect.** `sub_4300E0` runs on whatever dword lies 0x3FC bytes past the array (in the
  caller's frame): a write through a stack value or a crash [G: not run].
- **Corpus.** None: no song of the user's 129 1.x songs has such a controller.
- **rb.** `crates/rb-engine/src/load/rb1x.rs` (`load_1x`) refuses the song with
  `LoadError::Rb1x`; `tools/rb1x.py` raises.
- **Home.** `docs/song_load.md` section 12.2.

### OB-16: the 1.x song sysex is decoded past its end, and a foreign sysex is skipped twice [C: the code; G: the effect]

- **What.** `sub_4304C5` reads the sysex data into a buffer of its own length + 1, then decodes
  a fixed 0x4000-byte block from it, reading 18728 bytes whatever the length. A sysex that fails
  the `F0 00 00 66 ..` check is skipped by its length (`sub_4A0D77`) although its bytes were
  already read.
- **Where.** `sub_4304C5`: the buffer `operator new([arg_4+10h] + 1)` at 0x430506, the read
  `sub_4A0F79` at 0x430522, the decode `sub_4665CD` at 0x4305C4 (size computed at 0x43059C, 0x4000
  when the copy protection is intact); the second skip in the else branch (0x4305FF on).
- **Trigger.** A 1.x song whose song sysex is shorter than 18728 data bytes, or which holds
  another sysex before or after it.
- **Effect.** The decode reads heap bytes past the buffer into the block (garbage settings or a
  crash) [G]; the double skip moves the reader into the middle of the following events [G].
- **Corpus.** None: each of the 129 songs has one sysex of 18729 data bytes.
- **rb.** `rbs::rb1x::Block::decode` reads missing bytes as zero (a deviation where the original
  is undefined); a sysex that fails the check, or a second song sysex, is an error in `load_1x`.
- **Home.** `docs/song_load.md` section 12.3.

## 2. Devices

### OB-5: the PCF's wave byte indexes the pattern data with no bounds check [C]

- **What.** RBS42.txt gives wave 0..0x37 (55 patterns) but the exe's track load accepts 0..127 and
  the DLL checks nothing: waves 55..127 read the bytes after the 55 patterns (the steps end at
  0x10033AD0; the `CPCFInsert` vtable and base-relocated pointer words follow, up to 0x10034400).
- **Where.** The PCF block 0x100166D0; the pattern tables at 0x10033310..0x10034400 (resolution,
  length, steps at `byte_100333F0 + w*32`).
- **Trigger.** A PCF wave 'pattern' of 55..127, from a mixer/PCF event or a song's pattern-mode
  byte.
- **Effect.** Resolution, length and steps are other data. 17 waves (55, 111, 113..127) read length
  0 and fault (OB-6). Wave 64 reads resolution 0 and length 16 with all 16 steps 0: no trigger ever,
  the filter follows the base alone. Wave 112 reads length 120, so its steps reach 88 bytes past its
  own 32. 57 of the waves 55..127 have a relocated pointer word among their steps, so the output
  depends on the DLL's load address (`ImageBase`); the corpus assumes the preferred base.
- **Corpus.** `pcf_lp_wave64`, `pcf_bp_wave64` (wave 64, no trigger); `pcf_lp_wave127`,
  `pcf_bp_wave127` (OB-6).
- **rb.** `crates/rb-engine/src/pcf.rs` reads `PCF_DATA`, the raw bytes 0x10033310..0x10034400
  (generated by `tools/gen_static_tables.py`; decided by the user 2026-10-27, task-027: the file's
  bytes at the preferred base). Deviation: a wave above 127 is clamped to 127 (`pcf.rs:91`), since
  the original would read past 0x10034400 into BSS.
- **Home.** `docs/initial_investigation.md` section 8.2 ("Past the patterns").

### OB-6: the PCF divides by a pattern length of 0, and the render call is swallowed [C]

- **What.** The step is `q % len` by `idiv`; with length 0 it raises a divide error.
  `SafeSoundCallback` wraps the render in `try`/`catch (...)`, which catches it, and its
  continuation lies past the int16 conversion and the zero fill, so the call returns with the
  caller's buffer unwritten and the song advanced as far as the block got. The skipped
  `ReleaseMutex` calls leave the Win32 mutexes owned.
- **Where.** `idiv ecx` at 0x10016811 (the smoother base `+0x84 = +0x80` is stored before it);
  `SafeSoundCallback` 0x1001AE40, handler 0x1001B055 (returns the continuation 0x1001B05B, which
  jumps to 0x1001B03D).
- **Trigger.** A PCF wave of 55, 111 or 113..127 (OB-5), routed to a channel.
- **Effect.** Every call faults in its first PCF block. Combined with OB-8 the export is silence
  for the whole song: everything is silenced, not only the PCF's channel.
- **Corpus.** `pcf_lp_wave127`, `pcf_bp_wave127` (all zeros, sha256 `0a90d04342bf`, identical to
  `silence`).
- **rb.** `frame::Fault::PcfZeroLength`: the block and the render call end there; `Engine::fault`
  records the first fault and the count; the `rb` CLI warns when the song may route the PCF.
- **Home.** `docs/initial_investigation.md` section 8.2, `docs/export_state.md` section 13.

### OB-7: a stale stack quotient for a PCF resolution other than 8, 16 or 32 [G]

- **What.** The tick quotient is `tick/24`, `/48` or `/96` by resolution 32, 16 or 8; any other
  resolution leaves the stack slot `var_1C` untouched, so the step uses whatever the last write
  left.
- **Where.** The PCF block 0x100166D0 (the `idiv` at 0x10016811 takes the quotient).
- **Trigger.** Waves 55..127 with a resolution byte other than 8/16/32 and a non-zero length
  (all 73 waves of 55..127 read such a resolution; 17 of them also have length 0, OB-6).
- **Effect.** The step follows an unrelated number. Not observed: no corpus song reaches it with a
  trigger.
- **Corpus.** `pcf_lp_wave64`, `pcf_bp_wave64` reach it but never trigger (all steps 0).
- **rb.** `pcf.rs` uses the engine's last quotient, a [G] stand-in for the stack slot.
- **Home.** `docs/initial_investigation.md` section 8.2.

### OB-8: the export buffer is never cleared, so a faulting chunk repeats the previous one [C for the allocation, G for the content]

- **What.** The exe's export `sub_427988` allocates one buffer with `operator new(0x2000)` and
  `sub_4280D5` renders every chunk into it; nothing clears it. A render call that writes nothing
  (OB-6) leaves the previous chunk (or uninitialized heap memory at the start), and the writer
  writes it again.
- **Where.** `sub_427988`: `new(0x2000)` result stored at 0x427C2F; `sub_4280D5`: callback at
  0x4281C3, then a virtual call (vtable slot 3 of `[ebp+8]`, the writer [G]: the object is not
  identified) at 0x4281F2; a chunk is at most 0x800 frames (`cmp ..., 800h` at 0x428186).
- **Trigger.** Any render call that returns without writing: today only OB-6.
- **Effect.** A fault after the first chunk repeats the last good chunk; a fault from the start
  writes the initial buffer (zeros, observed).
- **Corpus.** `pcf_lp_wave127`, `pcf_bp_wave127` (a fault from the start: zeros).
- **rb.** The export shares one zeroed buffer (`frame/export.rs`), as rbhost does. [G] for a heap
  buffer that is not zero.
- **Home.** `docs/initial_investigation.md` section 8.2, `docs/export_state.md` section 13.

### OB-9: distortion amount and shape above 127 index past their tables [C: no check; G: reachable from a file]

- **What.** The distortion update indexes the 5x5 grid, `T` and the shaper `W` with the amount and
  shape bytes, with no check.
- **Where.** Update 0x10010894: 0x10010917 `mov bl,[ecx+3Ah]` (amount) then 0x10010932 `fld
  [4*ebx+0x10054550]` (T), and the grid index at 0x100109DC `fld [4*edx+0x10151AB8]` from a
  `fistp`; no `cmp` guards either [C]. Block 0x10010A50.
- **Trigger.** A shape or amount byte above 127. A TRAK event cannot (the exe's range is 0..127);
  an RBS file's DIST bytes can: the parse `sub_4345A6` (called at 0x435B28) and the packer
  `sub_423048` copy them unchanged [C for the parse and the packer, task-055; no file tried]. The
  panel's sub 92 sends 0..127 only.
- **Effect.** The DLL reads adjacent memory (the next tables).
- **Corpus.** None.
- **rb.** `dist.rs:117` clamps to 127 (`x.min(127)`; whether to keep it for song files is
  task-071), a deliberate deviation: the original's
  adjacent memory is not modelled.
- **Home.** `docs/initial_investigation.md` section 8.1.

### OB-10: delay pan above 127 indexes past `PAN[127]` [C: no check; G: reachable from a file]

- **What.** The send's pan indexes row 127 of the pan table (`flt_10054150` = `PAN[127]`) with the
  pan byte, unchecked.
- **Where.** Delay update 0x1000F754 (targets `+0x54`, `+0x58`): 0x1000F8A5 `mov al,[ebx+38h]`
  then 0x1000F8B1 `fadd [8*eax+0x10054150]` (= `PAN[127][0]`); no `cmp`.
- **Trigger.** A delay pan byte above 127 (an RBS pattern-mode byte, not a TRAK event): the DELY
  parse `sub_43465C` (called at 0x43589A) and the packer `sub_422EB7` copy it unchanged [C for
  the parse and the packer, task-055; no file tried]. The panel's sub 84 sends 0..127 only.
- **Effect.** Reads past the row, into the next table.
- **Corpus.** None.
- **rb.** `delay.rs:75` clamps to 127 (slot 11, the data block and so the panel's sub 84);
  whether to keep the clamp for song files is task-071.
- **Home.** `docs/initial_investigation.md` section 8.4.

### OB-11: PCF decay and resonance bytes past their tables [C: no check; G: reachable from a file]

- **What.** `dec = dword_100332D0[decay >> 3]` (16 entries) and the resonance row
  `(res*8) >> 7` (8 rows) are not bounded for bytes above 127.
- **Where.** The PCF block 0x100166D0. Decay: 0x1001684C `mov cl,[esi+3Dh]` loads the byte,
  0x1001684F `sar ecx,3` shifts it, then 0x1001685E `mov ebx,[4*ebx+0x100332D0]` (bytes
  `8A 4E 3D` and `C1 F9 03` in the file). Resonance row: 0x1001691B `shl eax,3` / 0x1001691E `sar eax,7`, then
  `lea eax,[4*eax+0x1010C270]` at 0x10016941 (and the 0x10100270 table at 0x10016AC2). No `cmp`
  guards any of them.
- **Trigger.** A decay or resonance byte above 127 (an RBS file) [G].
- **Effect.** Reads past the tables.
- **Corpus.** None.
- **rb.** `pcf.rs:333` (`.min(15)`) and `pcf.rs:416` (`.min(7)`), task-027's deliberate clamps.
- **Home.** `docs/initial_investigation.md` section 8.2.

### OB-12: the delay's 25-update tempo average is dead code [C]

- **What.** The update averages the tempo over 25 updates into `+0x48`, `+0x4C`, `+0x50`, but
  overwrites `+0x4C` with the current tempo before the length reads it.
- **Where.** 0x1000F7DD `call sub_1001FCF0`, 0x1000F7E2 `mov [ebx+4Ch], eax` runs on every
  update. Both length reads see that value: 0x1000F7EC (the x3 formula, `[ebx+36h] == 0`) and
  0x1000F83F (the x4 formula, otherwise); the `jne` at 0x1000F7EA only chooses the formula. The
  other `+0x4C` accesses are the average's own: the writes at 0x1000F7AD, 0x1000F7B5, 0x1000F7BA
  and the read at 0x1000F773 that feeds them. So the average is dead on every path.
- **Trigger.** Always (every delay update).
- **Effect.** None on the output beyond the model: the delay length follows the tempo at every
  update instead of a 25-update average.
- **Corpus.** Every song with an enabled delay.
- **rb.** `delay.rs` follows the tempo directly.
- **Home.** `docs/initial_investigation.md` section 8.4.

### OB-17: the panel's pattern and mixer-channel messages store with no bound on the pattern, step, instrument or channel [C: no check; G: the effect]

- **What.** The live device messages (msg 41 with a device's own sub) compute their store
  address from the message bytes and check only the 303's step field (0..5) and the header field
  (0, 1). A pattern above 31, a step above 15, a drum instrument above 11 or a mixer channel
  above 3 writes into another step or pattern or past the pattern array or the channel blocks,
  into the rest of the device object.
- **Where.** 303 sub 42 0x10025F25 (`+0x3C`/`+0x3D + 98p`: pattern 32's header lands on the live
  enabled byte `+0xC7C`); 303 sub 45 0x10025FC4 (`+0x3E + 98p + 6s + field`: a step above 15
  writes into the next pattern); 808 subs 59 0x10028EFE (`+0x50`/`+0x51 + 194p`) and 62
  0x10028F9D (`+0x52 + 194p + 12s + inst`); 909 subs 76 0x1002C57F and 79 0x1002C61E (the same
  from `+0x51`); mixer sub 147 0x100159DB (`+0x40 + 6c + k`: channel 4 writes at `+0x58`, the
  pattern-mode copy). Also the unsent subs 43, 46, 60, 63, 77, 80 and 148.
- **Trigger.** Only a message the exe never sends: the panel addresses its current pattern
  (0..31), steps 0..15, instruments 0..11 and channels 0..3 (`docs/song_load.md` section 4.10).
- **Effect.** A store into another pattern, the live parameters or other device state [G: not
  run].
- **Corpus.** None.
- **rb.** `crates/rb-engine/src/frame/edit.rs` (module doc, lines 13-19): within the pattern
  array rb stores at the same flat address (another step or pattern); a store past the array is
  dropped, and `Engine::mixer_channel` drops a channel above 3.
- **Home.** `docs/song_load.md` section 4.10 ("Bounds").

## 3. Export (Rebirth.exe)

See OB-8 for the unclean buffer.

## 4. Panel (Rebirth.exe)

### OB-13: the 303 Note/Pause key and the drum AC key are VK_OEM_PLUS, not the documented `-` [C]

- **What.** The key translation maps the virtual key VK_OEM_PLUS (0xBB) to the panel key code 28,
  the code the TB-303 handler uses for Note/Pause and the drum tap handler for AC. VK_OEM_MINUS
  (0xBD) has no code. The manual (pages 43, 225, 226) documents the `-` key.
- **Where.** `sub_492312` (case 187 -> code 28); TB-303 `sub_43F673` (case 28 -> Note/Pause
  button `+0x2E0`); TR-808 `sub_44311B` (case 0x1C -> AC).
- **Trigger.** Program Synth from Keyboard on a US layout: `-` does nothing, `=` toggles the step's
  rest. On a Swedish layout the VK_OEM_PLUS key is probably the key right of `0`, where the
  manual's picture has it, which would mean the table was made on such a keyboard [G: not checked
  on a Swedish layout].
- **Effect.** UI only; no audio path.
- **Corpus.** None (no song data involved).
- **rb.** Not decided: task-054 (`Decide:` the UI's keyboard mapping); `docs/ui_spec.md`
  section 10 has both facts.
- **Home.** `docs/ui_spec.md` sections 10 and 13.

## 5. Mods (Rebirth.exe)

### OB-14: the open mod's table of 200 EMBF entries has no bound check [C: no check; G: the effect]

- **What.** `COpenMod` (`sub_41813E`) keeps one 276-byte entry per EMBF chunk (name, payload
  offset, size, ordinal) in a fixed table of 200 entries and writes entry `count` for every EMBF
  of the file without comparing `count` with 200.
- **Where.** The table: 200 entries (`mov [ebp+var_23C], 0C8h` at 0x4182ED, stride 0x114) from
  object offset 0x69C (1692); the count is the word at 0xDE3C (56892 = 1692 + 200 * 276). The
  write: 0x418758..0x41876B (`count * 0x114 + 0x69C`), `strcpy` at 0x418786, the fields at
  0x41879A..0x4187BC, `count + 1` at 0x4187DA.
- **Trigger.** A mod with more than 200 EMBF chunks.
- **Effect.** Entry 200 starts at the count word itself, so the 201st name overwrites the count
  and later entries land at an index taken from the name's bytes, past the object [G: not run].
- **Corpus.** None: every known mod has at most 106 EMBF chunks (docs/rbm.md 11.1).
- **rb.** `rb recrypt` refuses a result with more than 200 EMBF chunks (`rbm::MAX_EMBF`); the
  reader parses any count, as it should for files the original never opens.
- **Home.** `docs/rbm.md` section 12.

## Quirks that are not claimed as defects

### Q-1: the export's frame count is one frame short of an exact length [C]

- **What.** The export loop counts frames in double precision: `n = trunc((end - pos) / (tempo *
  (192/2646e6)))` per chunk of at most 2048 frames, and `pos += (tempo*n)*(192/2646e6)`. The
  accumulated rounding makes the total one frame less than the exact value.
- **Where.** 0x428165..0x428183 (`fild`, `fdiv`/`fmul` of `dbl_4BBE08` and `dbl_4BBE10`, `__ftol`).
- **Trigger.** Most exports: 152 of the 153 references end on a chunk that is not a multiple of 64.
  Re-running the loop in IEEE double gives 88199 frames for one bar at 120 BPM (exact 88200) and
  18899999 for Fast Forward (140 BPM, 250 bars; exact 18900000).
- **Verdict.** Not a logic bug: it is the arithmetic of the original, which the sample-exact goal
  requires rb to repeat. It is listed here because it is visible in the file length.
- **rb.** `crates/rb-engine/src/frame/export.rs`, which matches the frame count of all references.
- **Home.** `docs/initial_investigation.md` section 3 (the export loop).

### Q-2: state carried across msg 33 and across songs [C for the facts, see `docs/export_state.md`]

- **What.** Some device state is never reset by a song load or by msg 33: the TB-303 filter
  coefficients `+0xCF4..+0xD00` (set to 0 at construction only; ramped to the target every block,
  so they settle [G]); the 808 and 909 snare noise counters `word_1014CBD4` and `word_1014E548`
  (BSS, 0 at DLL load, `c = (c+1) & 31` per hit, global); the drum voices' sample pointer and
  filter state (rewritten at every trigger).
- **Verdict.** Not claimed as defects: they are ordinary static state that a design without
  host-driven resets would also have. They matter because they make an export depend on what ran
  before it in the process (the counters), which rb models by starting fresh (counters 0).
- **Home.** `docs/export_state.md` (the state tables near the 303 and drum rows).

### Q-3: the AIFF reader trusts COMM over the chunk sizes [C for the mono read]

- **What.** The DLL's AIFF reader reads the COMM fields from the chunk's start whatever its size,
  and reads `2 * frames` bytes of audio after the SSND header whatever the SSND size says.
- **Where.** COMM: `sub_10001674` (no size argument). SSND: `sub_10002FC4`, the header by
  `sub_10004F00`, the offset skip (vtable +0x2C), then the read of `2 * frames` bytes
  (0x10003060 `lea eax,[ebx+ebx]`, vtable +4 at 0x1000306F) for a mono file.
- **Trigger.** Mods: 19 AIFFs of Metallicon.rbm have a 12-byte COMM; TR808BD.aif of 8tRack.rbm,
  DJ Turn's 8-Track.rbm and DJ Turn's MOD.rbm has FORM and SSND sizes smaller than the audio
  that follows (21064 frames where SSND holds 15798; 18052 where it holds 13539).
- **Verdict.** Leniency, not a defect: the file holds the audio and the DLL plays all of it
  (rbhost `--dump-samples`: 42128 and 36104 bytes, equal to the file's audio; task-058).
- **rb.** `crates/rb-engine/src/samples/aiff.rs` reads both through to the end of the file.
- **Open.** A file that really ends before `2 * frames` bytes: the DLL ignores the read's result
  (0x10003071) and the stream's behavior at its end was not traced [G]; rb's policy is an error,
  which `Rbm::load_samples` propagates, refusing the whole mod rather than falling back for that
  slot. The stereo path (0x10003087, `sub_10009DD0`) was not examined. No mod reaches either.
- **Home.** `docs/rbm.md` sections 9 and 11.

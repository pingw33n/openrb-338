# Defects of the original (ReBirth 2.01)

orb reproduces the original's behavior, bugs included. This file lists every known defect of
`Rebirth.exe` (addresses `0x4xxxxx`) and `ReBirth.dll` (`0x10xxxxxx`) in one place. Each entry has
its home document, where the mechanism is written in full; **a newly found defect of the original
goes into this file as well as into its home document**, and the home document links here.

Fields: **What**, **Where** (addresses), **Trigger** (the song data or action that reaches it),
**Effect**, **Corpus** (the songs in `tests/songs` that reach it), **orb** (the model: file and
function, and any deliberate deviation), and a mark: [C] confirmed in the disassembly (every
address here was re-read with `llvm-objdump` on the installed exe and dll), [G] guessed or inferred.

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
- **orb.** `crates/orb-engine/src/load.rs` (comment "0x433558"), the same copy, unit-tested.
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
- **orb.** `load.rs`, `old_tom_tune` applied before the range check.
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
- **orb.** `seq.rs` (`SONG_END`) and the track insert drop events above 767231 like the DLL [G for
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
- **orb.** `crates/orb-engine/src/seq.rs`, the same wrap and chase.
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
- **orb.** `crates/orb-engine/src/load/rb1x.rs` (`load_1x`) refuses the song with
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
- **orb.** `rbs::rb1x::Block::decode` reads missing bytes as zero (a deviation where the original
  is undefined); a sysex that fails the check, or a second song sysex, is an error in `load_1x`.
- **Home.** `docs/song_load.md` section 12.3.

### OB-20: a song load the node pool refuses leaves its half-loaded song in the engine [C: the code; G: the effect]

- **What.** When ReBirth.dll refuses a msg-37 batch (more events than free nodes in the pool,
  reply `['K', 22]`), Rebirth.exe abandons the load with an exception and an alert, but sends the
  engine nothing to close the song it opened for the load: the engine's song keeps the batches it
  took.
- **Where.** The DLL refusal `sub_10021010` (0x1002105B, reply 0x100210ED); the exe throw
  0x436E95 after the `'K'` test 0x436E65 (1.x: the test 0x430305), the rethrows and conversion to `XSongOutOfMemory` (0x436EEA, 0x436AD7,
  0x426430, 0x425FDB), the alert at 0x40B778 (`sub_475D5C(0x1DCF592A)`), File > Open's
  `catch(...)` 0x40A3D4. No unwind of `sub_436B6F`, `sub_434D05`, `sub_425C4D`, `sub_40B575` or
  `sub_40A2A3` sends a message; the slot came from msg 52 (`sub_425A5D`).
- **Trigger.** Opening a song of about 100000 events or more (fewer when other songs are open).
- **Effect.** The alert "There is not enough memory for opening the Song." and no song window;
  the nodes of the batches before the refused one stay taken until the engine closes, so each
  such attempt lowers the free count for every later load and recording [G: the effect on a later
  load is inferred from the shared pool, not run].
- **Corpus.** None: the largest RBS file of `tests/songs` has 13003 track events
  (`shipped/Mutilated Species II.rbs`).
- **orb.** `Engine::load` stops at the refused batch with `Err(LoadRefused)` and the engine keeps
  the earlier batches, as the DLL does; orb has one pool per song (task-082), so the leak cannot
  reach another song, and orb's free count at a load is that of an engine holding no other song:
  orb accepts some songs the exe refuses (those the app refuses only because the Untitled song or
  other open songs, or an earlier refused load, hold nodes). `orb render` fails with the alert's
  text; the player and the UI do not yet refuse to open such a song (task-088).
- **Home.** `docs/song_load.md` section 5.2.

### OB-21: the song text's line breaks count twice against its 400 bytes, so a full text is cut on open [C: the code; G: the effect in the dialog]

- **What.** The USRI reader copies the 400-byte text field into memory with a 0x0A after each
  0x0D, and the added byte counts toward the same 400, so a stored text whose bytes plus line
  breaks pass 400 loses its tail.
- **Where.** `sub_434955` calls `sub_465D8D(src, 0x190, dst)` for the text (0x434975);
  `sub_465D8D` writes the 0x0A at 0x465DED and stops at `max` (0x465DA4, 0x465DE2).
- **Trigger.** A song whose text, with its line breaks, is longer than 400 bytes (a file made by
  another tool; the original's own Save cannot write one, since its writer `sub_465D0E` drops the
  0x0A of a text that fits in memory).
- **Effect.** The song information shows the shortened text, and a Save of the song writes it
  shortened. No effect on the audio.
- **Corpus.** None known.
- **orb.** orb reads the whole field (`rbs` `Usri`); File > Info limits an edited text to 400
  bytes with a line break counting two (`crates/orb-ui/src/info.rs`, `info::cost`), so the
  original reads every text orb writes whole.
- **Home.** `docs/song_load.md` section 13.

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
- **orb.** `crates/orb-engine/src/pcf.rs` reads `PCF_DATA`, the raw bytes 0x10033310..0x10034400
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
- **orb.** `frame::Fault::PcfZeroLength`: the block and the render call end there; `Engine::fault`
  records the first fault and the count; the `orb` CLI warns when the song may route the PCF.
- **Home.** `docs/initial_investigation.md` section 8.2, `docs/export_state.md` section 13.

### OB-7: a stale stack quotient for a PCF resolution other than 8, 16 or 32 [C]

- **What.** The tick quotient is `tick/24`, `/48` or `/96` by resolution 32, 16 or 8; any other
  resolution leaves the stack slot `var_1C` (entry `esp - 0x1C`) untouched, so the step uses
  whatever the last write left. The PCF's own quotient does not survive: between two PCF blocks
  other code at the same stack depth rewrites the slot. On the 808 and 909 channels it is the
  channel's audio slot, called from the same `esp` just before the inserts: its last call leaves
  its return address there, 0x10028278 (808) or 0x1002B8B4 (909), in every block. The 303 audio
  never writes the slot, so on the 303 channels it is the last of, in block order (task-072): the
  render call's lock (0, what Windows' `WaitForSingleObject` leaves there on the test machine),
  every 16th block the mixer update's compressor id, in a block that reaches a new tick the
  clock's tempo (BPM*1000), an enabled compressor on channel 0 for the PCF on channel 1 (its
  release factor 0x3F7F9724), and from the block before the 909's return address, an enabled
  compressor on channel 3 (0x3F7F9724) or the enabled delay's last right-output pointer
  (`0x100F012C + 4*(pos + 63)` for the block at frame `pos` of the render call). The addresses
  are the values at the preferred base 0x10000000: they move with the DLL's load address
  (`ImageBase`), so the step does too; the corpus and rbhost assume the preferred base, as for
  the pattern data (OB-5). The lock's 0 depends on the Windows build.
- **Where.** The PCF block 0x100166D0 (the `idiv` at 0x10016811 takes the quotient); the 808
  audio 0x100280B0 (`call sub_1001E4D0` at 0x10028273), the 909 audio 0x1002B6F0
  (`call sub_1001E9A0` at 0x1002B8AF), both from `esp0 - 0x18`; the mixer block calls them at
  0x10013580/0x1001359B and the inserts at 0x100135C3/D9/EF. The distortion (0x10010A50) does not
  touch the slot. On the 303 channels: the 303 audio 0x10023F10 and 0x100248C4 (no local or
  call that reaches it); the try-lock `sub_10009450` at 0x10013A5E in `sub_100139E0`; the mixer
  update 0x10012C3A; the clock 0x1001F3B3; the mono compressor 0x1000ECE9; the delay 0x1000FA9A
  ([G] its partial block's pointer is `song+0x408`, a heap address: derived statically from
  0x10013BAA, which renders that block into `song+0x20C+0x100c`; not observed).
- **Measured.** rbhost with a thunk in the PCF's vtable slot 15 logging `[esp - 0x1C]` and a
  hardware write watchpoint: the model matches all 66485 PCF blocks of the live test
  `pcf_junk_wave_on_the_303_channels` and five export songs (PCF on channels 0 and 1, compressor
  off, on the master, on channels 0 and 3, delay on and off, five tempos, both sound models).
  The sessions use 1024- and 2048-frame calls, so no measured PCF block followed a partial block
  with the delay on: the heap-address case above is static only.
- **Trigger.** Waves 55..127 with a resolution byte other than 8/16/32 and a non-zero length
  (all 73 waves of 55..127 read such a resolution; 17 of them also have length 0, OB-6).
- **Effect.** The step is the return address modulo the length: constant, so a wave with a
  trigger there triggers once; and the last step `+0x74` it leaves decides whether the first
  block after a switch to a real wave is a new step. Observed (task-069, rbhost `--live` with a
  write watchpoint): `full_mix` (wave 64, PCF on the 909) has `var_1C` = 0x1002B8B4 at every PCF
  block and `+0x74` = 4 = 0x1002B8B4 % 16; a return from song mode to a panel-set wave 30 starts
  on step 4 without a trigger.
- **Corpus.** `pcf_lp_wave64`, `pcf_bp_wave64` reach it but never trigger (all steps 0); the live
  tests `pcf_wave_after_returning_to_pattern_mode` and `pcf_junk_wave_on_the_303_channels`
  (wave 108, every step a velocity; `crates/orb-player/tests/live_messages.rs`).
- **orb.** `Frame::stack_word`, the word at the slot's address, set by every writer in the
  original's order (`Engine::render`: `LOCK_WORD`, `Frame::out_end`; `Frame::block`: the mixer
  update, `Sequencer::tick`, `ChannelDevice::stack_word` of the 808 and 909,
  `InsertEffect::stack_word` of the PCF and the compressor, `SendEffect::stack_word` of the
  delay) and passed to the inserts in `BlockContext::stack_word`. After a partial block with the
  delay on (a heap address) it is unknown and `pcf.rs` keeps its last quotient, a stand-in;
  with a lock word other than the test machine's 0 (another Windows) orb follows the test
  machine.
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
- **orb.** The export shares one zeroed buffer (`frame/export.rs`), as rbhost does. [G] for a heap
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
- **orb.** `dist.rs:117` clamps to 127 (`x.min(127)`; whether to keep it for song files is
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
- **orb.** `delay.rs:75` clamps to 127 (slot 11, the data block and so the panel's sub 84);
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
- **orb.** `pcf.rs:333` (`.min(15)`) and `pcf.rs:416` (`.min(7)`), task-027's deliberate clamps.
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
- **orb.** `delay.rs` follows the tempo directly.
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
- **orb.** `crates/orb-engine/src/frame/edit.rs` (module doc, "Bounds"): within the pattern
  array orb stores at the same flat address (another step or pattern); a store past the array is
  dropped, and `Engine::mixer_channel` drops a channel above 3.
- **Home.** `docs/song_load.md` section 4.10 ("Bounds").

### OB-18: a change recorded just before record off is lost [C: the code and the DLL run]

- **What.** The merge `sub_1001F520(track, T)` moves into the track only the recorded events at
  or before `T` and then frees the whole recording list. The record hook rounds a change made
  while playing **up** to the controller's unit (24 ticks, or a bar for the 303/808/909 pattern
  and on/off), so a change made less than a unit before the merge tick is above `T`.
- **Where.** 0x1001F71A..0x1001F754 (the copy loop stops at `T`), 0x1001F75B `sub_100149A0` (frees
  every node of `+0x198`); the rounding at 0x100214A7 (`sub_100146E0`).
- **Trigger.** Record off while playing (msg 35 `{2,0}`: the Record button, Stop, Rewind, Fast
  forward, Bar down, the switch to pattern mode) within a unit after a recorded change, e.g. a
  pattern change recorded in the same bar as the Stop that follows it. At the loop wrap (`T =
  loopEnd - 1`) a change rounded to the loop end goes to the loop start and is kept.
- **Effect.** The change is not in the track. It was heard live (the store), and the record-off
  chase in song mode sets the controller back to the track's value at once.
- **Corpus.** None (no reference capture records). Checked by rbhost: the session
  `sweep_session` of `crates/orb-player/tests/live_messages.rs` moves a level and turns record off
  in the same call (rounded tick 1272 > T 1257); the DLL's WAV equals the one of the same session
  without the move.
- **orb.** The same (`Track::merge` in `crates/orb-engine/src/seq.rs`).
- **Home.** `docs/initial_investigation.md` section 5.3.

### OB-19: the merge frees the track's applied node and leaves the pointer to it [C: the code; G: the effect]

- **What.** The merge unlinks and frees a track event whose controller was recorded, without
  looking at the track's applied pointer `+0x228`; when that pointer is the freed node it points
  into the free pool (the freed node's next becomes the pool's free list, `sub_10014484` at
  0x10014491).
- **Where.** 0x1001F62D `sub_10014B30` (unlink and `sub_10014484`); `+0x228` is set only by the
  chase (`sub_100151F0`) and the apply loops.
- **Trigger.** Record off in **pattern mode** after recording in song mode (the only merge with no
  chase after it): the exe's switch to pattern mode sends `{0,0}` and then `{2,0}`, which merges
  with no chase. Every other merge (loop wrap, record off in song mode, song end) chases right
  after, which sets the pointer again; so does the locate of the next switch to song mode.
- **Effect.** None from the panel: in pattern mode the clock applies no events, and the next
  song-mode switch chases. An apply loop from the dangling pointer (a locate forward while
  stopped with record on in pattern mode, which the panel cannot send because Record does nothing
  in pattern mode) would walk the free list [G].
- **Corpus.** None.
- **orb.** `Track::merge` moves its index to the node before the freed one; no path orb models
  reads it before the next chase.
- **Home.** `docs/initial_investigation.md` section 5.3.

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
- **orb.** Not decided: task-054 (`Decide:` the UI's keyboard mapping); `docs/ui_spec.md`
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
- **orb.** `orb mod recrypt` refuses a result with more than 200 EMBF chunks (`rbm::MAX_EMBF`); the
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
  requires orb to repeat. It is listed here because it is visible in the file length.
- **orb.** `crates/orb-engine/src/frame/export.rs`, which matches the frame count of all references.
- **Home.** `docs/initial_investigation.md` section 3 (the export loop).

### Q-2: state carried across msg 33 and across songs [C for the facts, see `docs/export_state.md`]

- **What.** Some device state is never reset by a song load or by msg 33: the TB-303 filter
  coefficients `+0xCF4..+0xD00` (set to 0 at construction only; ramped to the target every block,
  so they settle [G]); the 808 and 909 snare noise counters `word_1014CBD4` and `word_1014E548`
  (BSS, 0 at DLL load, `c = (c+1) & 31` per hit, global); the drum voices' sample pointer and
  filter state (rewritten at every trigger).
- **Verdict.** Not claimed as defects: they are ordinary static state that a design without
  host-driven resets would also have. They matter because they make an export depend on what ran
  before it in the process (the counters), which orb models by starting fresh (counters 0).
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
- **orb.** `crates/orb-engine/src/samples/aiff.rs` reads both through to the end of the file.
- **Open.** A file that really ends before `2 * frames` bytes: the DLL ignores the read's result
  (0x10003071) and the stream's behavior at its end was not traced [G]; orb's policy is an error,
  which `Rbm::load_samples` propagates, refusing the whole mod rather than falling back for that
  slot. The stereo path (0x10003087, `sub_10009DD0`) was not examined. No mod reaches either.
- **Home.** `docs/rbm.md` sections 9 and 11.

### Q-4: a mod change from the Mods menu activates another open song and back [C: the exe code; C: the MDI order, from a probe; G: the composed sequence]

- **What.** Changing the mod rebuilds the song window (`sub_42D80D`). Its destructor sends
  WM_MDIDESTROY (0x49626B) [C]. On the test machine's Windows, a probe program that mimics these
  calls showed that the next MDI child gets WM_NCACTIVATE 1 before the old one is destroyed, when
  the frame is the active window [C, probe]. In the exe, WM_NCACTIVATE 1 runs `sub_494DF1` → slot
  14 `sub_42D7BE` → `sub_40D163`, which sends top-level 53 for that window's song [C]. So with
  another song window open, the exe sends 53 for the other song, and the new window's constructor
  then sends 53 for the changed song (0x42E5AE) [G: inferred, not observed in Rebirth.exe].
- **Where.** `sub_42D80D` (0x42D886 mark, 0x42D892 `sub_473401` deletes, 0x42D8BE new window);
  `sub_49617E` (0x49625C `push 221h`); `sub_494DF1` (0x494E20 `call [edx+38h]`); 0x42D7DD;
  0x42E5AE. All re-read with llvm-objdump on the installed `Rebirth.exe`.
- **Trigger.** Two or more songs open; a mod chosen from the Mods menu (command `5000 + i`, the
  frame active). The Mod Selector's OK runs the same code while the dialog is up. The probe
  tested an owned popup being the active window, and then the other child got no WM_NCACTIVATE 1.
  [G] That the dialog is the active window when OK runs, and so that OK sends no msg 53, is
  inferred, not traced.
- **Effect.** A playing song: none (both activations only set the pending song, `sub_1001BDA4`). A
  stopped song: slot 6 runs on the devices of both songs (303 note-off release, drum voices
  silenced, the shared delay line cleared). [G] An audio callback may render the other song for
  the moment between the two messages (not observed).
- **Verdict.** A side effect of the window rebuild, not claimed as a defect; there is nothing to
  reproduce in an export.
- **Corpus.** None (the corpus uses one song, and its exports change no mod).
- **orb.** Not modelled: orb opens one song.
- **Home.** `docs/rbm.md` section 14.5.

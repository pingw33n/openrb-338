# rbhost: the original ReBirth.dll, offline

`rbhost.exe` loads the original `ReBirth.dll` into its own process on the Windows test machine,
plays the panel's side of the engine's shared-memory port, replays the song-load messages of an
`.rbs` song, and renders the song's loop the way File > Export Loop as Audio File does. There is no
`Rebirth.exe`, no window, no realtime audio thread and no registration: one thread, deterministic.
The facts it relies on are in `docs/export_state.md` (sections 2.7, 4 and 12) and
`docs/song_load.md`.

| File | What |
|---|---|
| `rbhost.c` | the host (C, i686 Windows) |
| `build.sh` | builds `build/rbhost.exe` on the Mac with llvm-mingw |
| `rbsmsg.py` | `.rbs` to the engine's load messages, written as a `.rbhm` message file |
| `run.py` | build, convert, upload to `C:\re\rb\rbhost`, run under one permit, download |
| `rbsmsg_test.py` | unit tests of the conversion |

## Toolchain

`build.sh` uses the pinned llvm-mingw of `~/dev/dcre/common/tools/fetch_llvm_mingw.sh` (release
20260922, installed in `$LLVM_MINGW`, default `~/llvm-mingw`; run the script once to install it).
The exe is static (`-static`), so the machine needs nothing besides `ReBirth.dll` and the system DLLs
it imports (msvcrt, winmm, version).

## Running

```
tools/rbhost/run.py -o OUT tests/songs/diag/songs/303_accent0.rbs        # OUT/303_accent0.wav + .json
tools/rbhost/run.py --runs 2 -o OUT SONG.rbs                             # OUT/SONG.1.wav, OUT/SONG.2.wav
tools/rbhost/run.py --dump-tables --dump-state -o OUT SONG.rbs           # OUT/SONG.tables/, OUT/SONG.state/
tools/rbhost/run.py --dump-float -o OUT SONG.rbs                         # OUT/SONG.float/float.bin
tools/rbhost/run.py --idle-sweep 6400:9600:16 --match APP.wav -o OUT SONG.rbs   # fit N to an app export (slow: fit.py)
tools/rbhost/run.py --mod MOD.rbm --rb20ful RB20FUL.DAT --dump-samples -o OUT SONG.rbs  # with a mod
```

`--mod` (task-030, `docs/rbm.md`) uploads the mod to `C:\re\rb\rbhost\<tag>.rbm` with its message
55, built by `tools/rbm.py` with the key table from the local `RB20FUL.DAT`; rbhost sends it instead
of message 56 {411} and creates the songs with the mod's id (`--mod-id`, default 412).
`--dump-samples` writes the mod's 46 loaded samples from the DLL's sample table to
`OUT/SONG.samples/slotNN.bin` (i16, the CWaveSample buffers). Mods, `RB20FUL.DAT` and the dumps are
Propellerhead material: keep OUT outside the repository.

`--remote-dir NAME` (run.py and fit.py) works in `C:\re\rb\NAME` instead of `C:\re\rb\rbhost`:
rbhost.exe, the messages, the mod and the renders go there; delete the folder when done.

`run.py` holds one permit of `redc-win-run` (rbhost takes neither the keyboard nor the screen) and
stops only the processes it started. The sidecar JSON has the arguments, rbhost's log, the hashes and
the RIFF check of `tools/rb_export.py` (the frame count must equal the export length formula).
`float.bin` (`--dump-float`, task-023) is the export's float output before the int16 conversion,
f32 left/right pairs per frame, read from `sub_100139E0`'s buffers `unk_100E012C`/`unk_100F012C`
after each chunk; it does not change the render.

On the machine, `rbhost.exe [options] SONG.rbhm OUT.wav` (`rbsmsg.py SONG.rbs SONG.rbhm` makes the
message file). rbhost does not write to the installation; it loads the DLL from `C:\ReBirth RB-338\ReBirth.dll` unless `--dll`
names another copy.

## What it does, in order

1. Sets the x87 control word to 0x027F, the app's (53-bit precision; `docs/export_state.md` 12).
2. `LoadLibrary`, optional `--drum-counters` preset of the four process-global drum counters,
   `EngineOpen()`, then opens the two rings the DLL created ("ReBirth Panel->DLL" to write,
   "ReBirth DLL->Panel" to read).
3. Top-level 50 (handshake, reply '@'), 56 {411} (the built-in mod "Standard ReBirth", reply 'C'),
   an empty "untitled" song (52, then 53 to activate it; `--no-untitled` skips it), 52 {411} (the
   song, reply 'A' + slot), optional 58 {411} (`--startup-sound`). The untitled song matters: the
   activation of the song calls note-off on the previously active song's devices, or on the song's
   own when none is active, which changes its 303s' idle state (`docs/export_state.md` section 12).
4. The song's load messages (`docs/song_load.md` section 2), then 53 {slot} (activate).
5. `--idle-blocks N`: N 64-frame blocks of the active, stopped song, as 1024-frame calls.
6. The export (`docs/export_state.md` section 4): stop, `--post-stop-blocks M` more idle blocks,
   locate to the loop start and play, the 2048-frame chunk loop with the exe's double-precision
   length arithmetic (`sub_4280D5`), then stop, msg 33 and a 0-frame call.

Every message is dispatched as the app does without an audio channel: 0-frame `SafeSoundCallback`
calls drain the port, `EngineIdle` runs the queued types (52, 56, 58), and the replies are read back.
`--first-chunk-drain` leaves locate and play in the port for the first export chunk instead, as in
the app.

## Fitting N to an app export

An app export depends on the number of idle blocks the song rendered before it (`docs/export_state.md`
sections 7, 9, 11). `--idle-sweep A:B:S --match APP.wav` renders one file per N in `range(A, B, S)`
and stops at the first render whose samples equal the app's (`--match-hash`: the hashes are compared on
the machine and only a match is downloaded). Results so far are in `docs/export_state.md` section 12.

**`fit.py`** (task-038) is the fast way, about 0.5 s per song instead of 10 to 40 s:

```
tools/rbhost/fit.py --app-dir APPOUT -o OUT SONG.rbs...      # APPOUT: rb_export.py's <stem>.wav + .json
tools/rbhost/fit.py --app-dir APPOUT --all --max-candidates 31 -o OUT SONG.rbs   # every candidate: is the fit unique?
```

It tries `N` (multiples of 16) outward from `N_t + 48`, `N_t` being the blocks between "song opened"
and "export started" in the app sidecar's timeline (`docs/export_state.md` sections 11.6 and 13), and
learns the offset `N - N_t` from the songs it has fitted. It runs `--jobs` rbhost processes at the same
time (`--private-port`, one permit each), uploads rbhost and all the messages once and compares
the WAV hashes on the machine. `OUT/fit.json` has the fitted N, the number of renders and the times.

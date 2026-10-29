# The original's run-time tables

Dumped once from the running original DLL by rbhost (task-019); `manifest.json` lists every file
with its address and shape. The unit tests in `crates/orb-engine/src/tables/tests.rs` compare the
generated tables with these files bit for bit.

```
tools/rbhost/run.py --dump-tables -o OUT tests/songs/diag/songs/303_accent0.rbs
```

| What | sha256 |
|---|---|
| `C:\ReBirth RB-338\ReBirth.dll` on the test machine (= the installed and the CD's ReBirth.dll) | `4c3a6441a2cd50f797d99b1c5b6cf0aaf198c7ad8ca2648196b530c2e3e58b12` |
| `rbhost.exe` (`tools/rbhost/rbhost.c` with the `smooth303` and `smooth_pcf` items) | `f397f2cd2aa43ba12d47179748de46a7d02a56f4b45e86901e6dea8b1920de38` |
| the song `303_accent0.rbs` | `5262ddd1962dccaea7435cf8fc3aefa1eb0c1d2c4004cd6a33cd7d2267827c2d` |

The tables do not depend on the song: they are built by the constructors of the first song and
by the DLL's static initializers. An earlier run without the two smoother items gave the same
bytes for the ten other files.

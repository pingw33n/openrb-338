# original/

The original ReBirth RB-338 2.01 program. It is Propellerhead's copyrighted software, so it is
**not in git**: everything in this folder except this README is ignored (`.gitignore`).

Put here the files of your own installation (the `Rebirth.exe` and `ReBirth.dll` of
`C:\ReBirth RB-338`), with these SHA-256:

| File | Bytes | SHA-256 |
|---|---|---|
| `ReBirth.dll` | 2,472,960 | `4c3a6441a2cd50f797d99b1c5b6cf0aaf198c7ad8ca2648196b530c2e3e58b12` |
| `Rebirth.exe` | 1,594,880 | `6a651e8164b8444a5f0639cb6a2fef2aa43c5500bd9c5288627c2030b047f876` |

The `Rebirth.exe` of the installation is the patched one; the CD installer's
(`e69fef41...7004`, docs/rbm.md 13.1) differs from it.

rb never needs these files except for the reverse engineering (IDA, `llvm-objdump`). The tests
do not read this folder: `RB_CD_ISO` (the 2001 reissue
CD image, `rb-testkit`) provides the same `ReBirth.dll` to them, and they skip with a message
when it is not set.

The IDA databases of these two files go in `ida/` (`Rebirth.exe.i64`, `ReBirth.dll.i64`), also
local and untracked. IDA's working files (`*.id0`, `*.nam`, ...) appear here when a database is
open in IDA.

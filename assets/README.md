# assets

- `ui/`: the panel's SVG art set (`crates/rb-ui-art`).
- `Standard.rbm`: the standard mod, rb's built-in kit (task-067, docs/rbm.md section 13).

## Standard.rbm

A recrypted ReBirth mod named "Standard ReBirth": the 46 built-in drum samples of ReBirth 2.01
(ReBirth.dll's AIFF resources 1000..1045, byte for byte) and its standard panel (Rebirth.exe's 58
panel JPEG resources and its splash, resource 903). It is Propellerhead's material, committed by
the user's decision of 2026-10-27 as the one exception to the rule that nothing of the original
is in the repository. `crates/rbm/src/standard.rs` embeds it (`include_bytes!`): `rb render`,
`rb compare`, `rb play` and rb-ui take the built-in kit from it and need no file of the original.

Made with:

```
rb mod pack-standard --rebirth-dll original/ReBirth.dll --rebirth-exe CD/Rebirth.exe \
    -o assets/Standard.rbm
```

| File | Size | SHA-256 |
|---|---|---|
| `original/ReBirth.dll` (ReBirth 2.01, installed) | | `4c3a6441a2cd50f797d99b1c5b6cf0aaf198c7ad8ca2648196b530c2e3e58b12` |
| `Rebirth.exe` of the ReBirth CD (from the CAB at file offset 0x30266C of `Install ReBirth RB-338.exe`, extracted with bsdtar; not committed) | 1,606,656 | `e69fef4153856f4ad985797a3f312a0027633c00f835c3348415b031bbd87004` |
| `assets/Standard.rbm` | 2,572,792 | `6355b5f14fc2ebfe7ed9e6e42208f97817f96c5e30ed42fdca4205d3cdab5fbe` |

The CD's exe gives the splash without the Steinberg logo that `original/Rebirth.exe` (patched,
installed) carries; the 58 panel bitmaps are the same in both. The build is deterministic: with
`RB_CD_REBIRTH_EXE` naming the CD's exe, `cargo test --release -p rb-cli --test standard_mod`
rebuilds the file and checks it byte for byte.

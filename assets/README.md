# assets

- `ui/`: the panel's SVG art set (`crates/orb-ui-art`).
- `app_icon.ico`: the Windows icon (the `app_icon` part at 16 to 256 px), made by
  `cargo run --release -p orb-ui-art --bin ui-art -- ico` and embedded in `OpenRB-338.exe` by
  `crates/orb-ui/build.rs` (task-111).
- `Standard.rbm`: the standard mod, orb's built-in kit (task-067, docs/rbm.md section 13).

## Standard.rbm

A recrypted ReBirth mod named "Standard ReBirth": the 46 built-in drum samples of ReBirth 2.01
(ReBirth.dll's AIFF resources 1000..1045, byte for byte) and its standard panel (Rebirth.exe's 58
panel JPEG resources and its splash, resource 903). It is Propellerhead's material, committed by
the user's decision of 2026-10-27 as the one exception to the rule that nothing of the original
is in the repository. `crates/rbm/src/standard.rs` embeds it (`include_bytes!`): `orb render`,
`orb compare`, `orb play` and OpenRB-338 take the built-in kit from it and need no file of the
original.

Made with:

```
orb mod pack-standard --rebirth-dll PATH/ReBirth.dll --rebirth-exe CD/Rebirth.exe \
    -o assets/Standard.rbm
```

| File | Size | SHA-256 |
|---|---|---|
| `ReBirth.dll` (ReBirth 2.01, installed; also on the CD) | 2,472,960 | `4c3a6441a2cd50f797d99b1c5b6cf0aaf198c7ad8ca2648196b530c2e3e58b12` |
| `Rebirth.exe` of the ReBirth CD (from the CAB at file offset 0x30266C of `Install ReBirth RB-338.exe`, extracted with bsdtar; not committed) | 1,606,656 | `e69fef4153856f4ad985797a3f312a0027633c00f835c3348415b031bbd87004` |
| `assets/Standard.rbm` | 2,572,792 | `6355b5f14fc2ebfe7ed9e6e42208f97817f96c5e30ed42fdca4205d3cdab5fbe` |

The CD's exe gives the splash without the Steinberg logo that the installed `Rebirth.exe` (patched,
`6a651e8164b8444a5f0639cb6a2fef2aa43c5500bd9c5288627c2030b047f876`, 1,594,880 bytes) carries; the 58 panel bitmaps are the same in both. The build is deterministic: with
`ORB_CD_ISO` naming the CD image, `cargo test --release -p orb-cli --test standard_mod` takes the
DLL and the exe from it (`orb-testkit`, task-121), rebuilds the file and checks it byte for byte.
`orb mod pack-standard --iso CD.iso -o FILE` (task-065, docs/rbm.md 16) reads both files straight
from the CD image and gives the same bytes.

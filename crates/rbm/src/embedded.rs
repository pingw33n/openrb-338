//! The `RB20FUL.DAT` key material that orb embeds (task-059; the user's decisions of 2026-10-27:
//! embed a minimum, and of that a 2 KB window, so that recrypted mods stay compact):
//! `WINDOW_LEN` bytes at `WINDOW` of the user's file. A mod written by [`crate::recrypt`] steers
//! both of its key pages to `WINDOW`; every key's period is at most 2047, so every file of it is
//! descrambled with these bytes alone (docs/rbm.md section 12).
//!
//! `rb20ful_window.bin` was generated from the RB20FUL.DAT whose SHA-256 is [`DAT_SHA256`]:
//! `tail -c +$((WINDOW + 1)) RB20FUL.DAT | head -c WINDOW_LEN`. The gated test
//! `embedded_window_is_the_dats` of `tests/rbm.rs` checks the bytes, [`WINDOW_SHA256`] and
//! [`DAT_SHA256`] against the DAT of the CD image (`ORB_CD_ISO`).

/// The offset in `RB20FUL.DAT` of the embedded bytes: the start of PBE2.rbm's page 0
/// (docs/rbm.md 5.3), a page the original already reads for a shipped mod.
pub const WINDOW: u64 = 118_280_160;

/// The number of embedded bytes: the longest period of the descrambler, `(17 * key) % 1925 +
/// 123` at most 2047 (the last byte of a 2048-byte page is never used; docs/rbm.md section 12).
pub const WINDOW_LEN: usize = 2047;

/// The SHA-256 of the `RB20FUL.DAT` the bytes were taken from (134,217,984 bytes).
pub const DAT_SHA256: &str = "62071e999697e7ac49936cf605eb2c888d1097c6c6b85ea10c30429a482f2797";

/// The SHA-256 of [`WINDOW_BYTES`].
pub const WINDOW_SHA256: &str = "2c292d920763bc50b058feb019eccc839369528aa92ac4cab3935b8c744956bc";

/// `RB20FUL.DAT[WINDOW .. WINDOW + WINDOW_LEN]`.
pub const WINDOW_BYTES: &[u8; WINDOW_LEN] = include_bytes!("rb20ful_window.bin");

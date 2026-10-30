//! Checks of the key file and of a mod's files before `orb mod recrypt` writes anything (task-064,
//! docs/rbm.md 12.5).
//!
//! A different `RB20FUL.DAT` with the right header gives a silently wrong recrypt: the samples
//! decrypt to garbage, which is scrambled again against the embedded window, and the round trip
//! check of [`crate::recrypt`] passes because it reuses the same wrong key. So the key file is
//! identified by its SHA-256 ([`verify_dat`]) and the decrypted files are checked for what they
//! must be ([`validate_files`]).

use std::io::{Read, Seek, SeekFrom};

use orb_engine::samples::Sample;
use sha2::{Digest, Sha256};

use crate::{
    DAT_BODY, DAT_HEADER, DAT_MAGIC, KeyTable, Rbm, RbmError, be32, embedded, is_slot_name,
};

/// The SHA-256 of everything `r` yields, as 64 lowercase hex digits.
pub fn sha256_hex<R: Read>(r: &mut R) -> std::io::Result<String> {
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = r.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Checks that `dat` is the `RB20FUL.DAT` the embedded window comes from: the header magic, the
/// length and the SHA-256 of the whole file against `expected` (normally
/// [`embedded::DAT_SHA256`]). Reads the whole file.
///
/// Only one CD edition's file is known (the 2001 reissue); another edition is an error that
/// says so. On a hash mismatch the error also says whether the bytes at [`embedded::WINDOW`]
/// are the embedded ones.
pub fn verify_dat<R: Read + Seek>(dat: &mut R, expected: &str) -> Result<(), RbmError> {
    let len = dat.seek(SeekFrom::End(0)).map_err(RbmError::Io)?;
    dat.seek(SeekFrom::Start(0)).map_err(RbmError::Io)?;
    let mut head = [0u8; 4];
    if dat.read_exact(&mut head).is_err() || be32(&head, 0) != DAT_MAGIC {
        return Err(RbmError::KeyFile(format!(
            "bad magic, not the ReBirth CD's RB20FUL.DAT (expected 0x{DAT_MAGIC:08X}); only the \
             2001 reissue CD's file is supported yet, another edition is not supported"
        )));
    }
    let want = DAT_HEADER + DAT_BODY;
    if len != want {
        return Err(RbmError::KeyFile(format!(
            "{len} bytes, expected {want}: not the ReBirth CD's RB20FUL.DAT, or truncated; only \
             the 2001 reissue CD's file is supported yet, another edition is not supported"
        )));
    }
    dat.seek(SeekFrom::Start(0)).map_err(RbmError::Io)?;
    let actual = sha256_hex(dat).map_err(RbmError::Io)?;
    if actual == expected {
        return Ok(());
    }
    let mut window = [0u8; embedded::WINDOW_LEN];
    let window_note = match dat
        .seek(SeekFrom::Start(embedded::WINDOW))
        .and_then(|_| dat.read_exact(&mut window))
    {
        Ok(()) if window == *embedded::WINDOW_BYTES => {
            "its bytes at the embedded window are the embedded ones, so the file differs \
             elsewhere (modified or corrupt)"
        }
        Ok(()) => "its bytes at the embedded window differ too: another edition of the CD",
        Err(_) => "its bytes at the embedded window could not be read",
    };
    Err(RbmError::KeyFile(format!(
        "SHA-256 {actual}, expected {expected} (the 2001 reissue CD's file). {window_note}. Only \
         that edition's RB20FUL.DAT is supported yet; another edition is not"
    )))
}

/// Bytes at the start of every embedded file that are never scrambled (docs/rbm.md 4).
pub const UNSCRAMBLED: usize = 1024;
/// A jump between neighbouring 16-bit samples larger than this counts as a large jump.
pub const BIG_JUMP: i32 = 16384;
/// A sample file with more than this share of large jumps looks like noise (docs/rbm.md 12.5:
/// 0.0000..0.69 on the 2,000+ good files, among them real noise drums; 0.49..0.61 on every file
/// of a wrongly descrambled mod).
pub const NOISY_SHARE: f64 = 0.3;
/// A mod is judged by its audio only with at least this many measurable sample files.
pub const NOISE_MIN_FILES: usize = 4;
/// At most this many samples of a file are measured.
const NOISE_MEASURED: usize = 100_000;

/// What kind of file an embedded name is, by the rules of [`validate_files`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A slot sample, or a name ending in `.aif` / `.aiff`.
    Aiff,
    /// A name ending in `.jpg`.
    Jpeg,
}

/// The kind of the embedded file `name`; `None` for the files that are not checked (the demo
/// song and the like).
pub fn kind_of(name: &str) -> Option<Kind> {
    let lower = name.to_ascii_lowercase();
    if is_slot_name(name) || lower.ends_with(".aif") || lower.ends_with(".aiff") {
        Some(Kind::Aiff)
    } else if lower.ends_with(".jpg") {
        Some(Kind::Jpeg)
    } else {
        None
    }
}

/// The share of large jumps (more than [`BIG_JUMP`]) between neighbouring samples; `None` for
/// fewer than 64 samples. Real audio has almost none; a file scrambled with a wrong key is
/// uniform noise, whose share is about 0.56.
pub fn jump_share(audio: &[i16]) -> Option<f64> {
    let a = &audio[..audio.len().min(NOISE_MEASURED)];
    if a.len() < 64 {
        return None;
    }
    let big = a
        .windows(2)
        .filter(|w| (i32::from(w[1]) - i32::from(w[0])).abs() > BIG_JUMP)
        .count();
    Some(big as f64 / (a.len() - 1) as f64)
}

/// [`jump_share`] of the big-endian 16-bit words of a decrypted file from byte
/// [`UNSCRAMBLED`] on, the part a wrong key damages.
pub fn file_jump_share(file: &[u8]) -> Option<f64> {
    let words: Vec<i16> = file
        .get(UNSCRAMBLED..)?
        .as_chunks::<2>()
        .0
        .iter()
        .take(NOISE_MEASURED)
        .map(|w| i16::from_be_bytes(*w))
        .collect();
    jump_share(&words)
}

/// Whether the [`jump_share`]s of a mod's sample files say its audio is noise: at least
/// [`NOISE_MIN_FILES`] shares and more than half of them above [`NOISY_SHARE`]. A single file
/// proves nothing (a noise drum is legitimately noisy), a mod does.
pub fn audio_is_noise(shares: &[f64]) -> bool {
    shares.len() >= NOISE_MIN_FILES
        && shares.iter().filter(|&&s| s > NOISY_SHARE).count() * 2 > shares.len()
}

/// [`audio_is_noise`] over the named shares of a mod's sample files; the error names the first
/// few noisy files and their count.
pub fn check_audio(shares: &[(String, f64)]) -> Result<(), RbmError> {
    let values: Vec<f64> = shares.iter().map(|s| s.1).collect();
    if !audio_is_noise(&values) {
        return Ok(());
    }
    let noisy: Vec<&str> = shares
        .iter()
        .filter(|s| s.1 > NOISY_SHARE)
        .map(|s| s.0.as_str())
        .collect();
    let first = noisy.iter().take(3).copied().collect::<Vec<_>>().join(", ");
    let more = if noisy.len() > 3 { ", ..." } else { "" };
    Err(RbmError::WrongKey(format!(
        "{} of its {} sample files are noise ({first}{more})",
        noisy.len(),
        shares.len()
    )))
}

/// Checks one decrypted embedded file (without its name) by what can be known of it alone,
/// the part of [`validate_files`] a rescue can call per file:
/// - an AIFF must parse with the engine's reader;
/// - a JPEG must start with FF D8 and, past byte [`UNSCRAMBLED`], end with FF D9 (trailing zero
///   padding allowed) or at least hold JPEG data ([`jpeg_end_ok`]): a wrong key leaves noise.
///
/// The audio of one file is not checked here (see [`file_jump_share`], [`audio_is_noise`]).
/// The error is [`RbmError::Sample`] with the embedded name, or [`RbmError::WrongKey`] for a JPEG
/// end.
pub fn check_file(name: &str, kind: Kind, file: &[u8]) -> Result<(), RbmError> {
    match kind {
        Kind::Aiff => {
            Sample::from_aiff(file).map_err(|e| RbmError::Sample(name.into(), e.to_string()))?;
        }
        Kind::Jpeg => {
            if !file.starts_with(&[0xFF, 0xD8]) {
                return Err(RbmError::Sample(
                    name.into(),
                    "a JPEG that does not start with FF D8 (a wrong key file, or a damaged mod)"
                        .into(),
                ));
            }
            if !jpeg_end_ok(file) {
                return Err(RbmError::WrongKey(format!(
                    "the JPEG {name} has no end marker FF D9 and its data is not JPEG data"
                )));
            }
        }
    }
    Ok(())
}

/// Whether the part of a JPEG past byte [`UNSCRAMBLED`] can be JPEG data. True if the file ends
/// with FF D9 followed only by zero bytes. A file of [`UNSCRAMBLED`] bytes or fewer is not
/// checked (it is never scrambled).
///
/// A JPEG without the end marker is not necessarily noise: YellowBirth's 59 panel JPEGs were cut
/// short by its packer (docs/rbm.md 11.1). Such a file is accepted as long as every FF byte past
/// byte 1024 is followed by what JPEG data allows (00 stuffing, FF fill, or a marker C0..FE, 01).
/// The set is 66 of 256 byte values, so in uniform noise a FF is followed by a byte outside it
/// 190 times in 256 (about 3 in 4). A noise JPEG of `size` bytes passes with a probability of
/// about exp(-0.0029 * (size - 1024)): 0.80 at 1.1 KB, 0.23 at 1.5 KB, 0.06 at 2 KB, 0.003 at
/// 3 KB, about 0 from 5 KB. Any one failing JPEG refuses the mod.
pub fn jpeg_end_ok(file: &[u8]) -> bool {
    if file.len() <= UNSCRAMBLED {
        return true;
    }
    let end = file.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    if file[..end].ends_with(&[0xFF, 0xD9]) {
        return true;
    }
    file[UNSCRAMBLED..]
        .windows(2)
        .all(|w| w[0] != 0xFF || matches!(w[1], 0x00 | 0x01 | 0xC0..=0xFF))
}

/// Checks the decrypted embedded files of `rbm` under `table`:
/// - each file by [`check_file`] (an AIFF parses, a JPEG starts with FF D8 and ends with FF D9 or
///   holds only JPEG data, see [`jpeg_end_ok`]);
/// - the audio of all the sample files together by [`audio_is_noise`], which finds a mod scrambled
///   with another key file (docs/rbm.md 12.5).
///
/// The first 1024 bytes of every file are never scrambled, so the headers lie where a wrong key
/// cannot show; the checks past byte 1024 are what finds it. Other files are not checked.
pub fn validate_files(rbm: &Rbm, table: &KeyTable) -> Result<(), RbmError> {
    let mut shares = Vec::new();
    for c in rbm.embedded() {
        let name = c.name.clone().unwrap_or_default();
        let Some(kind) = kind_of(&name) else { continue };
        let file = rbm.file(c, table)?;
        check_file(&name, kind, &file)?;
        if kind == Kind::Aiff {
            shares.extend(file_jump_share(&file).map(|s| (name.clone(), s)));
        }
    }
    check_audio(&shares)
}

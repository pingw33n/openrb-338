//! Checks of the key file and of a mod's files before `rb mod recrypt` writes anything (task-064,
//! docs/rbm.md 12.5).
//!
//! A different `RB20FUL.DAT` with the right header gives a silently wrong recrypt: the samples
//! decrypt to garbage, which is scrambled again against the embedded window, and the round trip
//! check of [`crate::recrypt`] passes because it reuses the same wrong key. So the key file is
//! identified by its SHA-256 ([`verify_dat`]) and the decrypted files are checked for what they
//! must be ([`validate_files`]).

use std::io::{Read, Seek, SeekFrom};

use rb_engine::samples::Sample;
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

/// Checks the decrypted embedded files of `rbm` under `table`: a sample (a slot name, or a name
/// ending in `.aif` / `.aiff`) must parse as AIFF with the engine's reader, and a `.jpg` must
/// start with FF D8. The error is [`RbmError::Sample`] with the embedded name.
///
/// This finds a damaged file, not a wrong key file: the first 1024 bytes of every file are never
/// scrambled, and the headers checked here lie there (a sample whose chunks follow the audio is
/// the exception). The key file is identified by [`verify_dat`].
pub fn validate_files(rbm: &Rbm, table: &KeyTable) -> Result<(), RbmError> {
    for c in rbm.embedded() {
        let name = c.name.clone().unwrap_or_default();
        let lower = name.to_ascii_lowercase();
        let is_aiff = is_slot_name(&name) || lower.ends_with(".aif") || lower.ends_with(".aiff");
        let is_jpeg = lower.ends_with(".jpg");
        if !is_aiff && !is_jpeg {
            continue;
        }
        let file = rbm.file(c, table)?;
        if is_aiff {
            Sample::from_aiff(&file).map_err(|e| RbmError::Sample(name.clone(), e.to_string()))?;
        } else if !file.starts_with(&[0xFF, 0xD8]) {
            return Err(RbmError::Sample(
                name,
                "a JPEG that does not start with FF D8 (a wrong key file, or a damaged mod)".into(),
            ));
        }
    }
    Ok(())
}

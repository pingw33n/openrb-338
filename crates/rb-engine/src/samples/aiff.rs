//! The AIFF reader of ReBirth.dll (`CAIFFReader`), for the built-in samples: 16-bit only, channel
//! 0 only, the sample rate ignored.
//!
//! The DLL's path, for reference (docs/initial_investigation.md section 7.4 has the details):
//! the chunk walk 0x10004F94 (chunks padded to even sizes), COMM found by 0x10003170 and its
//! fields read by 0x10001674 (checked by 0x100016F4), SSND read by 0x10002FC4 (its offset and
//! block size by 0x10001774), the mono read and byte swap 0x10009990, the multichannel
//! de-interleave 0x10009DD0.

use super::SampleError;

fn bad(msg: impl Into<String>) -> SampleError {
    SampleError::Aiff(msg.into())
}

/// The padding after the audio: `CWaveSample` (0x1000A9C0) allocates `2 * frames + 1024` bytes
/// and zeroes them, so 512 zero samples follow the data.
pub const PADDING: usize = 512;

/// Whether the DLL sees the sample rate as zero. `sub_10001674` (0x100016B2..0x100016DE) converts
/// the 80-bit rate with `fistp` under a truncating control word (|0xC00) into a 64-bit slot and
/// keeps the low 32 bits; `sub_100016F4` rejects 0. So a rate below 1 Hz (it truncates to 0) is
/// rejected too, and so is one whose integer has zero low 32 bits (2^32), or one that does not
/// fit 63 bits (the integer indefinite 0x8000000000000000, an infinity or a NaN).
fn rate_is_zero(r: &[u8]) -> bool {
    let exp = i32::from(u16::from_be_bytes([r[0], r[1]]) & 0x7FFF);
    let mant = u64::from_be_bytes(r[2..10].try_into().unwrap());
    let e = exp - 16383;
    if e < 0 {
        return true; // |value| < 1 (or the mantissa is zero with a small exponent)
    }
    if e >= 63 {
        return true; // overflow of the 64-bit integer: the indefinite, low dword 0
    }
    (mant >> (63 - e)) as u32 == 0
}

/// Parses an AIFF file into its channel 0 samples, `frames` of them, then [`PADDING`] zeros.
pub fn parse(d: &[u8]) -> Result<Vec<i16>, SampleError> {
    if d.len() < 12 || &d[..4] != b"FORM" {
        return Err(bad("not an AIFF file (no FORM header)"));
    }
    if &d[8..12] != b"AIFF" {
        return Err(bad("not an AIFF file (the FORM type is not AIFF)"));
    }
    let form_size = u32::from_be_bytes(d[4..8].try_into().unwrap()) as usize;
    let end = d.len().min(form_size.saturating_add(8));

    // Chunk walk: id, big-endian size, body, a pad byte after an odd body.
    let mut comm = None;
    let mut ssnd = None;
    let mut p = 12;
    while p + 8 <= end {
        let id = &d[p..p + 4];
        let size = u32::from_be_bytes(d[p + 4..p + 8].try_into().unwrap()) as usize;
        let body_end = (p + 8)
            .checked_add(size)
            .filter(|&e| e <= d.len())
            .ok_or_else(|| {
                bad(format!(
                    "the {} chunk runs past the end of the file",
                    String::from_utf8_lossy(id)
                ))
            })?;
        match id {
            // The COMM fields are read from the file at the chunk's start, whatever its size:
            // the Metallicon mod's AIFFs have a 12-byte COMM (a 4-byte rate field) and the DLL
            // reads the 18 bytes straight through into the SSND header (task-032).
            b"COMM" if comm.is_none() => comm = Some(&d[p + 8..]),
            // Likewise the audio: sub_10002FC4 reads the SSND offset and block size, skips the
            // offset, then reads `2 * frames` bytes from the stream (0x10003060..0x1000306F)
            // with no check against the chunk's size. The 8tRack and DJ Turn's MOD mods have a
            // TR808BD.aif whose FORM and SSND sizes are stale (smaller than the audio that
            // follows), and the DLL plays the whole sample (task-058).
            b"SSND" if ssnd.is_none() => ssnd = Some(&d[p + 8..]),
            _ => {}
        }
        p = body_end + (size & 1);
    }
    let comm = comm.ok_or_else(|| bad("missing COMM chunk"))?;
    let ssnd = ssnd.ok_or_else(|| bad("missing SSND chunk"))?;

    if comm.len() < 18 {
        return Err(bad(
            "the COMM chunk is shorter than 18 bytes and the file ends before 18 bytes of it are available",
        ));
    }
    let channels = u16::from_be_bytes([comm[0], comm[1]]) as usize;
    let frames = u32::from_be_bytes(comm[2..6].try_into().unwrap()) as usize;
    let bits = u16::from_be_bytes([comm[6], comm[7]]);
    // sub_100016F4 rejects a zero channel count, frame count or sample rate, and a depth
    // outside 8..=32. The rate (an 80-bit extended float) is otherwise ignored; it is tested
    // as the integer sub_10001674 makes of it (see `rate_is_zero`).
    if channels == 0 || frames == 0 || !(8..=32).contains(&bits) || rate_is_zero(&comm[8..18]) {
        return Err(bad(
            "the COMM chunk is invalid (zero channels, frames or rate, or a bad depth)",
        ));
    }
    if bits != 16 {
        return Err(bad(format!(
            "unsupported sample size: {bits} bits (only 16-bit is supported)"
        )));
    }

    if ssnd.len() < 8 {
        return Err(bad("the SSND chunk is shorter than 8 bytes"));
    }
    let offset = u32::from_be_bytes(ssnd[..4].try_into().unwrap()) as usize;
    let data = ssnd[8..]
        .get(offset..)
        .ok_or_else(|| bad("the SSND offset is past the end of the file"))?;
    let need = frames
        .checked_mul(channels)
        .and_then(|n| n.checked_mul(2))
        .ok_or_else(|| bad("the frame count overflows"))?;
    if data.len() < need {
        return Err(bad(format!(
            "the file holds {} bytes of audio after the SSND header, the COMM chunk says {need}",
            data.len()
        )));
    }

    let mut out = Vec::with_capacity(frames + PADDING);
    out.extend(
        data[..need]
            .chunks_exact(2 * channels)
            .map(|f| i16::from_be_bytes([f[0], f[1]])),
    );
    out.resize(frames + PADDING, 0);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::rate_is_zero;

    fn ext(exp: u16, mant: u64) -> [u8; 10] {
        let mut r = [0u8; 10];
        r[..2].copy_from_slice(&exp.to_be_bytes());
        r[2..].copy_from_slice(&mant.to_be_bytes());
        r
    }

    /// A mono 16-bit AIFF of `frames` samples whose FORM and SSND sizes claim only `claimed`.
    fn stale_sizes(frames: usize, claimed: usize) -> Vec<u8> {
        let mut d = b"FORM".to_vec();
        d.extend(((4 + 8 + 18 + 8 + 8 + 2 * claimed) as u32).to_be_bytes());
        d.extend(b"AIFFCOMM");
        d.extend(18u32.to_be_bytes());
        d.extend(1u16.to_be_bytes());
        d.extend((frames as u32).to_be_bytes());
        d.extend(16u16.to_be_bytes());
        d.extend([0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
        d.extend(b"SSND");
        d.extend((8 + 2 * claimed as u32).to_be_bytes());
        d.extend([0u8; 8]);
        for i in 0..frames {
            d.extend((i as i16 * 3 - 100).to_be_bytes());
        }
        d
    }

    #[test]
    fn the_audio_is_read_past_a_stale_ssnd_size() {
        // sub_10002FC4 reads 2 * frames bytes after the SSND header whatever the chunk says
        let s = super::parse(&stale_sizes(100, 60)).unwrap();
        assert_eq!(s.len(), 100 + super::PADDING);
        assert_eq!(s[99], 99 * 3 - 100);
        // a file that really ends early is still an error
        let mut d = stale_sizes(100, 60);
        d.truncate(d.len() - 2);
        assert!(super::parse(&d).is_err());
    }

    #[test]
    fn rates_the_dll_sees_as_zero() {
        assert!(rate_is_zero(&[0; 10]));
        // 44100 Hz
        assert!(!rate_is_zero(&[0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]));
        // 1 Hz is the smallest accepted
        assert!(!rate_is_zero(&ext(0x3FFF, 1 << 63)));
        // 0.5 Hz: non-zero bytes, truncates to 0 (the DLL rejects it)
        assert!(rate_is_zero(&ext(0x3FFE, 1 << 63)));
        // 1.999 Hz truncates to 1
        assert!(!rate_is_zero(&ext(0x3FFF, 0xFFC0_0000_0000_0000)));
        // 2^32 has zero low 32 bits
        assert!(rate_is_zero(&ext(0x3FFF + 32, 1 << 63)));
        // infinity: the integer indefinite
        assert!(rate_is_zero(&ext(0x7FFF, 1 << 63)));
    }
}

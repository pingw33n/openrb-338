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
            b"SSND" if ssnd.is_none() => ssnd = Some(&d[p + 8..body_end]),
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
    // outside 8..=32. The rate (an 80-bit extended float) is otherwise ignored, so it is not
    // decoded: only its zero-ness (exponent and mantissa) is checked.
    if channels == 0
        || frames == 0
        || !(8..=32).contains(&bits)
        || comm[8..18].iter().all(|&b| b == 0)
    {
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
        .ok_or_else(|| bad("the SSND offset is past the end of the chunk"))?;
    let need = frames
        .checked_mul(channels)
        .and_then(|n| n.checked_mul(2))
        .ok_or_else(|| bad("the frame count overflows"))?;
    if data.len() < need {
        return Err(bad(format!(
            "the SSND chunk holds {} bytes of audio, the COMM chunk says {need}",
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

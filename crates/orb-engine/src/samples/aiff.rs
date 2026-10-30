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

/// One search of the chunk walk `sub_10004F94` for `id`: the offset of the first chunk header
/// with that id, or `None` when the walk reaches the FORM's end (`form_end`, the FORM size + 8,
/// 32-bit) first.
///
/// The DLL reads the first header at 12 unchecked (0x10005071), then on a mismatch moves to
/// `pos + size + 8`, +1 after an odd size (0x100050B1..0x100050CA), all in wrapping 32-bit
/// arithmetic, and stops unfound when `pos + 8 >= form_end` as signed integers
/// (0x100050CD..0x100050D6, the `jge` at 0x100050D6). No chunk size is compared with the stream,
/// and the memory stream (vtable 0x10030824) has no bounds check, so a header the DLL would read
/// outside the file is an out-of-bounds read there; orb rejects the file instead. A walk that comes
/// back to a position (a size of -8, or -9 with the pad) never ends in the DLL; orb rejects that
/// too.
fn find(d: &[u8], id: &[u8; 4], form_end: u32) -> Result<Option<usize>, SampleError> {
    let mut pos: u32 = 12;
    // Every header read lies in the file, so more steps than the file has bytes repeat one.
    for _ in 0..d.len() {
        let p = pos as usize;
        let h = d.get(p..p.saturating_add(8)).ok_or_else(|| {
            bad(format!(
                "the {} chunk search reads a chunk header outside the file (at {pos:#x})",
                String::from_utf8_lossy(id)
            ))
        })?;
        if &h[..4] == id {
            return Ok(Some(p));
        }
        let size = u32::from_be_bytes(h[4..8].try_into().unwrap());
        pos = pos
            .wrapping_add(size)
            .wrapping_add(8)
            .wrapping_add(size & 1);
        if pos.wrapping_add(8) as i32 >= form_end as i32 {
            return Ok(None);
        }
    }
    Err(bad(format!(
        "the {} chunk search never ends (a chunk size sends it back)",
        String::from_utf8_lossy(id)
    )))
}

/// Parses an AIFF file into its channel 0 samples, `frames` of them, then [`PADDING`] zeros.
pub fn parse(d: &[u8]) -> Result<Vec<i16>, SampleError> {
    // sub_100054D4 accepts the header when its id is "FORM" and its size is at least 4
    // (unsigned, 0x1000550C); the FORM type is decoded but never compared (task-155).
    if d.len() < 12 || &d[..4] != b"FORM" {
        return Err(bad("not an AIFF file (no FORM header)"));
    }
    let form_size = u32::from_be_bytes(d[4..8].try_into().unwrap());
    if form_size < 4 {
        return Err(bad("not an AIFF file (the FORM size is below 4)"));
    }
    let form_end = form_size.wrapping_add(8);

    // COMM and SSND are each found by their own walk from the file's start, which stops at the
    // first chunk with the id, so nothing after it is read: junk after SSND is ignored. The
    // COMM fields are read from the file at the chunk's start, whatever its size: the
    // Metallicon mod's AIFFs have a 12-byte COMM (a 4-byte rate field) and the DLL reads the 18
    // bytes straight through into the SSND header (task-032).
    let comm = &d[find(d, b"COMM", form_end)?.ok_or_else(|| bad("missing COMM chunk"))? + 8..];
    // Likewise the audio: sub_10002FC4 reads the SSND offset and block size, skips the offset,
    // then reads `2 * frames` bytes from the stream (0x10003060..0x1000306F) with no check
    // against the chunk's size. The 8tRack and DJ Turn's MOD mods have a TR808BD.aif whose FORM
    // and SSND sizes are stale (smaller than the audio that follows), and the DLL plays the
    // whole sample (task-058); Black_Steel.rbm and others have an SSND size 8 short, then 8
    // bytes the walk would take for a chunk header (task-155).
    let ssnd = &d[find(d, b"SSND", form_end)?.ok_or_else(|| bad("missing SSND chunk"))? + 8..];

    if comm.len() < 18 {
        return Err(bad(
            "the COMM chunk is shorter than 18 bytes and the file ends before 18 bytes of it \
             are available",
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
    // The offset is skipped only when it is positive as a signed 32-bit value
    // (0x1000302D..0x10003033, `test eax,eax; jle`).
    let offset = i32::from_be_bytes(ssnd[..4].try_into().unwrap()).max(0) as usize;
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

    /// The COMM body of a mono 16-bit 44100 Hz file of `frames` frames.
    fn comm(frames: u32) -> Vec<u8> {
        let mut c = 1u16.to_be_bytes().to_vec();
        c.extend(frames.to_be_bytes());
        c.extend(16u16.to_be_bytes());
        c.extend([0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
        c
    }

    /// The SSND body (offset 0, block size 0) holding `frames` samples 1, 2, 3...
    fn ssnd(frames: i16) -> Vec<u8> {
        let mut s = vec![0u8; 8];
        for i in 1..=frames {
            s.extend(i.to_be_bytes());
        }
        s
    }

    /// FORM AIFF with `chunks` (id, declared size, body, a pad byte after an odd body), then
    /// `tail`; the FORM size covers the whole file.
    fn file(chunks: &[(&[u8; 4], u32, &[u8])], tail: &[u8]) -> Vec<u8> {
        let mut d = b"FORM\0\0\0\0AIFF".to_vec();
        for (id, size, body) in chunks {
            d.extend(*id);
            d.extend(size.to_be_bytes());
            d.extend(*body);
            if body.len() % 2 == 1 {
                d.push(0);
            }
        }
        d.extend(tail);
        let n = (d.len() - 8) as u32;
        d[4..8].copy_from_slice(&n.to_be_bytes());
        d
    }

    #[test]
    fn junk_after_ssnd_is_never_read() {
        // Black_Steel.rbm, Black_Steel_2.rbm, O.S.C.rbm: the SSND size leaves out the 8-byte
        // header, so the audio ends 8 bytes past it and those 8 bytes (audio) would be taken for
        // a chunk header with a huge size. The walk stops at SSND and never reads them.
        let (c, s) = (comm(10), ssnd(10));
        let short = (s.len() - 8) as u32;
        let d = file(&[(b"COMM", 18, &c), (b"SSND", short, &s)], &[]);
        let tail = &d[d.len() - 8..];
        assert!(u32::from_be_bytes(tail[4..8].try_into().unwrap()) as usize > d.len());
        let out = super::parse(&d).unwrap();
        assert_eq!(&out[..10], &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);

        // mellow.rbm: COMM, INST, a correct SSND, then 20 stray bytes inside the FORM.
        let inst = [0u8; 20];
        let stray = [0xA5u8; 20];
        let d = file(
            &[
                (b"COMM", 18, &c),
                (b"INST", 20, &inst),
                (b"SSND", s.len() as u32, &s),
            ],
            &stray,
        );
        let out = super::parse(&d).unwrap();
        assert_eq!(&out[..10], &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert!(out[10..].iter().all(|&x| x == 0));

        // SSND before COMM, junk last: each search starts from the file's start.
        let d = file(&[(b"SSND", s.len() as u32, &s), (b"COMM", 18, &c)], &stray);
        assert_eq!(&super::parse(&d).unwrap()[..2], &[1, 2]);
    }

    #[test]
    fn a_junk_chunk_before_ssnd() {
        let (c, s) = (comm(4), ssnd(4));
        // A chunk whose size runs past the FORM's end: the walk gives up, SSND is missing
        // (the DLL throws at 0x10003018).
        let d = file(
            &[
                (b"COMM", 18, &c),
                (b"JUNK", 0x7000_0000, &[]),
                (b"SSND", 16, &s),
            ],
            &[],
        );
        assert!(
            super::parse(&d)
                .unwrap_err()
                .to_string()
                .contains("missing SSND")
        );
        // COMM is still found when the junk follows it, and missing when the junk precedes it.
        let d = file(
            &[
                (b"JUNK", 0x7000_0000, &[]),
                (b"COMM", 18, &c),
                (b"SSND", 16, &s),
            ],
            &[],
        );
        assert!(
            super::parse(&d)
                .unwrap_err()
                .to_string()
                .contains("missing COMM")
        );

        // A size that wraps the position below 0: pos + 8 is negative, so the DLL goes on and
        // reads before the buffer; orb rejects the file.
        let d = file(
            &[
                (b"COMM", 18, &c),
                (b"JUNK", 0xFFFF_0000, &[]),
                (b"SSND", 16, &s),
            ],
            &[],
        );
        assert!(
            super::parse(&d)
                .unwrap_err()
                .to_string()
                .contains("outside the file")
        );

        // A FORM size past the end of the file lets the walk read past it (out of bounds in
        // the DLL); orb rejects the file.
        let mut d = file(
            &[(b"COMM", 18, &c), (b"JUNK", 100, &[]), (b"SSND", 16, &s)],
            &[],
        );
        d[4..8].copy_from_slice(&1000u32.to_be_bytes());
        assert!(
            super::parse(&d)
                .unwrap_err()
                .to_string()
                .contains("outside the file")
        );

        // A size of -8 leaves the position where it is: the DLL loops forever.
        let d = file(
            &[
                (b"COMM", 18, &c),
                (b"JUNK", 0xFFFF_FFF8, &[]),
                (b"SSND", 16, &s),
            ],
            &[],
        );
        assert!(
            super::parse(&d)
                .unwrap_err()
                .to_string()
                .contains("never ends")
        );

        // A damaged chunk whose size lands on SSND's header (here an odd size, padded): found.
        let d = file(
            &[(b"COMM", 18, &c), (b"JUNK", 7, &[0; 8]), (b"SSND", 16, &s)],
            &[],
        );
        assert_eq!(&super::parse(&d).unwrap()[..4], &[1, 2, 3, 4]);
    }

    #[test]
    fn the_walk_bounds_and_the_form_header() {
        let (c, s) = (comm(1), ssnd(1));
        // A chunk header is read only when it ends before the FORM's end (signed `jge`): an
        // SSND of size 0 whose header ends exactly at the FORM's end is not found.
        let mut d = file(&[(b"COMM", 18, &c), (b"SSND", 0, &[])], &[]);
        assert!(
            super::parse(&d)
                .unwrap_err()
                .to_string()
                .contains("missing SSND")
        );
        // The first header (at 12) is read whatever the FORM size: a FORM of size 4 still
        // finds a COMM there (and then misses SSND, which the bound excludes).
        d[4..8].copy_from_slice(&4u32.to_be_bytes());
        assert!(
            super::parse(&d)
                .unwrap_err()
                .to_string()
                .contains("missing SSND")
        );
        d[4..8].copy_from_slice(&3u32.to_be_bytes());
        assert!(
            super::parse(&d)
                .unwrap_err()
                .to_string()
                .contains("FORM size")
        );

        // The FORM type is never compared (sub_100054D4): an AIFC form type is read the same.
        let mut d = file(&[(b"COMM", 18, &c), (b"SSND", 10, &s)], &[]);
        d[8..12].copy_from_slice(b"AIFC");
        assert_eq!(super::parse(&d).unwrap()[0], 1);

        // A negative SSND offset is not skipped (0x10003033 `jle`).
        let mut neg = s.clone();
        neg[..4].copy_from_slice(&(-4i32).to_be_bytes());
        let d = file(&[(b"COMM", 18, &c), (b"SSND", 10, &neg)], &[]);
        assert_eq!(super::parse(&d).unwrap()[0], 1);
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

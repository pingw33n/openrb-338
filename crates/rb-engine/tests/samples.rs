//! The built-in sample loader (task-020): all 46 samples from the original ReBirth.dll, and the
//! parsers' errors.

use rb_engine::samples::{self, PADDING, Sample, SampleError, SampleSet};
use std::path::Path;

fn original(name: &str) -> Vec<u8> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../original")
        .join(name);
    std::fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

/// (resource ID, frames, sum of all audio samples) of each resource 1000..=1045, read
/// independently of the Rust code: a Python script with its own PE and AIFF reader printed the
/// COMM chunk's numSampleFrames and the sum of the SSND samples (big-endian i16, after the SSND
/// offset) of every resource. All are mono and 16-bit; 1034..=1036 have an SSND offset of 458.
#[rustfmt::skip]
const SAMPLES: [(u16, usize, i64); 46] = [
    (1000, 72704, 4473395), (1001, 8800, 543415), (1002, 8800, 545114), (1003, 8800, 543971),
    (1004, 8800, 543989), (1005, 8800, 542146), (1006, 8800, -5500), (1007, 8800, -6356),
    (1008, 8800, -6324), (1009, 8800, -7475), (1010, 51200, 3177403), (1011, 4992, 298558),
    (1012, 1816, 114154), (1013, 87040, 5228929), (1014, 174430, -114017), (1015, 11680, 704229),
    (1016, 17498, 1068951), (1017, 17056, 1050295), (1018, 22063, 1374373), (1019, 1468, 89107),
    (1020, 11603, 704280), (1021, 15351, 937893), (1022, 26578, 1674874), (1023, 690, 41780),
    (1024, 84575, 24686642), (1025, 22062, -241302), (1026, 22001, -275865),
    (1027, 22036, -246560), (1028, 22099, -247778), (1029, 14007, -327634),
    (1030, 14008, -326032), (1031, 14006, -327225), (1032, 14014, -322835),
    (1033, 14034, -324613), (1034, 12180, -81455), (1035, 12180, -29711), (1036, 12180, -28172),
    (1037, 28072, -163237), (1038, 20427, -157029), (1039, 20862, -275431),
    (1040, 5006, 206630), (1041, 30365, -19732), (1042, 8401, -2781), (1043, 25811, -12020),
    (1044, 43690, 248610), (1045, 42458, -54780),
];

#[test]
fn all_46_samples_load_with_the_expected_frame_counts() {
    let set = SampleSet::from_dll(&original("ReBirth.dll")).unwrap();
    assert_eq!(samples::COUNT, 46);
    for &(id, frames, sum) in &SAMPLES {
        let s = set.get(id);
        assert_eq!(s.frames(), frames, "resource {id}");
        assert_eq!(
            s.audio().iter().map(|&v| v as i64).sum::<i64>(),
            sum,
            "resource {id}"
        );
        assert_eq!(s.audio().len(), frames);
        assert_eq!(s.padded().len(), frames + 512, "resource {id}");
        assert!(
            s.padded()[frames..].iter().all(|&v| v == 0),
            "resource {id}"
        );
        assert!(s.audio().iter().any(|&v| v != 0), "resource {id} is silent");
    }
    assert_eq!(PADDING, 512);
    assert_eq!(set.tr808(0).frames(), 72704); // BD
    assert_eq!(set.tr808(23).frames(), 690); // RS
    assert_eq!(set.tr909(0).frames(), 22062); // bd1
    assert_eq!(set.tr909(20).frames(), 42458); // rcy
    assert_eq!(samples::NAMES_808.len() + samples::NAMES_909.len() + 1, 46);
}

#[test]
fn the_ssnd_offset_is_skipped() {
    // Resources 1034..=1036 have an SSND offset of 458 bytes; the first samples are the ones
    // the independent reader found after the offset.
    let set = SampleSet::from_dll(&original("ReBirth.dll")).unwrap();
    assert_eq!(&set.get(1034).audio()[..3], &[709, 1258, 412]);
    assert_eq!(&set.get(1035).audio()[..3], &[501, 398, -556]);
    assert_eq!(&set.get(1036).audio()[..3], &[181, 399, 354]);
}

/// A minimal AIFF with the given COMM fields and SSND body (extended rate 44100).
fn aiff(channels: u16, frames: u32, bits: u16, ssnd_offset: u32, audio: &[u8]) -> Vec<u8> {
    let mut comm = Vec::new();
    comm.extend(channels.to_be_bytes());
    comm.extend(frames.to_be_bytes());
    comm.extend(bits.to_be_bytes());
    comm.extend([0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]);
    let mut ssnd = Vec::new();
    ssnd.extend(ssnd_offset.to_be_bytes());
    ssnd.extend(0u32.to_be_bytes());
    ssnd.extend(vec![0xEE; ssnd_offset as usize]);
    ssnd.extend(audio);
    let mut body = b"AIFF".to_vec();
    for (id, c) in [(b"COMM", &comm), (b"SSND", &ssnd)] {
        body.extend(id);
        body.extend((c.len() as u32).to_be_bytes());
        body.extend(c.iter());
        if c.len() % 2 == 1 {
            body.push(0);
        }
    }
    let mut out = b"FORM".to_vec();
    out.extend((body.len() as u32).to_be_bytes());
    out.extend(body);
    out
}

fn err(r: Result<Sample, SampleError>) -> String {
    r.unwrap_err().to_string()
}

#[test]
fn aiff_mono_stereo_and_offset() {
    let mono = aiff(1, 3, 16, 0, &[0x01, 0x02, 0xFF, 0xFE, 0x80, 0x00]);
    let s = Sample::from_aiff(&mono).unwrap();
    assert_eq!(s.audio(), &[0x0102, -2, i16::MIN]);
    assert_eq!(s.padded().len(), 3 + 512);

    // Stereo: channel 0 only.
    let st = aiff(2, 2, 16, 0, &[0, 1, 0xFF, 0xFF, 0, 2, 0xFF, 0xFF]);
    assert_eq!(Sample::from_aiff(&st).unwrap().audio(), &[1, 2]);

    // An SSND offset skips that many bytes of the data.
    let off = aiff(1, 2, 16, 4, &[0, 7, 0, 8]);
    assert_eq!(Sample::from_aiff(&off).unwrap().audio(), &[7, 8]);
}

#[test]
fn a_short_comm_chunk_reads_through_into_ssnd() {
    // The Metallicon mod's AIFFs: COMM of 12 bytes (a 4-byte rate field), SSND right after.
    // The DLL reads the 18 bytes from the chunk start whatever its size (sub_10001674).
    let full = aiff(1, 2, 16, 0, &[0, 5, 0, 6]);
    let mut d = full[..12 + 8 + 12].to_vec(); // FORM header, COMM header, 12 body bytes
    d[12 + 4..12 + 8].copy_from_slice(&12u32.to_be_bytes());
    d.extend(&full[12 + 8 + 18..]); // SSND chunk
    let n = (d.len() - 8) as u32;
    d[4..8].copy_from_slice(&n.to_be_bytes());
    let s = Sample::from_aiff(&d).unwrap();
    assert_eq!(s.audio(), &[5, 6]);

    // SSND first, then a short COMM that is the last chunk: the COMM read runs off the end of
    // the file and is rejected for that reason.
    let mut end = full[..12].to_vec();
    end.extend(&full[12 + 8 + 18..]); // SSND
    end.extend(&full[12..12 + 8 + 12]); // COMM header and 12 body bytes
    let at = end.len() - 12 - 4;
    end[at..at + 4].copy_from_slice(&12u32.to_be_bytes());
    let n = (end.len() - 8) as u32;
    end[4..8].copy_from_slice(&n.to_be_bytes());
    let e = err(Sample::from_aiff(&end));
    assert!(e.contains("COMM chunk is shorter than 18 bytes"), "{e}");
}

#[test]
fn aiff_errors() {
    let e = err(Sample::from_aiff(&aiff(1, 2, 8, 0, &[1, 2])));
    assert!(e.contains("8 bits") && e.contains("16-bit"), "{e}");
    let e = err(Sample::from_aiff(&aiff(1, 1, 24, 0, &[1, 2, 3])));
    assert!(e.contains("24 bits"), "{e}");

    // Missing chunks: cut the FORM body down to one chunk.
    let full = aiff(1, 1, 16, 0, &[0, 1]);
    let comm_end = 12 + 8 + 18;
    let mut only_comm = full[..comm_end].to_vec();
    only_comm[4..8].copy_from_slice(&((comm_end - 8) as u32).to_be_bytes());
    assert!(err(Sample::from_aiff(&only_comm)).contains("missing SSND"));
    let mut only_ssnd = full[..12].to_vec();
    only_ssnd.extend(&full[comm_end..]);
    let n = (only_ssnd.len() - 8) as u32;
    only_ssnd[4..8].copy_from_slice(&n.to_be_bytes());
    assert!(err(Sample::from_aiff(&only_ssnd)).contains("missing COMM"));

    assert!(err(Sample::from_aiff(b"RIFF....WAVE")).contains("no FORM"));
    assert!(err(Sample::from_aiff(b"FORM\0\0\0\x04AIFC")).contains("not AIFF"));
    assert!(err(Sample::from_aiff(b"")).contains("no FORM"));

    // Short audio: the COMM chunk says 4 frames, the SSND holds 1.
    let e = err(Sample::from_aiff(&aiff(1, 4, 16, 0, &[0, 1])));
    assert!(e.contains("SSND"), "{e}");
    // A chunk size past the end of the file.
    let mut cut = aiff(1, 1, 16, 0, &[0, 1]);
    cut.truncate(cut.len() - 1);
    assert!(Sample::from_aiff(&cut).is_err());
    // Zero channels.
    assert!(err(Sample::from_aiff(&aiff(0, 1, 16, 0, &[0, 1]))).contains("COMM"));
}

#[test]
fn pe_errors() {
    let e = SampleSet::from_dll(b"this is not a PE file, just text").unwrap_err();
    assert!(matches!(e, SampleError::Pe(_)), "{e:?}");
    assert!(e.to_string().contains("not a PE"), "{e}");
    assert!(matches!(SampleSet::from_dll(b""), Err(SampleError::Pe(_))));
    // MZ with a PE offset past the end.
    let mut mz = vec![0u8; 0x40];
    mz[..2].copy_from_slice(b"MZ");
    mz[0x3C] = 0xF0;
    assert!(matches!(SampleSet::from_dll(&mz), Err(SampleError::Pe(_))));

    // A truncated real DLL: the resource directory runs past the end, an error and no panic.
    let dll = original("ReBirth.dll");
    for cut in [0x200, 0x1000, 0x20000, dll.len() * 3 / 4] {
        assert!(SampleSet::from_dll(&dll[..cut]).is_err(), "cut at {cut}");
    }
}

#[test]
fn a_pe_without_aiff_resources_is_rejected() {
    // Rebirth.exe is a real PE with resources, but no "AIFF" type.
    let e = SampleSet::from_dll(&original("Rebirth.exe")).unwrap_err();
    assert_eq!(e, SampleError::NoAiffResources, "{e}");
}

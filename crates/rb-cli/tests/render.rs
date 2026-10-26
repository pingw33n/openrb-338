//! `rb render` end to end (task-028): all three formats, FLAC against WAV, the WAV against the
//! reference manifest, and the error exits.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rb(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rb"))
        .args(args)
        .output()
        .unwrap()
}

fn render(input: &Path, output: &Path, extra: &[&str]) -> Output {
    let mut a: Vec<&std::ffi::OsStr> = vec!["render".as_ref(), input.as_ref(), "-o".as_ref()];
    a.push(output.as_ref());
    a.extend(extra.iter().map(|s| std::ffi::OsStr::new(*s)));
    rb(&a)
}

/// The int16 samples of a 44-byte-header WAV.
fn wav_samples(wav: &[u8]) -> Vec<i16> {
    wav[44..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&b| i16::from_le_bytes(b))
        .collect()
}

#[test]
fn all_formats_agree() {
    let song = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let dir = tempfile::tempdir().unwrap();
    let (wav, flac, ogg) = (
        dir.path().join("a.wav"),
        dir.path().join("a.flac"),
        dir.path().join("a.ogg"),
    );
    for out in [&wav, &flac, &ogg] {
        let o = render(&song, out, &[]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
    let wav_bytes = std::fs::read(&wav).unwrap();
    let want = wav_samples(&wav_bytes);
    assert!(want.iter().any(|&s| s != 0), "the render is silent");

    let mut r = claxon::FlacReader::open(&flac).unwrap();
    let info = r.streaminfo();
    assert_eq!(
        (info.channels, info.sample_rate, info.bits_per_sample),
        (2, 44100, 16)
    );
    let got: Vec<i16> = r.samples().map(|s| s.unwrap() as i16).collect();
    assert_eq!(got, want, "FLAC differs from WAV");

    let ogg_bytes = std::fs::read(&ogg).unwrap();
    assert_eq!(&ogg_bytes[..4], b"OggS");
    let mut dec = vorbis_rs::VorbisDecoder::new(std::io::Cursor::new(ogg_bytes)).unwrap();
    assert_eq!(dec.channels().get(), 2);
    assert_eq!(dec.sampling_frequency().get(), 44100);
    let mut frames = 0;
    while let Some(b) = dec.decode_audio_block().unwrap() {
        frames += b.samples()[0].len();
    }
    assert_eq!(frames, want.len() / 2);
}

#[test]
fn silence_wav_matches_manifest() {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root().join("tests/refs/manifest.json")).unwrap())
            .unwrap();
    let entry = &manifest["diag"]["silence"];
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("s.wav");
    let o = render(&root().join(entry["song"].as_str().unwrap()), &wav, &[]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let hash = Sha256::digest(std::fs::read(&wav).unwrap());
    let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, entry["wav_sha256"].as_str().unwrap());
}

#[test]
fn range_changes_the_length() {
    let song = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("r.wav");
    let frames = |range: &str| {
        let o = render(&song, &wav, &["--range", range, "--idle-blocks", "0"]);
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        wav_samples(&std::fs::read(&wav).unwrap()).len() / 2
    };
    // one bar at 120 BPM is 2 s; the export's chunk arithmetic (ftol truncation, export::chunk_sizes) gives 88199
    assert_eq!(frames("0:1"), 88199);
    assert_eq!(frames("0:2"), 176400);
}

#[test]
fn pattern_mode_warns() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("p.wav");
    let song = root().join("tests/songs/shipped/Silent Default Song.rbs");
    let o = render(&song, &wav, &[]);
    assert!(o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("pattern mode"));
    let song = root().join("tests/songs/diag/songs/303_saw_lowcut.rbs");
    let o = render(&song, &wav, &[]);
    assert!(!String::from_utf8_lossy(&o.stderr).contains("pattern mode"));
}

#[test]
fn errors_have_exit_codes_and_no_panic() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("x.wav");
    let song = root().join("tests/songs/diag/songs/silence.rbs");

    let o = render(&dir.path().join("missing.rbs"), &out, &[]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("cannot read"));

    let junk = dir.path().join("junk.rbs");
    std::fs::write(&junk, b"not a song").unwrap();
    let o = render(&junk, &out, &[]);
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("not a valid"));

    let o = render(&song, &dir.path().join("x.mp3"), &[]);
    assert_eq!(o.status.code(), Some(2));
    let o = render(&song, &out, &["--range", "4:2"]);
    assert_eq!(o.status.code(), Some(2));
    let o = render(&song, &out, &["--range", "abc"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(!out.exists());
}

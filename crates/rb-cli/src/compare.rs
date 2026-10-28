//! `rb compare`: renders songs and compares each with its reference by the pass criterion of
//! task-013, a byte-exact WAV (header included), reporting per song EQUAL or the first
//! difference (frame, time, bar, channel and both values; `rb_engine::compare`).
//!
//! Two forms:
//! - `rb compare SONG.rbs --reference REF.flac|REF.wav`: one song against one reference, at
//!   `--idle-blocks` (default 9088).
//! - `rb compare --manifest tests/refs/manifest.json [--set diag,app,shipped] [NAME...]`: the
//!   entries of a corpus manifest (all of the sets' entries when no NAME is given; a NAME is an
//!   entry name or `set/name`), each at its own `idle_blocks` unless `--idle-blocks` is given. Song
//!   paths are relative to the repository root (the manifest's directory's grandparent), FLAC paths
//!   to the manifest's directory. Shipped entries have no audio committed, so a difference there is
//!   reported by the sha256 and the frame count only.

use std::path::{Path, PathBuf};

use rb_engine::compare::{describe, first_difference};
use rb_engine::frame::export::{self, ExportRange};
use rb_engine::song::Song;
use serde_json::Value;
use sha2::{Digest, Sha256};

use rb_cli::mods::ModOpts;
use rb_cli::render::{Renderer, load_song, warn_unsupported};
use rb_cli::{Fail, fail};

/// The default idle-block count of the single-song form, the corpus' capture protocol.
const DEFAULT_IDLE: u32 = 9088;

/// The manifest's sets, in the order they are compared.
const SETS: &[&str] = &["diag", "app", "shipped", "mod", "modapp"];

/// The sets compared when `--set` is not given: the last two (the mod songs) only when a mod
/// folder was asked for with `--mod-dir` or `RB_MOD_DIR`, as they need the .rbm files.
fn default_sets(mods: &ModOpts) -> Vec<&'static str> {
    let n = if mods.mod_dir_given() { SETS.len() } else { 3 };
    SETS[..n].to_vec()
}

pub struct Args {
    pub names: Vec<String>,
    pub reference: Option<PathBuf>,
    pub manifest: Option<PathBuf>,
    pub sets: Vec<String>,
    pub idle_blocks: Option<u32>,
    pub mods: ModOpts,
}

/// Runs the comparison: `Ok(true)` when every song is equal to its reference.
pub fn run(a: &Args) -> Result<bool, Fail> {
    let mut r = Renderer::new(a.mods.clone());
    match (&a.manifest, &a.reference) {
        (Some(m), None) => manifest(a, m, &mut r),
        (None, Some(reference)) => {
            let [song] = a.names.as_slice() else {
                return Err(fail(2, "--reference needs exactly one song"));
            };
            let song = Path::new(song);
            let idle = a.idle_blocks.unwrap_or(DEFAULT_IDLE);
            let s = load_song(song)?;
            warn_unsupported(&s);
            let label = song.display().to_string();
            let got = r.render(&label, &s, idle)?;
            let outcome = against_file(&got, &s, reference)?;
            Ok(report(&label, idle, outcome))
        }
        (Some(_), Some(_)) => Err(fail(2, "give either --manifest or --reference, not both")),
        (None, None) => Err(fail(
            2,
            "nothing to compare: give SONG --reference REF, or --manifest PATH [NAME...]",
        )),
    }
}

/// Prints the outcome of one song; returns whether it is equal.
fn report(label: &str, idle: u32, outcome: Option<String>) -> bool {
    match outcome {
        None => {
            println!("{label}: EQUAL");
            true
        }
        Some(d) => {
            println!("{label}: DIFFERENT (N {idle}): {d}");
            false
        }
    }
}

fn sha256(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The interleaved int16 samples of a 16-bit stereo 44100 Hz FLAC.
fn flac_samples(p: &Path) -> Result<Vec<i16>, Fail> {
    let bad = |e: String| fail(1, format!("{}: cannot read the FLAC: {e}", p.display()));
    let mut r = claxon::FlacReader::open(p).map_err(|e| bad(e.to_string()))?;
    let i = r.streaminfo();
    if (i.channels, i.sample_rate, i.bits_per_sample) != (2, 44100, 16) {
        return Err(bad(format!(
            "{} channels, {} Hz, {} bits; expected 2, 44100, 16",
            i.channels, i.sample_rate, i.bits_per_sample
        )));
    }
    r.samples()
        .map(|s| s.map(|v| v as i16).map_err(|e| bad(e.to_string())))
        .collect()
}

/// The interleaved int16 samples of a PCM WAV's `data` chunk.
fn wav_samples(p: &Path, b: &[u8]) -> Result<Vec<i16>, Fail> {
    let bad = |m: &str| fail(1, format!("{}: not a 16-bit PCM WAV: {m}", p.display()));
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return Err(bad("no RIFF/WAVE header"));
    }
    let mut at = 12;
    while at + 8 <= b.len() {
        let len = u32::from_le_bytes(b[at + 4..at + 8].try_into().unwrap()) as usize;
        let body = &b[at + 8..(at + 8 + len).min(b.len())];
        if &b[at..at + 4] == b"data" {
            return Ok(body
                .as_chunks::<2>()
                .0
                .iter()
                .map(|&s| i16::from_le_bytes(s))
                .collect());
        }
        at += 8 + len + (len & 1);
    }
    Err(bad("no data chunk"))
}

/// The render `got` of `song` against a reference file (.flac or .wav): `None` when equal.
fn against_file(got: &[i16], song: &Song, reference: &Path) -> Result<Option<String>, Fail> {
    let range = ExportRange::from(&song.globals);
    let ext = reference
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let want = match ext.as_deref() {
        Some("flac") => flac_samples(reference)?,
        Some("wav") => {
            let b = std::fs::read(reference)
                .map_err(|e| fail(1, format!("{}: cannot read: {e}", reference.display())))?;
            if b == export::wav_bytes(got) {
                return Ok(None);
            }
            let want = wav_samples(reference, &b)?;
            if want == got {
                return Ok(Some(
                    "the samples are equal but the WAV header differs from rb's 44-byte one".into(),
                ));
            }
            want
        }
        _ => {
            return Err(fail(
                2,
                format!(
                    "{}: the reference must be a .flac or a .wav",
                    reference.display()
                ),
            ));
        }
    };
    // A FLAC holds the samples; the WAV rb writes for them is then byte-exact iff they are equal
    // (its header depends on the frame count only).
    Ok(first_difference(got, &want).map(|d| describe(d, range)))
}

/// The manifest form.
fn manifest(a: &Args, path: &Path, r: &mut Renderer) -> Result<bool, Fail> {
    let data = std::fs::read(path)
        .map_err(|e| fail(1, format!("{}: cannot read: {e}", path.display())))?;
    let m: Value = serde_json::from_slice(&data)
        .map_err(|e| fail(1, format!("{}: not JSON: {e}", path.display())))?;
    let refs = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let root = refs.join("../..");
    let sets: Vec<&str> = if a.sets.is_empty() {
        default_sets(&a.mods)
    } else {
        for s in &a.sets {
            if !SETS.contains(&s.as_str()) {
                return Err(fail(
                    2,
                    format!("--set {s}: expected one of {}", SETS.join(", ")),
                ));
            }
        }
        a.sets.iter().map(String::as_str).collect()
    };
    let mut entries: Vec<(String, &Value)> = Vec::new();
    for set in &sets {
        let Some(es) = m[*set].as_object() else {
            continue;
        };
        for (name, e) in es {
            entries.push((format!("{set}/{name}"), e));
        }
    }
    if !a.names.is_empty() {
        for n in &a.names {
            if !entries
                .iter()
                .any(|(l, _)| l == n || l.split_once('/').unwrap().1 == n)
            {
                return Err(fail(2, format!("{n}: no such entry in the sets {sets:?}")));
            }
        }
        entries.retain(|(l, _)| {
            a.names
                .iter()
                .any(|n| l == n || l.split_once('/').unwrap().1 == n)
        });
    }
    let field = |l: &str, e: &Value, k: &str| -> Result<String, Fail> {
        e[k].as_str()
            .map(str::to_string)
            .ok_or_else(|| fail(1, format!("{}: {l} has no {k:?}", path.display())))
    };
    let mut equal = 0;
    for (label, e) in &entries {
        let song = root.join(field(label, e, "song")?);
        let idle = match a.idle_blocks {
            Some(n) => n,
            None => e["idle_blocks"]
                .as_u64()
                .and_then(|n| u32::try_from(n).ok())
                .ok_or_else(|| {
                    fail(1, format!("{}: {label} has no idle_blocks", path.display()))
                })?,
        };
        let s = load_song(&song)?;
        let got = r.render(label, &s, idle)?;
        let outcome = if sha256(&export::wav_bytes(&got)) == field(label, e, "wav_sha256")? {
            None
        } else if let Some(flac) = e["flac"].as_str() {
            let range = ExportRange::from(&s.globals);
            Some(
                match first_difference(&got, &flac_samples(&refs.join(flac))?) {
                    Some(d) => describe(d, range),
                    None => "the samples equal the FLAC's, but the sha256 differs from \
                         wav_sha256 (the manifest and the FLAC disagree)"
                        .into(),
                },
            )
        } else {
            Some(format!(
                "sha256 differs ({} frames, reference {}); no audio committed, so no first \
                 difference",
                got.len() / 2,
                e["frames"]
            ))
        };
        equal += report(label, idle, outcome) as usize;
    }
    println!("{equal} of {} equal", entries.len());
    Ok(equal == entries.len())
}

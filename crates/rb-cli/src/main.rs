//! The `rb` command line tool: renders an `.rbs` song to audio (`rb render`), and compares
//! renders with their references (`rb compare`).
//!
//! Exit codes: 0 success, 1 the song or the output failed (unreadable, invalid, not writable)
//! or, for `compare`, a song differs from its reference, 2 a usage error (bad arguments,
//! unknown output extension).

mod compare;
mod encode;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use encode::Format;
use rb_engine::frame::export;
use rb_engine::frame::{Devices, Engine};
use rb_engine::samples::SampleSet;
use rb_engine::song::{DrumPattern, MAX_TICK, Song, TICKS_PER_BAR};
use rb_engine::tables::Tables;

#[derive(Parser)]
#[command(name = "rb", about = "ReBirth RB-338 engine reimplementation")]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Render a song's loop as the original's "Export Loop as Audio File" does (44.1 kHz, 16-bit
    /// stereo), encoded by the output extension: .wav, .flac (lossless) or .ogg (Vorbis).
    Render {
        /// The song, an .rbs file.
        input: PathBuf,
        /// The output file: .wav, .flac or .ogg.
        #[arg(short, long)]
        output: PathBuf,
        /// Ogg Vorbis quality, -0.2 (smallest) to 1.0 (best). Ignored for other formats.
        #[arg(long, default_value_t = 0.5, allow_hyphen_values = true)]
        quality: f32,
        /// Render the bars START:END instead of the song's loop. Bars are the original's song
        /// positions, counted from 0 as in the file (tick = bar * 768, four beats of 192
        /// ticks); END is exclusive, so 0:2 is the first two bars.
        #[arg(long, value_name = "START:END")]
        range: Option<String>,
        /// The initial-state preset: the number of idle 64-frame blocks the engine runs before
        /// the export starts. The default, 9088, is the reference corpus' capture protocol; the
        /// original's own export starts from whatever the running app was doing.
        #[arg(long, default_value_t = 9088, value_name = "N")]
        idle_blocks: u32,
        /// The ReBirth 2.01 ReBirth.dll, which holds the built-in drum samples. Needed only
        /// when a drum machine plays. Without it the search is: the RB_REBIRTH_DLL environment
        /// variable, then a ReBirth.dll next to the rb binary, then the usual install
        /// locations on Windows.
        #[arg(long, value_name = "PATH")]
        rebirth_dll: Option<PathBuf>,
    },
    /// Render songs and compare each with its reference, byte for byte as a WAV: prints EQUAL
    /// or the first difference (frame, time, bar, channel, both values) per song, and exits 1
    /// when any song differs. Either `SONG --reference REF` or `--manifest PATH [NAME...]`.
    Compare {
        /// With --reference: the song, an .rbs file. With --manifest: the entries to compare,
        /// as NAME or SET/NAME (all of the sets' entries when none is given).
        names: Vec<String>,
        /// The reference of the one song: a .flac or a .wav (16-bit stereo 44100 Hz).
        #[arg(long, value_name = "PATH")]
        reference: Option<PathBuf>,
        /// A corpus manifest, e.g. tests/refs/manifest.json.
        #[arg(long, value_name = "PATH")]
        manifest: Option<PathBuf>,
        /// With --manifest: the sets to compare, comma-separated (default diag,app,shipped).
        #[arg(long = "set", value_name = "SET", value_delimiter = ',')]
        sets: Vec<String>,
        /// The idle 64-frame blocks before the export, as in render. Default: each manifest
        /// entry's own idle_blocks, or 9088 for a single song.
        #[arg(long, value_name = "N")]
        idle_blocks: Option<u32>,
        /// The ReBirth.dll with the built-in drum samples, as in render.
        #[arg(long, value_name = "PATH")]
        rebirth_dll: Option<PathBuf>,
    },
}

/// A failure with its exit code.
struct Fail(u8, String);

fn fail(code: u8, msg: impl Into<String>) -> Fail {
    Fail(code, msg.into())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Cmd::Render {
            input,
            output,
            quality,
            range,
            idle_blocks,
            rebirth_dll,
        } => render(
            &input,
            &output,
            quality,
            range.as_deref(),
            idle_blocks,
            rebirth_dll.as_deref(),
        ),
        Cmd::Compare {
            names,
            reference,
            manifest,
            sets,
            idle_blocks,
            rebirth_dll,
        } => compare::run(&compare::Args {
            names,
            reference,
            manifest,
            sets,
            idle_blocks,
            rebirth_dll,
        })
        .and_then(|equal| {
            if equal {
                Ok(())
            } else {
                Err(Fail(1, "some songs differ from their references".into()))
            }
        }),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(Fail(code, msg)) => {
            eprintln!("rb: error: {msg}");
            ExitCode::from(code)
        }
    }
}

fn parse_range(s: &str) -> Result<(u32, u32), Fail> {
    let bad = || {
        fail(
            2,
            format!("--range {s:?}: expected START:END in bars, e.g. 0:4"),
        )
    };
    let (a, b) = s.split_once(':').ok_or_else(bad)?;
    let a: u32 = a.trim().parse().map_err(|_| bad())?;
    let b: u32 = b.trim().parse().map_err(|_| bad())?;
    if a >= b {
        return Err(fail(2, format!("--range {s:?}: START must be below END")));
    }
    if b.saturating_mul(TICKS_PER_BAR) > MAX_TICK {
        return Err(fail(
            2,
            format!(
                "--range {s:?}: END is past bar {}",
                MAX_TICK / TICKS_PER_BAR
            ),
        ));
    }
    Ok((a * TICKS_PER_BAR, b * TICKS_PER_BAR))
}

/// The drum machines that play in the song: "TR-808" and "TR-909".
fn drums_playing(song: &Song) -> Vec<&'static str> {
    // A drum machine is audible when it is enabled and a pattern it plays has a step: the
    // initial selection (controller 1, `params[1]`) plus the values of its controller-1 track
    // events (tracks 3 and 4, docs/song_load.md section 5). With no such event the initial
    // selection plays throughout.
    // The enable flag counts as on when the chunk or any enable event (controller 0) sets it.
    let plays = |params: &[u8], track: usize, patterns: &[DrumPattern]| {
        (params[0] != 0
            || song.tracks[track]
                .iter()
                .any(|e| e.ctrl == 0 && e.value != 0))
            && std::iter::once(params[1])
                .chain(
                    song.tracks[track]
                        .iter()
                        .filter(|e| e.ctrl == 1)
                        .map(|e| e.value),
                )
                .filter_map(|p| patterns.get(p as usize))
                .any(|p| p.steps.iter().any(|s| s.iter().any(|&c| c != 0)))
    };
    let mut v = Vec::new();
    if plays(&song.tr808.params, 3, &song.tr808.patterns) {
        v.push("TR-808");
    }
    if plays(&song.tr909.params, 4, &song.tr909.patterns) {
        v.push("TR-909");
    }
    v
}

/// The ReBirth.dll to read the drum samples from, in the order: the `--rebirth-dll` path (which
/// must exist), the `RB_REBIRTH_DLL` environment variable (likewise), a ReBirth.dll next to the
/// rb binary, then the Windows install locations. An explicit choice is never second-guessed
/// by a search; the implicit locations are tried only when nothing was asked for.
fn locate_rebirth_dll(explicit: Option<&Path>) -> Result<PathBuf, Fail> {
    if let Some(p) = explicit {
        return if p.is_file() {
            Ok(p.to_path_buf())
        } else {
            Err(fail(
                1,
                format!("--rebirth-dll {}: no such file", p.display()),
            ))
        };
    }
    if let Some(p) = std::env::var_os("RB_REBIRTH_DLL").filter(|p| !p.is_empty()) {
        let p = PathBuf::from(p);
        return if p.is_file() {
            Ok(p)
        } else {
            Err(fail(
                1,
                format!("RB_REBIRTH_DLL={}: no such file", p.display()),
            ))
        };
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(d) = std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        dirs.push(d);
    }
    if cfg!(windows) {
        for var in ["ProgramFiles(x86)", "ProgramFiles"] {
            if let Some(pf) = std::env::var_os(var) {
                let pf = PathBuf::from(pf);
                dirs.push(pf.join("Propellerhead").join("ReBirth"));
                dirs.push(pf.join("ReBirth RB-338"));
            }
        }
        dirs.push(PathBuf::from(
            r"C:\Program Files (x86)\Propellerhead\ReBirth",
        ));
        dirs.push(PathBuf::from(r"C:\ReBirth RB-338"));
    }
    for d in dirs {
        for name in ["ReBirth.dll", "Rebirth.dll", "rebirth.dll"] {
            let p = d.join(name);
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    Err(fail(
        1,
        "no ReBirth.dll found. Pass --rebirth-dll PATH, set RB_REBIRTH_DLL, or put it next to the rb binary",
    ))
}

/// Loads the built-in drum samples from the ReBirth.dll that `locate_rebirth_dll` finds.
fn load_samples(explicit: Option<&Path>, drums: &[&str]) -> Result<SampleSet, Fail> {
    let need = format!(
        "the song plays the {}, which needs the built-in drum samples",
        drums.join(" and ")
    );
    let path =
        locate_rebirth_dll(explicit).map_err(|Fail(c, m)| fail(c, format!("{need}; {m}")))?;
    let bytes = std::fs::read(&path)
        .map_err(|e| fail(1, format!("{}: cannot read: {e}", path.display())))?;
    SampleSet::from_dll(&bytes).map_err(|e| {
        fail(
            1,
            format!(
                "{}: not a ReBirth 2.01 ReBirth.dll with the built-in samples: {e}",
                path.display()
            ),
        )
    })
}

fn render(
    input: &Path,
    output: &Path,
    quality: f32,
    range: Option<&str>,
    idle_blocks: u32,
    rebirth_dll: Option<&Path>,
) -> Result<(), Fail> {
    let format =
        Format::from_extension(output.extension().and_then(|e| e.to_str())).ok_or_else(|| {
            fail(
                2,
                format!(
                    "{}: the output extension must be .wav, .flac or .ogg",
                    output.display()
                ),
            )
        })?;
    if !(-0.2..=1.0).contains(&quality) {
        return Err(fail(
            2,
            format!("--quality {quality}: expected -0.2 to 1.0"),
        ));
    }
    let range = range.map(parse_range).transpose()?;

    let mut song = load_song(input)?;
    if let Some((start, end)) = range {
        song.globals.loop_start = start;
        song.globals.loop_end = end;
    }
    warn_unsupported(&song);
    let label = input.display().to_string();
    let samples = Renderer::new(rebirth_dll).render(&label, &song, idle_blocks)?;
    let bytes = encode::encode(format, &samples, quality)
        .map_err(|e| fail(1, format!("encoding failed: {e}")))?;
    std::fs::write(output, bytes)
        .map_err(|e| fail(1, format!("{}: cannot write: {e}", output.display())))?;
    Ok(())
}

/// Reads and converts an `.rbs` song.
fn load_song(input: &Path) -> Result<Song, Fail> {
    let data = std::fs::read(input)
        .map_err(|e| fail(1, format!("{}: cannot read the song: {e}", input.display())))?;
    let parsed = rbs::Song::parse(&data).map_err(|e| {
        fail(
            1,
            format!("{}: not a valid .rbs song: {e}", input.display()),
        )
    })?;
    rb_engine::load::convert(&parsed).map_err(|e| {
        fail(
            1,
            format!("{}: cannot load the song: {e:?}", input.display()),
        )
    })
}

/// Warns of what the render cannot do as the original would: a mod, pattern mode.
fn warn_unsupported(song: &Song) {
    if song.mod_name != "Standard ReBirth" {
        eprintln!(
            "rb: warning: the song uses the mod {:?}; mods are not supported yet (m-2), \
             rendering with the built-in kit",
            song.mod_name
        );
    }
    if song.globals.song_mode == 0 {
        eprintln!(
            "rb: warning: the song is in pattern mode; the original cannot export it, the \
             output is the engine's song-mode render of the loop"
        );
    }
}

/// Renders songs, each in a new engine with every device, sharing the tables and the drum
/// samples (loaded from the ReBirth.dll the first time a song plays a drum machine).
struct Renderer {
    tables: Arc<Tables>,
    rebirth_dll: Option<PathBuf>,
    samples: Option<Arc<SampleSet>>,
}

impl Renderer {
    fn new(rebirth_dll: Option<&Path>) -> Self {
        Renderer {
            tables: Arc::new(Tables::new()),
            rebirth_dll: rebirth_dll.map(Path::to_path_buf),
            samples: None,
        }
    }

    /// The export of `song` after `idle_blocks` idle blocks: interleaved int16 stereo.
    /// `label` names the song in a warning.
    fn render(&mut self, label: &str, song: &Song, idle_blocks: u32) -> Result<Vec<i16>, Fail> {
        // The drum samples are loaded only for a song that plays a drum machine; a 303-only
        // song renders without a ReBirth.dll (its drum machines play no step, so the silent
        // placeholders the engine uses without samples are exact).
        let drums = drums_playing(song);
        let samples = if drums.is_empty() {
            None
        } else {
            if self.samples.is_none() {
                let set = load_samples(self.rebirth_dll.as_deref(), &drums)?;
                self.samples = Some(Arc::new(set));
            }
            self.samples.clone()
        };
        let mut engine = Engine::new(self.tables.clone(), Devices::implemented(samples));
        let samples = export::export_song(&mut engine, song, idle_blocks);
        // A render call that faults in the original (the PCF's divide by zero) writes nothing;
        // the output is what the original writes, but say why it is silent or repeats itself.
        if let Some(f) = engine.fault() {
            eprintln!(
                "rb: warning: {label}: the original produces silence here: {}; {} render calls \
                 faulted, the first in block {}, and each left its output unwritten",
                f.first, f.calls, f.block
            );
        }
        Ok(samples)
    }
}

//! The `rb` command line tool: renders an `.rbs` song to audio.
//!
//! Exit codes: 0 success, 1 the song or the output failed (unreadable, invalid, not writable),
//! 2 a usage error (bad arguments, unknown output extension).

mod encode;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, Subcommand};
use encode::Format;
use rb_engine::frame::export;
use rb_engine::frame::{Devices, Engine};
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
    },
}

/// A failure with its exit code.
struct Fail(u8, String);

fn fail(code: u8, msg: impl Into<String>) -> Fail {
    Fail(code, msg.into())
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let Cmd::Render {
        input,
        output,
        quality,
        range,
        idle_blocks,
    } = cli.command;
    match render(&input, &output, quality, range.as_deref(), idle_blocks) {
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

/// The devices the song uses that the engine does not implement yet (task-024..027, m-2).
fn unimplemented_devices(song: &Song) -> Vec<&'static str> {
    // A drum machine is audible when it is enabled and a pattern it plays has a step: the
    // initial selection (controller 1, `params[1]`) plus the values of its controller-1 track
    // events (tracks 3 and 4, docs/song_load.md section 5). With no such event the initial
    // selection plays throughout.
    let plays = |params: &[u8], track: usize, patterns: &[DrumPattern]| {
        params[0] != 0
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
    for (name, on) in [
        ("distortion", song.dist.params[0]),
        ("delay", song.delay.params[0]),
        ("PCF", song.pcf.params[0]),
        ("compressor", song.comp.params[0]),
    ] {
        if on != 0 {
            v.push(name);
        }
    }
    v
}

fn render(
    input: &Path,
    output: &Path,
    quality: f32,
    range: Option<&str>,
    idle_blocks: u32,
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

    let data = std::fs::read(input)
        .map_err(|e| fail(1, format!("{}: cannot read the song: {e}", input.display())))?;
    let parsed = rbs::Song::parse(&data).map_err(|e| {
        fail(
            1,
            format!("{}: not a valid .rbs song: {e}", input.display()),
        )
    })?;
    let mut song = rb_engine::load::convert(&parsed).map_err(|e| {
        fail(
            1,
            format!("{}: cannot load the song: {e:?}", input.display()),
        )
    })?;
    if let Some((start, end)) = range {
        song.globals.loop_start = start;
        song.globals.loop_end = end;
    }

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
    let missing = unimplemented_devices(&song);
    if !missing.is_empty() {
        eprintln!(
            "rb: warning: not implemented yet, you will not hear them: {}",
            missing.join(", ")
        );
    }

    let mut engine = Engine::new(Arc::new(Tables::new()), Devices::implemented());
    let samples = export::export_song(&mut engine, &song, idle_blocks);
    let bytes = encode::encode(format, &samples, quality)
        .map_err(|e| fail(1, format!("encoding failed: {e}")))?;
    std::fs::write(output, bytes)
        .map_err(|e| fail(1, format!("{}: cannot write: {e}", output.display())))?;
    Ok(())
}

//! The `rb` command line tool: renders an `.rbs` song to audio (`rb render`), compares renders
//! with their references (`rb compare`), plays a song through the audio output with the
//! real-time player (`rb play`), rewrites a mod to need no RB20FUL.DAT (`rb recrypt`), and
//! builds the standard mod, the built-in kit and panel as a mod (`rb standard-mod`), from which
//! the other commands take the built-in drum samples.
//!
//! Exit codes: 0 success, 1 the song or the output failed (unreadable, invalid, not writable)
//! or, for `compare`, a song differs from its reference, 2 a usage error (bad arguments,
//! unknown output extension).

mod compare;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use rb_cli::encode::Format;
use rb_cli::mods::{self, ModOpts};
use rb_cli::render::{Renderer, load_song, read_mod, warn_unsupported};
use rb_cli::{Fail, fail};
use rb_engine::song::{MAX_TICK, TICKS_PER_BAR};

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
        /// The standard mod (made once by `rb standard-mod --rebirth-dll ReBirth.dll`), which
        /// holds the built-in drum samples. Needed only when a drum machine plays. Without it
        /// the search is: the RB_STANDARD_MOD environment variable, then Standard.rbm next to
        /// the rb binary, then a mod named "ReBirth Standard Kit" in the mod folder.
        #[arg(long, value_name = "PATH")]
        standard_mod: Option<PathBuf>,
        /// The song's mod is looked up in this folder of .rbm files, by the name in the song
        /// (the mod's INFO name, as the original's Mods menu; a file name also matches).
        /// Default: the RB_MOD_DIR environment variable (RB_RBM_DIR is a
        /// deprecated alias), then a Mods folder next to the rb
        /// binary, then the usual install locations on Windows. "Standard ReBirth" needs none.
        /// Only recrypted mods (rb recrypt) load; a mod that is not is still found by name and
        /// fails with an error that says to run rb recrypt on it.
        #[arg(long, value_name = "DIR")]
        mod_dir: Option<PathBuf>,
        /// Use this .rbm mod whatever mod the song names.
        #[arg(long = "mod", value_name = "FILE")]
        mod_file: Option<PathBuf>,
        /// When the song's mod is not found, render with the built-in kit (with a warning)
        /// instead of failing; the original offers the same as "Standard".
        #[arg(long)]
        standard_kit: bool,
    },
    /// Play a song through the default audio output device with the real-time player, for
    /// listening (no UI): from its song position (or --bar) until the song stops (or Ctrl+C).
    /// A song in pattern mode plays its patterns until --seconds.
    Play {
        /// The song, an .rbs file.
        input: PathBuf,
        /// Start at this bar, counted from 0 as in --range.
        #[arg(long, value_name = "N")]
        bar: Option<u32>,
        /// Stop after this many seconds.
        #[arg(long, value_name = "S")]
        seconds: Option<f64>,
        /// The standard mod with the built-in drum samples, as in render.
        #[arg(long, value_name = "PATH")]
        standard_mod: Option<PathBuf>,
        /// The folder of .rbm mods, as in render.
        #[arg(long, value_name = "DIR")]
        mod_dir: Option<PathBuf>,
        /// Use this .rbm mod whatever mod the song names.
        #[arg(long = "mod", value_name = "FILE")]
        mod_file: Option<PathBuf>,
        /// When the song's mod is not found, play with the built-in kit, as in render.
        #[arg(long)]
        standard_kit: bool,
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
        /// With --manifest: the sets to compare, comma-separated (default diag,app,shipped,
        /// plus mod,modapp when a mod folder is given with --mod-dir or RB_MOD_DIR).
        #[arg(long = "set", value_name = "SET", value_delimiter = ',')]
        sets: Vec<String>,
        /// The idle 64-frame blocks before the export, as in render. Default: each manifest
        /// entry's own idle_blocks, or 9088 for a single song.
        #[arg(long, value_name = "N")]
        idle_blocks: Option<u32>,
        /// The standard mod with the built-in drum samples, as in render.
        #[arg(long, value_name = "PATH")]
        standard_mod: Option<PathBuf>,
        /// As in render: the song's mod is looked up in this folder of .rbm files, by the name
        /// in the song (the mod's INFO name, as the original's Mods menu; a file name also matches).
        /// Default: the RB_MOD_DIR environment variable (RB_RBM_DIR is a
        /// deprecated alias), then a Mods folder next to the rb
        /// binary, then the usual install locations on Windows. "Standard ReBirth" needs none.
        /// Only recrypted mods (rb recrypt) load; a mod that is not is still found by name and
        /// fails with an error that says to run rb recrypt on it.
        #[arg(long, value_name = "DIR")]
        mod_dir: Option<PathBuf>,
        /// Use this .rbm mod whatever mod the song names.
        #[arg(long = "mod", value_name = "FILE")]
        mod_file: Option<PathBuf>,
        /// When the song's mod is not found, render with the built-in kit (with a warning)
        /// instead of failing; the original offers the same as "Standard".
        #[arg(long)]
        standard_kit: bool,
    },
    /// Rewrite a .rbm mod so that it needs no RB20FUL.DAT: rb then loads it with the 2047 key
    /// bytes it embeds, and the original ReBirth loads it unchanged (it reads the same bytes of
    /// its own RB20FUL.DAT). Needs the key file once, unless the mod is already recrypted. This
    /// is the only rb command that reads RB20FUL.DAT: render, compare and play load recrypted
    /// mods only.
    Recrypt {
        /// The mod, an .rbm file.
        input: PathBuf,
        /// The recrypted mod to write.
        #[arg(short, long)]
        output: PathBuf,
        /// The CD's RB20FUL.DAT. Search order: this option, the RB_RB20FUL environment
        /// variable, then an RB20FUL.DAT in the folder of the input mod (or its parent), next
        /// to the rb binary, or in the usual install locations on Windows.
        #[arg(long, value_name = "PATH")]
        rb20ful: Option<PathBuf>,
    },
    /// Build the standard mod: the original's built-in kit (the 46 samples of ReBirth.dll) and
    /// panel (the bitmaps of Rebirth.exe) as a recrypted .rbm named "ReBirth Standard Kit".
    /// render, compare and play take the built-in kit from it; the original ReBirth loads it
    /// from its Mods folder like any mod. Run it once; it is the only rb command that reads
    /// ReBirth.dll.
    StandardMod {
        /// The ReBirth 2.01 ReBirth.dll. Search order: this option, then a ReBirth.dll next to
        /// the rb binary, then the usual install locations on Windows.
        #[arg(long, value_name = "PATH")]
        rebirth_dll: Option<PathBuf>,
        /// The ReBirth 2.01 Rebirth.exe, which holds the panel bitmaps. Default: the
        /// Rebirth.exe next to the ReBirth.dll.
        #[arg(long, value_name = "PATH")]
        rebirth_exe: Option<PathBuf>,
        /// The mod to write. Default: Standard.rbm next to the rb binary, where render,
        /// compare and play look for it.
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
    },
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
            standard_mod,
            mod_dir,
            mod_file,
            standard_kit,
        } => render(
            &input,
            &output,
            quality,
            range.as_deref(),
            idle_blocks,
            standard_mod.as_deref(),
            ModOpts {
                mod_file,
                mod_dir,
                standard_kit,
            },
        ),
        Cmd::Play {
            input,
            bar,
            seconds,
            standard_mod,
            mod_dir,
            mod_file,
            standard_kit,
        } => play(
            &input,
            bar,
            seconds,
            standard_mod.as_deref(),
            ModOpts {
                mod_file,
                mod_dir,
                standard_kit,
            },
        ),
        Cmd::Compare {
            names,
            reference,
            manifest,
            sets,
            idle_blocks,
            standard_mod,
            mod_dir,
            mod_file,
            standard_kit,
        } => compare::run(&compare::Args {
            names,
            reference,
            manifest,
            sets,
            idle_blocks,
            standard_mod,
            mods: ModOpts {
                mod_file,
                mod_dir,
                standard_kit,
            },
        })
        .and_then(|equal| {
            if equal {
                Ok(())
            } else {
                Err(Fail(1, "some songs differ from their references".into()))
            }
        }),
        Cmd::Recrypt {
            input,
            output,
            rb20ful,
        } => recrypt(&input, &output, rb20ful),
        Cmd::StandardMod {
            rebirth_dll,
            rebirth_exe,
            output,
        } => standard_mod(
            rebirth_dll.as_deref(),
            rebirth_exe.as_deref(),
            output.as_deref(),
        ),
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

/// The ReBirth.dll `rb standard-mod` reads, in the order: the `--rebirth-dll` path (which must
/// exist), a ReBirth.dll next to the rb binary, then the Windows install locations. An explicit
/// choice is never second-guessed by a search.
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
        "no ReBirth.dll found. Pass --rebirth-dll PATH, or put it next to the rb binary",
    ))
}

/// The Rebirth.exe `rb standard-mod` reads: `--rebirth-exe` (which must exist), else the
/// Rebirth.exe in the folder of the ReBirth.dll.
fn locate_rebirth_exe(explicit: Option<&Path>, dll: &Path) -> Result<PathBuf, Fail> {
    if let Some(p) = explicit {
        return if p.is_file() {
            Ok(p.to_path_buf())
        } else {
            Err(fail(
                1,
                format!("--rebirth-exe {}: no such file", p.display()),
            ))
        };
    }
    let dir = dll.parent().unwrap_or(Path::new("."));
    ["Rebirth.exe", "ReBirth.exe", "rebirth.exe", "REBIRTH.EXE"]
        .iter()
        .map(|n| dir.join(n))
        .find(|p| p.is_file())
        .ok_or_else(|| {
            fail(
                1,
                format!(
                    "no Rebirth.exe next to {} (it holds the panel bitmaps). Pass --rebirth-exe PATH",
                    dll.display()
                ),
            )
        })
}

/// `rb standard-mod`: the built-in kit and panel as a recrypted mod (task-036).
fn standard_mod(dll: Option<&Path>, exe: Option<&Path>, output: Option<&Path>) -> Result<(), Fail> {
    let dll = locate_rebirth_dll(dll)?;
    let exe = locate_rebirth_exe(exe, &dll)?;
    let output = match output {
        Some(o) => o.to_path_buf(),
        None => mods::default_standard_mod().ok_or_else(|| {
            fail(
                1,
                "cannot tell where the rb binary is; pass -o FILE for the standard mod",
            )
        })?,
    };
    let read = |p: &Path| {
        std::fs::read(p).map_err(|e| fail(1, format!("{}: cannot read: {e}", p.display())))
    };
    let built = rbm::standard::build(&read(&dll)?, &read(&exe)?).map_err(|e| {
        fail(
            1,
            format!("{e} (from {} and {})", dll.display(), exe.display()),
        )
    })?;
    std::fs::write(&output, &built.bytes)
        .map_err(|e| fail(1, format!("{}: cannot write: {e}", output.display())))?;
    eprintln!(
        "rb: {}: the standard mod \"{}\": {} samples from {}, {} panel bitmaps from {}, {} bytes",
        output.display(),
        rbm::standard::NAME,
        rb_engine::samples::COUNT,
        dll.display(),
        built.jpegs,
        exe.display(),
        built.bytes.len()
    );
    Ok(())
}

fn render(
    input: &Path,
    output: &Path,
    quality: f32,
    range: Option<&str>,
    idle_blocks: u32,
    standard_mod: Option<&Path>,
    mods: ModOpts,
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
    Renderer::new(standard_mod, mods).render_to_file(
        &label,
        &song,
        idle_blocks,
        format,
        quality,
        output,
    )
}

/// `rb play`: the song in the real-time player (`rb-player`) on the default output device.
fn play(
    input: &Path,
    bar: Option<u32>,
    seconds: Option<f64>,
    standard_mod: Option<&Path>,
    mods: ModOpts,
) -> Result<(), Fail> {
    use rb_player::{Command, Player};
    if let Some(s) = seconds
        && (s.is_nan() || s <= 0.0)
    {
        return Err(fail(
            2,
            format!("--seconds {s}: expected a positive number"),
        ));
    }
    if let Some(b) = bar
        && b.saturating_mul(TICKS_PER_BAR) >= MAX_TICK
    {
        return Err(fail(2, format!("--bar {b}: past the last bar")));
    }
    let song = load_song(input)?;
    let label = input.display().to_string();
    let mut r = Renderer::new(standard_mod, mods);
    let samples = r.samples(&label, &song)?;
    let (mut player, audio) = Player::new(r.tables.clone(), samples.clone());
    let queue = |_| fail(1, "the player's command queue is full");
    player.load(&song, samples).map_err(queue)?;
    if let Some(b) = bar {
        player
            .send(Command::Locate(b * TICKS_PER_BAR))
            .map_err(queue)?;
    }
    player.send(Command::Play).map_err(queue)?;
    let stream =
        rb_player::output::start(audio).map_err(|e| fail(1, format!("audio output: {e}")))?;
    eprintln!(
        "rb: playing {label} on {} ({} Hz{}, {} channels, {}); Ctrl+C stops",
        stream.device,
        stream.config.sample_rate,
        if stream.resampled {
            ", resampled from 44100"
        } else {
            ""
        },
        stream.config.channels,
        stream.format
    );
    let start = std::time::Instant::now();
    let mut started = false;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        player.collect_garbage();
        if stream.failed() {
            eprintln!();
            return Err(fail(1, "the audio output stopped with an error"));
        }
        let s = *player.snapshot();
        started |= s.playing;
        if s.song_mode {
            eprint!("\rbar {:4}", s.tick / TICKS_PER_BAR as i32 + 1);
        }
        if (started && !s.playing) || seconds.is_some_and(|t| start.elapsed().as_secs_f64() >= t) {
            break;
        }
    }
    eprintln!();
    drop(stream);
    Ok(())
}

fn read_key_table(rbm: &rbm::Rbm, key_file: &Path) -> Result<rbm::KeyTable, Fail> {
    let mut dat = std::fs::File::open(key_file)
        .map_err(|e| fail(1, format!("{}: cannot read: {e}", key_file.display())))?;
    rbm::KeyTable::read(rbm, &mut dat).map_err(|e| fail(1, format!("{}: {e}", key_file.display())))
}

/// `rb recrypt`: rewrites a mod to need only the key bytes rb embeds (task-059).
fn recrypt(input: &Path, output: &Path, rb20ful: Option<PathBuf>) -> Result<(), Fail> {
    let rbm = read_mod(input)?;
    let table = match rbm::KeyTable::embedded(&rbm).filter(|t| rbm.all_covered(t)) {
        Some(t) => t,
        None => {
            let key_file =
                mods::find_rb20ful(rb20ful.as_deref(), input).map_err(|Fail(c, m)| {
                    fail(
                        c,
                        format!(
                            "{}: recrypting needs the key file once; {m}",
                            input.display()
                        ),
                    )
                })?;
            read_key_table(&rbm, &key_file)?
        }
    };
    let out =
        rbm::recrypt(&rbm, &table).map_err(|e| fail(1, format!("{}: {e}", input.display())))?;
    let chunks = rbm::Rbm::parse(out.clone())
        .map(|r| r.chunks().len())
        .map_err(|e| {
            fail(
                1,
                format!(
                    "{}: internal: the recrypted mod does not parse: {e}",
                    input.display()
                ),
            )
        })?;
    let before = std::fs::metadata(input)
        .map(|m| m.len())
        .map_err(|e| fail(1, format!("{}: {e}", input.display())))?;
    std::fs::write(output, &out)
        .map_err(|e| fail(1, format!("{}: cannot write: {e}", output.display())))?;
    eprintln!(
        "rb: {} -> {}: {} -> {} chunks, {before} -> {} bytes",
        input.display(),
        output.display(),
        rbm.chunks().len(),
        chunks,
        out.len()
    );
    Ok(())
}

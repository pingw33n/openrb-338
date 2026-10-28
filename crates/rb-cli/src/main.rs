//! The `rb` command line tool: renders an `.rbs` song to audio (`rb render`), compares renders
//! with their references (`rb compare`), plays a song through the audio output with the
//! real-time player (`rb play`), rewrites a mod to need no RB20FUL.DAT (`rb mod recrypt`), and
//! builds the standard mod, the built-in kit and panel as a mod (`rb mod pack-standard`). rb
//! embeds that mod (`assets/Standard.rbm`) and takes the built-in drum samples from it. `rb mod
//! unpack` and `rb mod pack` turn a recrypted mod into a folder of its files with a mod.toml and
//! back.
//!
//! Exit codes: 0 success, 1 the song or the output failed (unreadable, invalid, not writable)
//! or, for `compare`, a song differs from its reference, 2 a usage error (bad arguments,
//! unknown output extension).

mod compare;
mod modcmd;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use rb_cli::encode::Format;
use rb_cli::mods::ModOpts;
use rb_cli::render::{Renderer, load_song, warn_unsupported};
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
        /// The song's mod is looked up in this folder of .rbm files, by the name in the song
        /// (the mod's INFO name, as the original's Mods menu; a file name also matches).
        /// Default: the RB_MOD_DIR environment variable (RB_RBM_DIR is a
        /// deprecated alias), then a Mods folder next to the rb
        /// binary, then the usual install locations on Windows. "Standard ReBirth", the
        /// built-in kit, needs none: rb embeds it. A mod's missing samples come from it.
        /// Only recrypted mods (rb mod recrypt) load; a mod that is not is still found by name and
        /// fails with an error that says to run rb mod recrypt on it.
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
        /// As in render: the song's mod is looked up in this folder of .rbm files, by the name
        /// in the song (the mod's INFO name, as the original's Mods menu; a file name also matches).
        /// Default: the RB_MOD_DIR environment variable (RB_RBM_DIR is a
        /// deprecated alias), then a Mods folder next to the rb
        /// binary, then the usual install locations on Windows. "Standard ReBirth", the
        /// built-in kit, needs none: rb embeds it.
        /// Only recrypted mods (rb mod recrypt) load; a mod that is not is still found by name and
        /// fails with an error that says to run rb mod recrypt on it.
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
    /// Work with .rbm mods: recrypt, unpack, pack and pack-standard.
    #[command(name = "mod", subcommand)]
    Mod(ModCmd),
}

#[derive(Subcommand)]
enum ModCmd {
    /// Rewrite .rbm mods so that they need no RB20FUL.DAT: rb then loads them with the 2047 key
    /// bytes it embeds, and the original ReBirth loads them unchanged (it reads the same bytes
    /// of its own RB20FUL.DAT). Needs the key file, unless every mod is already recrypted (such
    /// a mod is checked and copied unchanged). This is the only rb command that reads
    /// RB20FUL.DAT: render, compare and play load recrypted mods only.
    ///
    /// The key file must be the 2001 reissue CD's: its SHA-256 is checked once per run, before
    /// anything is written (another edition is not supported yet). Every decrypted sample must
    /// parse as AIFF and every JPEG must start with FF D8, else that mod is not written.
    ///
    /// One mod: `rb mod recrypt IN.rbm -o OUT.rbm`. Several mods or folders (every .rbm in a
    /// folder): `rb mod recrypt IN.rbm... DIR... -o OUTDIR`, which writes the mods under their
    /// names; OUTDIR must not be an input folder. A mod that fails does not stop the others;
    /// the summary lists the failures and the exit code is 1 if any failed.
    ///
    /// `--iso CD.iso` reads RB20FUL.DAT from the ReBirth CD image itself (nothing is extracted);
    /// with no input mod, the four mods shipped on the CD (inside its installer) are recrypted
    /// into the folder given with -o.
    Recrypt {
        /// The mods (.rbm files) and folders of them. None with --iso: the CD's shipped mods.
        #[arg(required_unless_present = "iso")]
        inputs: Vec<PathBuf>,
        /// The recrypted mod to write (one input file, a path ending in .rbm), else the folder
        /// to write the mods into (created).
        #[arg(short, long)]
        output: PathBuf,
        /// Overwrite output files that exist.
        #[arg(long)]
        force: bool,
        /// The CD's RB20FUL.DAT. Search order: this option, the RB_RB20FUL environment
        /// variable, then an RB20FUL.DAT in the folder of the input mod (or its parent), next
        /// to the rb binary, or in the usual install locations on Windows.
        #[arg(long, value_name = "PATH", conflicts_with = "iso")]
        rb20ful: Option<PathBuf>,
        /// The ReBirth 2.01 CD as an ISO 9660 image (.iso). RB20FUL.DAT is read from its root.
        /// Its SHA-256 is checked as for --rb20ful.
        #[arg(long, value_name = "CD.iso")]
        iso: Option<PathBuf>,
    },
    /// Build the standard mod: the original's built-in kit (the 46 samples of ReBirth.dll) and
    /// panel (the bitmaps of Rebirth.exe) as a recrypted .rbm named "Standard ReBirth". rb
    /// embeds the one it was built with (assets/Standard.rbm, from the CD's Rebirth.exe), so
    /// render, compare and play need no ReBirth file; this command rebuilds it. It is the only
    /// rb command that reads ReBirth.dll and Rebirth.exe.
    ///
    /// `--iso CD.iso` reads both files from the installer on the ReBirth CD image (nothing is
    /// extracted); the result is byte for byte assets/Standard.rbm.
    PackStandard {
        /// The ReBirth 2.01 ReBirth.dll. Search order: this option, then a ReBirth.dll next to
        /// the rb binary, then the usual install locations on Windows.
        #[arg(long, value_name = "PATH", conflicts_with = "iso")]
        rebirth_dll: Option<PathBuf>,
        /// The ReBirth 2.01 Rebirth.exe, which holds the panel bitmaps. Default: the
        /// Rebirth.exe next to the ReBirth.dll.
        #[arg(long, value_name = "PATH", conflicts_with = "iso")]
        rebirth_exe: Option<PathBuf>,
        /// The ReBirth 2.01 CD as an ISO 9660 image (.iso): ReBirth.dll and Rebirth.exe are
        /// read from the CABs inside "Install ReBirth RB-338.exe" and checked by SHA-256.
        #[arg(long, value_name = "CD.iso")]
        iso: Option<PathBuf>,
        /// The mod to write.
        #[arg(short, long, value_name = "FILE")]
        output: PathBuf,
    },
    /// Unpack a recrypted .rbm mod into a folder: every embedded file under its own name
    /// (decrypted) and mod.toml, which holds the mod's name, version, description, URLs and
    /// the file order. Does not read RB20FUL.DAT; a mod that is not recrypted must go through
    /// `rb mod recrypt` first.
    Unpack {
        /// The recrypted mod.
        input: PathBuf,
        /// The folder to write; it must be empty or new unless --force is given.
        #[arg(short, long, value_name = "DIR")]
        output: PathBuf,
        /// Write into a folder that is not empty. Nothing is deleted; a file already there with
        /// the name of one of the mod's files is replaced, but a file of the mod never replaces
        /// another file of the mod (that clash is an error).
        #[arg(long)]
        force: bool,
    },
    /// Pack a folder made by `rb mod unpack` (mod.toml and the files it lists) into a recrypted
    /// .rbm that rb loads without RB20FUL.DAT and the original ReBirth loads with its own. A
    /// listed file that is missing, a file in the folder that mod.toml does not list, more
    /// than 199 files and a too long INFO field are errors.
    Pack {
        /// The folder with mod.toml.
        input: PathBuf,
        /// The mod to write.
        #[arg(short, long, value_name = "MOD.rbm")]
        output: PathBuf,
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
            mod_dir,
            mod_file,
            standard_kit,
        } => render(
            &input,
            &output,
            quality,
            range.as_deref(),
            idle_blocks,
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
            mod_dir,
            mod_file,
            standard_kit,
        } => play(
            &input,
            bar,
            seconds,
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
            mod_dir,
            mod_file,
            standard_kit,
        } => compare::run(&compare::Args {
            names,
            reference,
            manifest,
            sets,
            idle_blocks,
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
        Cmd::Mod(ModCmd::Recrypt {
            inputs,
            output,
            force,
            rb20ful,
            iso,
        }) => recrypt(&inputs, &output, rb20ful, iso, force),
        Cmd::Mod(ModCmd::PackStandard {
            rebirth_dll,
            rebirth_exe,
            iso,
            output,
        }) => match iso {
            Some(iso) => pack_standard_iso(&iso, &output),
            None => pack_standard(rebirth_dll.as_deref(), rebirth_exe.as_deref(), &output),
        },
        Cmd::Mod(ModCmd::Unpack {
            input,
            output,
            force,
        }) => modcmd::unpack(&input, &output, force),
        Cmd::Mod(ModCmd::Pack { input, output }) => modcmd::pack(&input, &output),
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

/// The ReBirth.dll `rb mod pack-standard` reads, in the order: the `--rebirth-dll` path (which must
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

/// The Rebirth.exe `rb mod pack-standard` reads: `--rebirth-exe` (which must exist), else the
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

/// `rb mod pack-standard`: the built-in kit and panel as a recrypted mod (task-036).
fn pack_standard(dll: Option<&Path>, exe: Option<&Path>, output: &Path) -> Result<(), Fail> {
    let dll = locate_rebirth_dll(dll)?;
    let exe = locate_rebirth_exe(exe, &dll)?;
    let read = |p: &Path| {
        std::fs::read(p).map_err(|e| fail(1, format!("{}: cannot read: {e}", p.display())))
    };
    write_standard(
        &read(&dll)?,
        &read(&exe)?,
        &dll.display().to_string(),
        &exe.display().to_string(),
        output,
    )
}

/// `rb mod pack-standard --iso`: ReBirth.dll and Rebirth.exe from the CD image's installer.
fn pack_standard_iso(iso: &Path, output: &Path) -> Result<(), Fail> {
    use rb_cli::cd;
    let mut cd_image = cd::CdImage::open(iso).map_err(|m| fail(1, m))?;
    let inst = cd_image.installer().map_err(|m| fail(1, m))?;
    let dll = cd::read_checked(&inst, "ReBirth.dll", cd::DLL_SHA256).map_err(|m| fail(1, m))?;
    let exe = cd::read_checked(&inst, "Rebirth.exe", cd::EXE_SHA256).map_err(|m| fail(1, m))?;
    let at = |n: &str| format!("{}:{n}", iso.display());
    write_standard(&dll, &exe, &at("ReBirth.dll"), &at("Rebirth.exe"), output)
}

fn write_standard(
    dll: &[u8],
    exe: &[u8],
    dll_name: &str,
    exe_name: &str,
    output: &Path,
) -> Result<(), Fail> {
    let built = rbm::standard::build(dll, exe)
        .map_err(|e| fail(1, format!("{e} (from {dll_name} and {exe_name})")))?;
    std::fs::write(output, &built.bytes)
        .map_err(|e| fail(1, format!("{}: cannot write: {e}", output.display())))?;
    eprintln!(
        "rb: {}: the standard mod \"{}\": {} samples from {}, {} panel bitmaps from {}, {} bytes",
        output.display(),
        rbm::standard::NAME,
        rb_engine::samples::COUNT,
        dll_name,
        built.jpegs,
        exe_name,
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
    Renderer::new(mods).render_to_file(&label, &song, idle_blocks, format, quality, output)
}

/// `rb play`: the song in the real-time player (`rb-player`) on the default output device.
fn play(input: &Path, bar: Option<u32>, seconds: Option<f64>, mods: ModOpts) -> Result<(), Fail> {
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
    let mut r = Renderer::new(mods);
    let samples = r.samples(&label, &song)?;
    let (mut player, audio) = Player::new(r.tables.clone(), samples.clone());
    let queue = |_| fail(1, "the player's command queue is full");
    player.load(&song, samples).map_err(|e| match e {
        // the exe's alert text, then the numbers
        rb_player::LoadError::Refused(r) => fail(1, format!("{label}: {r}")),
        e @ rb_player::LoadError::Queue(_) => fail(1, e.to_string()),
    })?;
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

/// `rb mod recrypt`: rewrites mods to need only the key bytes rb embeds (task-059, task-064).
fn recrypt(
    inputs: &[PathBuf],
    output: &Path,
    rb20ful: Option<PathBuf>,
    iso: Option<PathBuf>,
    force: bool,
) -> Result<(), Fail> {
    let report = rb_cli::recrypt::run(&rb_cli::recrypt::Options {
        inputs,
        output,
        rb20ful: rb20ful.as_deref(),
        iso: iso.as_deref(),
        force,
        expected_sha256: rbm::embedded::DAT_SHA256,
    })?;
    eprintln!("rb: {}", report.summary());
    match report.failed() {
        0 => Ok(()),
        n => Err(fail(
            1,
            format!("{n} of {} mods failed", report.results.len()),
        )),
    }
}

//! `orb-mcp`: the OpenRB-338 tools over the Model Context Protocol, so that an agent can read a
//! song, render it and compare the render with a reference without a shell.
//!
//! The tools wrap the same library the `orb` command line tool uses ([`orb_cli`]), so a tool and
//! its `orb` command can never disagree: `orb_info` is `orb info --json`, `orb_render` is
//! `orb render`, `orb_compare` is `orb compare SONG --reference REF`, and `orb_mod_list` is the
//! mod lookup of `orb render --mod-dir` ([`orb_cli::mods::scan`]).
//!
//! The transport is stdio: the server speaks JSON-RPC on stdin and stdout, so a client starts it
//! as a subprocess (`{"command": "orb-mcp", "args": []}` in an MCP configuration file). Nothing
//! is written to stdout outside the protocol; the warnings of a render, as `orb` prints them, go
//! to stderr.
//!
//! Exit codes are the interface of `orb`, not of a tool: a tool that fails returns a tool-level
//! error ([`CallToolResult::error`]) whose text is the `orb` message, so the client shows why the
//! song, the mod or the output was refused.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use orb_cli::encode::Format;
use orb_cli::mods::ModOpts;
use orb_cli::render::Renderer;
use orb_cli::{Fail, fail, info};

pub mod song;
use orb_engine::song::{MAX_TICK, TICKS_PER_BAR};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock};
use rmcp::{ServerHandler, schemars, tool, tool_handler, tool_router};
use serde::Deserialize;

/// The idle blocks of a render when the request gives no count: the reference corpus' capture
/// protocol, the default of `orb render`.
pub const DEFAULT_IDLE_BLOCKS: u32 = orb_cli::render::DEFAULT_IDLE_BLOCKS;

/// The version of the server, as `initialize` reports it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The MCP server: a renderer that keeps the mods it loaded between calls.
///
/// The renderer holds the tables the engine computes once and the samples of every mod it
/// loaded, so rendering two songs of one mod in a session decodes the mod once. A render needs
/// `&mut`, which is why it sits behind a [`Mutex`]: the client may call two tools at once, and a
/// render is then serialized with the other renders.
#[derive(Clone)]
pub struct Orb {
    inner: Arc<Mutex<Renderer>>,
}

impl Default for Orb {
    fn default() -> Self {
        Orb::new()
    }
}

impl Orb {
    pub fn new() -> Self {
        Orb {
            inner: Arc::new(Mutex::new(Renderer::new(ModOpts::default()))),
        }
    }

    /// Locks the renderer. A panic in another call poisons the lock; the renderer holds no
    /// invariant a panic could break (a cache and a set of tables), so the poison is cleared and
    /// the next call runs.
    fn renderer(&self) -> MutexGuard<'_, Renderer> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// The parameters the tools that read or render a song share: the song and its mod.
#[derive(Debug, Clone, Default, Deserialize, schemars::JsonSchema)]
pub struct SongParams {
    /// The song: an `.rbs` file (RBS 4.x), or a ReBirth 1.x MIDI file.
    pub song: String,
    /// The folder of recrypted `.rbm` mods the song's mod is looked up in, as `orb render
    /// --mod-dir` (default: the `ORB_MOD_DIR` environment variable). Only recrypted mods load;
    /// `orb_mod_recrypt` is not a tool, it needs `RB20FUL.DAT`.
    #[serde(default)]
    pub mod_dir: Option<String>,
    /// Use this `.rbm` mod whatever mod the song names, as `orb render --mod`.
    #[serde(default)]
    pub mod_file: Option<String>,
    /// When the song's mod is not found, use the built-in kit instead of failing, as `orb render
    /// --standard-kit`.
    #[serde(default)]
    pub standard_kit: Option<bool>,
    /// The directory the relative paths of a request count from. An MCP client starts the
    /// server as its own process, so the server's working directory is not the client's; a
    /// relative `song`, `mod_dir`, `mod_file`, `output` or `reference` is joined to this. An
    /// absolute path is unaffected, and with no `cwd` a relative path counts from the server's
    /// own directory.
    #[serde(default)]
    pub cwd: Option<String>,
}

impl SongParams {
    /// The path of the song: relative paths count from `cwd` when one is given.
    fn song_path(&self) -> PathBuf {
        self.resolve(&self.song)
    }

    /// A path of these parameters, anchored at `cwd` when it is given and the path is relative.
    fn resolve(&self, path: &str) -> PathBuf {
        self.anchor(Path::new(path))
    }

    fn anchor(&self, path: &Path) -> PathBuf {
        match &self.cwd {
            Some(dir) if path.is_relative() => PathBuf::from(dir).join(path),
            _ => path.to_path_buf(),
        }
    }

    /// The mod options the command line builds from the same values.
    fn mod_opts(&self) -> ModOpts {
        ModOpts {
            mod_file: self.mod_file.as_deref().map(|p| self.resolve(p)),
            mod_dir: self.mod_dir.as_deref().map(|p| self.resolve(p)),
            standard_kit: self.standard_kit.unwrap_or(false),
        }
    }
}

/// The parameters of `orb_render`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct RenderParams {
    #[serde(flatten)]
    pub song: SongParams,
    /// Where to write the audio. The extension picks the encoding: `.wav` (16-bit stereo 44100
    /// Hz, the bytes the engine exports), `.flac`, or `.ogg`.
    pub output: String,
    /// The bars `START:END` of the song's positions, END exclusive, as `orb render --range`
    /// (default: the song's loop). Bars count from 0 as in the file.
    #[serde(default)]
    pub range: Option<String>,
    /// The idle 64-frame blocks the engine runs before the export starts, as `orb render
    /// --idle-blocks` (default 9088, the reference corpus' capture protocol).
    #[serde(default)]
    pub idle_blocks: Option<u32>,
    /// The Ogg Vorbis quality, -0.2 to 1.0; ignored for the other formats.
    #[serde(default)]
    pub quality: Option<f32>,
}

/// The parameters of `orb_compare`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct CompareParams {
    #[serde(flatten)]
    pub song: SongParams,
    /// The reference to compare the render with: a `.flac` or a 16-bit stereo 44100 Hz `.wav`.
    pub reference: String,
    /// The idle blocks, as in `orb render`; the reference's own count when omitted.
    #[serde(default)]
    pub idle_blocks: Option<u32>,
}

/// The parameters of `orb_build`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct BuildParams {
    /// The spec: a JSON object with the shape `orb_song` returns, merged over the template song.
    /// Leave a field out to keep the template's value; an object with integer-string keys patches
    /// those indices of a list (see `docs/beats.md`).
    pub spec: serde_json::Value,
    /// Where to write the `.rbs`.
    pub output: String,
    /// The directory a relative `output` counts from, as the other tools' `cwd`.
    #[serde(default)]
    pub cwd: Option<String>,
}

impl BuildParams {
    /// A path of these parameters, anchored at `cwd` when it is given and the path is relative.
    fn resolve(&self, path: &str) -> PathBuf {
        let path = Path::new(path);
        match &self.cwd {
            Some(dir) if path.is_relative() => PathBuf::from(dir).join(path),
            _ => path.to_path_buf(),
        }
    }
}

/// The parameters of `orb_mod_list`.
#[derive(Debug, Clone, Deserialize, schemars::JsonSchema)]
pub struct ModListParams {
    /// The folder of `.rbm` mods to list, as `orb render --mod-dir` (default: the `ORB_MOD_DIR`
    /// environment variable, then the install locations).
    #[serde(default)]
    pub mod_dir: Option<String>,
    /// The directory a relative `mod_dir` counts from, as the other tools' `cwd`.
    #[serde(default)]
    pub cwd: Option<String>,
}

impl ModListParams {
    /// A path of these parameters, anchored at `cwd` when it is given and the path is relative.
    fn resolve(&self, path: &str) -> PathBuf {
        let path = Path::new(path);
        match &self.cwd {
            Some(dir) if path.is_relative() => PathBuf::from(dir).join(path),
            _ => path.to_path_buf(),
        }
    }
}

/// The song `orb render` renders: the file's song, with the loop set to `range`'s bars.
fn song_for(song: &mut orb_engine::song::Song, range: Option<&str>) -> Result<(), String> {
    if let Some((start, end)) = range.map(parse_range).transpose()? {
        song.globals.loop_start = start;
        song.globals.loop_end = end;
    }
    Ok(())
}

/// The label a warning or a report names a song with: the path as given.
fn label(path: &Path) -> String {
    path.display().to_string()
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

/// The interleaved int16 samples of a 16-bit PCM WAV.
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

/// What `orb_render` reports of the file it wrote.
fn render_summary(song: &Path, output: &Path, idle_blocks: u32) -> String {
    let bytes = std::fs::metadata(output).map(|m| m.len()).unwrap_or(0);
    let frames = bytes.saturating_sub(44) / 4;
    format!(
        "rendered {} -> {} ({} bytes, {frames} frames, N {idle_blocks})",
        song.display(),
        output.display(),
        bytes
    )
}

#[tool_handler(name = "orb-mcp", version = "0.1.0")]
impl ServerHandler for Orb {}

fn tool_error(f: Fail) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(format!(
        "{} (exit code {})",
        f.1, f.0
    ))])
}

/// A message as the tool-level error the client shows.
fn error(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message.into())])
}

/// The text of a tool that worked.
fn tool_ok(text: impl Into<String>) -> CallToolResult {
    CallToolResult::success(vec![ContentBlock::text(text.into())])
}

/// Parses the `START:END` of `orb render --range`, with its syntax and limits: two bar numbers,
/// END exclusive, END below the engine's last tick. The bars are returned as ticks.
fn parse_range(s: &str) -> Result<(u32, u32), String> {
    let bad = || format!("--range {s:?}: expected START:END in bars, e.g. 0:4");
    let (a, b) = s.split_once(':').ok_or_else(bad)?;
    let a: u32 = a.trim().parse().map_err(|_| bad())?;
    let b: u32 = b.trim().parse().map_err(|_| bad())?;
    if a >= b {
        return Err(format!("--range {s:?}: START must be below END"));
    }
    if b.saturating_mul(TICKS_PER_BAR) > MAX_TICK {
        return Err(format!(
            "--range {s:?}: END is past bar {}",
            MAX_TICK / TICKS_PER_BAR
        ));
    }
    Ok((a * TICKS_PER_BAR, b * TICKS_PER_BAR))
}


#[tool_router]
impl Orb {
    /// `orb info SONG --json`: what the song file says about itself, read without rendering.
    #[tool(
        description = "Print a song's metadata as File > Info shows it: format and version, \
                       title, text, web page, copyright, the mod and how it resolves, tempo, \
                       shuffle, mode, loop and synth sound. Reads RBS 4.x and ReBirth 1.x songs \
                       without rendering. Same output as `orb info --json`."
    )]
    fn orb_info(&self, Parameters(p): Parameters<SongParams>) -> CallToolResult {
        let path = p.song_path();
        match info::run(&path, &p.mod_opts(), true) {
            Ok(json) => tool_ok(json),
            Err(f) => tool_error(f),
        }
    }

    /// `orb render`: the song's loop (or `range`) as audio, encoded by the output extension.
    #[tool(
        description = "Render a song's loop to an audio file, as the original's \"Export Loop as \
                       Audio File\" does (44.1 kHz, 16-bit stereo): the extension of `output` \
                       picks .wav, .flac or .ogg. Returns the written file, its size and the \
                       number of frames. Same output as `orb render`."
    )]
    fn orb_render(&self, Parameters(p): Parameters<RenderParams>) -> CallToolResult {
        let output = p.song.resolve(&p.output);
        let Some(format) = Format::from_extension(output.extension().and_then(|e| e.to_str()))
        else {
            return error(format!(
                "{}: the output extension must be .wav, .flac or .ogg",
                output.display()
            ));
        };
        let quality = p.quality.unwrap_or(orb_cli::render::DEFAULT_QUALITY);
        if !(-0.2..=1.0).contains(&quality) {
            return error(format!("--quality {quality}: expected -0.2 to 1.0"));
        }
        let idle_blocks = p.idle_blocks.unwrap_or(DEFAULT_IDLE_BLOCKS);
        let path = p.song.song_path();
        let mut song = match orb_cli::render::load_song(&path) {
            Ok(s) => s,
            Err(f) => return tool_error(f),
        };
        if let Err(e) = song_for(&mut song, p.range.as_deref()) {
            return error(e);
        }
        orb_cli::render::warn_unsupported(&song);
        let mut renderer = self.renderer();
        let result = renderer.render_to_file(
            &label(&path),
            &song,
            idle_blocks,
            format,
            quality,
            &output,
        );
        match result {
            Ok(()) => tool_ok(render_summary(&path, &output, idle_blocks)),
            Err(f) => tool_error(f),
        }
    }


    /// `orb compare SONG --reference REF`: the render against a reference, sample by sample.
    #[tool(
        description = "Render a song and compare it with a reference (.flac or .wav) sample by \
                       sample, as `orb compare SONG --reference REF`. Returns EQUAL, or the first \
                       difference (frame, time, bar, channel, both values). This is how a change \
                       to the engine is checked against the reference corpus."
    )]
    fn orb_compare(&self, Parameters(p): Parameters<CompareParams>) -> CallToolResult {
        let path = p.song.song_path();
        let reference = p.song.resolve(&p.reference);
        let idle = p.idle_blocks.unwrap_or(DEFAULT_IDLE_BLOCKS);
        let song = match orb_cli::render::load_song(&path) {
            Ok(s) => s,
            Err(f) => return tool_error(f),
        };
        orb_cli::render::warn_unsupported(&song);
        let got = {
            let mut renderer = self.renderer();
            match renderer.render(&label(&path), &song, idle) {
                Ok(g) => g,
                Err(f) => return tool_error(f),
            }
        };
        match against_reference(&got, &song, &reference) {
            Ok(None) => tool_ok(format!("{}: EQUAL (N {idle})", label(&path))),
            Ok(Some(d)) => error(format!("{}: DIFFERENT (N {idle}): {d}", label(&path))),
            Err(f) => tool_error(f),
        }
    }

    /// The song's model as JSON: the shape `orb_build` takes.
    #[tool(
        description = "Read a song and return its model as JSON, the same text `tools/rbs.py json \
                       SONG` prints. Feed it back to orb_build to write a changed song: leave a \
                       field out to keep the template's value. Reads RBS 4.x and ReBirth 1.x."
    )]
    fn orb_song(&self, Parameters(p): Parameters<SongParams>) -> CallToolResult {
        let path = p.song_path();
        match song::describe(&path) {
            Ok(json) => tool_ok(json),
            Err(f) => tool_error(f),
        }
    }

    /// Writes a song from a spec, as `tools/rbs.py build` does.
    #[tool(
        description = "Write a .rbs song from a JSON spec, merged over the template song and \
                       validated, as `tools/rbs.py build SPEC OUT` does. The spec has the shape \
                       orb_song returns; docs/beats.md explains how to write one. Returns the \
                       written file and its size."
    )]
    fn orb_build(&self, Parameters(p): Parameters<BuildParams>) -> CallToolResult {
        let output = p.resolve(&p.output);
        let spec = match serde_json::to_string(&p.spec) {
            Ok(s) => s,
            Err(e) => return error(format!("the spec is not JSON: {e}")),
        };
        match song::build(&spec, &output) {
            Ok(()) => {
                let bytes = std::fs::metadata(&output).map(|m| m.len()).unwrap_or(0);
                tool_ok(format!("wrote {} ({bytes} bytes)", output.display()))
            }
            Err(f) => tool_error(f),
        }
    }

    /// The mods of a folder: the names `orb render --mod-dir` resolves a song's mod by, and the
    /// file each one lives in.
    #[tool(
        description = "List the .rbm mods of a folder: for each, its INFO name (the name a song \
                       matches, ignoring ASCII case, as the original's Mods menu does) and its \
                       file. This is the folder `orb render --mod-dir` looks a song's mod up in."
    )]
    fn orb_mod_list(&self, Parameters(p): Parameters<ModListParams>) -> CallToolResult {
        let opts = ModOpts {
            mod_dir: p.mod_dir.as_deref().map(|d| p.resolve(d)),
            ..ModOpts::default()
        };
        let dir = match opts.mod_dir() {
            Ok(Some(d)) => d,
            Ok(None) => {
                return error(
                    "no mod folder: give mod_dir, or set ORB_MOD_DIR, or put a Mods folder \
                     next to the orb-mcp binary",
                );
            }
            Err(f) => return tool_error(f),
        };
        match orb_cli::mods::scan(&dir) {
            Ok(entries) => {
                let mut out = format!("{}: {} mod file(s)\n", dir.display(), entries.len());
                for (file, name) in &entries {
                    // A mod whose INFO does not parse is listed with no name: `orb render`
                    // cannot resolve it, and says so here instead of hiding the file.
                    out.push_str(&format!(
                        "{}\t{}\n",
                        name.as_deref().unwrap_or("(no INFO name)"),
                        file.display()
                    ));
                }
                tool_ok(out)
            }
            Err(f) => tool_error(f),
        }
    }
}

/// Checks a render against a reference file the way `orb compare SONG --reference REF` does:
/// `None` when equal, else the first difference as `orb compare` describes it.
fn against_reference(
    got: &[i16],
    song: &orb_engine::song::Song,
    reference: &Path,
) -> Result<Option<String>, Fail> {
    use orb_engine::compare::{describe, first_difference};
    use orb_engine::frame::export::{self, ExportRange};

    let range = ExportRange::from(&song.globals);
    let ext = reference
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let want = match ext.as_deref() {
        Some("flac") => flac_samples(reference)?,
        Some("wav") => {
            let bytes = std::fs::read(reference)
                .map_err(|e| fail(1, format!("{}: cannot read: {e}", reference.display())))?;
            // The WAV `orb render` writes for exactly these samples: equal bytes need no
            // comparison of the samples themselves.
            if bytes == export::wav_bytes(got) {
                return Ok(None);
            }
            let want = wav_samples(reference, &bytes)?;
            if want == got {
                return Ok(Some(
                    "the samples are equal but the WAV header differs from orb's 44-byte one"
                        .into(),
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
    Ok(first_difference(got, &want).map(|d| describe(d, range)))
}


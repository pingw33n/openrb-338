//! Loading a song and rendering it to a file: the part of `rb render`, `rb compare` and `rb play`
//! that does not parse arguments, shared with the UI's export (task-047).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rb_engine::frame::export;
use rb_engine::frame::{Devices, Engine};
use rb_engine::samples::SampleSet;
use rb_engine::song::{DrumPattern, Song};
use rb_engine::tables::Tables;

use crate::encode::{self, Format};
use crate::mods::{self, Kit, ModOpts};
use crate::{Fail, fail};

/// The default initial-state preset of an export: the idle 64-frame blocks run before it starts
/// (the reference corpus' capture protocol).
pub const DEFAULT_IDLE_BLOCKS: u32 = 9088;

/// The default Ogg Vorbis quality.
pub const DEFAULT_QUALITY: f32 = 0.5;

/// Reads and converts an `.rbs` song: an RBS 4.x file or a ReBirth 1.x MIDI file.
pub fn load_song(input: &Path) -> Result<Song, Fail> {
    let data = std::fs::read(input)
        .map_err(|e| fail(1, format!("{}: cannot read the song: {e}", input.display())))?;
    if rbs::rb1x::is_1x(&data) {
        // a ReBirth 1.x song: a MIDI file, imported as Rebirth.exe does (docs/song_load.md 12)
        let parsed = rbs::rb1x::Song1x::parse(&data).map_err(|e| {
            fail(
                1,
                format!("{}: not a valid ReBirth 1.x song: {e}", input.display()),
            )
        })?;
        return rb_engine::load::convert_1x(&parsed)
            .map_err(|e| fail(1, format!("{}: cannot load the song: {e}", input.display())));
    }
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

/// Warns of what the render cannot do as the original would: pattern mode.
pub fn warn_unsupported(song: &Song) {
    if song.globals.song_mode == 0 {
        eprintln!(
            "rb: warning: the song is in pattern mode; the original cannot export it, the \
             output is the engine's song-mode render of the loop"
        );
    }
}

/// The drum machines that play in the song: "TR-808" and "TR-909".
pub fn drums_playing(song: &Song) -> Vec<&'static str> {
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

/// Reads and parses an `.rbm` file.
pub fn read_mod(path: &Path) -> Result<rbm::Rbm, Fail> {
    let bytes = std::fs::read(path)
        .map_err(|e| fail(1, format!("{}: cannot read the mod: {e}", path.display())))?;
    rbm::Rbm::parse(bytes).map_err(|e| fail(1, format!("{}: {e}", path.display())))
}

/// The mod's samples on top of the built-in ones: a slot the mod lacks stays built in. Only a
/// recrypted mod (`rb mod recrypt`) loads: with the key bytes rb embeds. Any other mod is an
/// error that names the fix; rb render, compare and play never read RB20FUL.DAT (task-063).
pub fn load_mod(path: &Path, builtin: &SampleSet) -> Result<SampleSet, Fail> {
    let rbm = read_mod(path)?;
    let Some(table) = rbm::KeyTable::embedded(&rbm).filter(|t| rbm.slots_covered(t)) else {
        return Err(fail(
            1,
            format!(
                "{}: the song plays a drum machine with this mod, which is not recrypted. \
                 Run `rb mod recrypt --rb20ful RB20FUL.DAT {} -o OUT.rbm` once and use OUT.rbm",
                path.display(),
                path.display()
            ),
        ));
    };
    let loaded = rbm
        .load_samples(&table, builtin)
        .map_err(|e| fail(1, format!("{}: {e}", path.display())))?;
    Ok(loaded.set)
}

/// Renders songs, each in a new engine with every device, sharing the tables and the sample
/// sets: the built-in drum samples (the embedded standard mod's, `rbm::standard`) and one set
/// per mod file used.
pub struct Renderer {
    pub tables: Arc<Tables>,
    mods: ModOpts,
    mod_sets: HashMap<PathBuf, Arc<SampleSet>>,
}

impl Renderer {
    pub fn new(mods: ModOpts) -> Self {
        Renderer {
            tables: Arc::new(Tables::new()),
            mods,
            mod_sets: HashMap::new(),
        }
    }

    /// The export of `song` after `idle_blocks` idle blocks: interleaved int16 stereo.
    /// `label` names the song in a warning.
    pub fn render(&mut self, label: &str, song: &Song, idle_blocks: u32) -> Result<Vec<i16>, Fail> {
        let samples = self.samples(label, song)?;
        let mut engine = Engine::new(self.tables.clone(), Devices::implemented(samples));
        let samples = export::export_song(&mut engine, song, idle_blocks)
            .map_err(|r| fail(1, format!("{label}: {r}")))?;
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

    /// The drum samples `song` renders with: its mod's on top of the built-in kit, or the
    /// built-in kit, or none when no drum machine plays.
    pub fn samples(&mut self, label: &str, song: &Song) -> Result<Option<Arc<SampleSet>>, Fail> {
        // The mod is resolved for every song, so that a missing one is an error even when no
        // drum machine plays (and then needs no key file: nothing of it is read).
        let kit = mods::resolve(&self.mods, label, &song.mod_name)?;
        // The drum samples are given only to a song that plays a drum machine; a 303-only song
        // renders without them (its drum machines play no step, so the silent placeholders the
        // engine uses without samples are exact), and a mod it names is never read.
        let samples = if drums_playing(song).is_empty() {
            None
        } else {
            let builtin = rbm::standard::builtin_samples();
            match kit {
                Kit::Standard => Some(builtin),
                Kit::Mod(path) => {
                    if !self.mod_sets.contains_key(&path) {
                        let set = load_mod(&path, &builtin)?;
                        self.mod_sets.insert(path.clone(), Arc::new(set));
                    }
                    Some(self.mod_sets[&path].clone())
                }
            }
        };
        Ok(samples)
    }

    /// Renders `song` and writes it to `output`, encoded by `format` (`rb render`'s output).
    pub fn render_to_file(
        &mut self,
        label: &str,
        song: &Song,
        idle_blocks: u32,
        format: Format,
        quality: f32,
        output: &Path,
    ) -> Result<(), Fail> {
        let samples = self.render(label, song, idle_blocks)?;
        let bytes = encode::encode(format, &samples, quality)
            .map_err(|e| fail(1, format!("encoding failed: {e}")))?;
        std::fs::write(output, bytes)
            .map_err(|e| fail(1, format!("{}: cannot write: {e}", output.display())))
    }
}

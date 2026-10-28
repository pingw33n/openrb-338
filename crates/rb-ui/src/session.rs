//! What the window needs to open: a song file and the built-in drum samples.
//!
//! The song loader is that of `rb play` (`crates/rb-cli/src/main.rs`): an RBS 4.x file or a
//! ReBirth 1.x MIDI-file song. The samples come from the standard mod (`rb mod pack-standard`,
//! docs/rbm.md section 13), found as `rb` finds it: `--standard-mod`, the `RB_STANDARD_MOD`
//! environment variable, or `Standard.rbm` next to the binary. The song's own mod is applied by
//! the window (`App::open_song`, the Mods menu).

use std::path::{Path, PathBuf};

use rb_engine::samples::SampleSet;
use rb_engine::song::Song;

/// A song file read: the engine's song and the `rbs` song it came from (the document's source).
pub struct Opened {
    pub engine: Song,
    pub source: rbs::Song,
}

/// Reads and converts an `.rbs` song, keeping the file's `rbs` song.
pub fn open_song(path: &Path) -> Result<Opened, String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: cannot read: {e}", path.display()))?;
    open_bytes(&data, &path.display().to_string())
}

/// Reads and converts the bytes of a song; `label` names the file in the errors.
pub fn open_bytes(data: &[u8], label: &str) -> Result<Opened, String> {
    if rbs::rb1x::is_1x(data) {
        let parsed = rbs::rb1x::Song1x::parse(data)
            .map_err(|e| format!("{label}: not a valid ReBirth 1.x song: {e}"))?;
        let engine = rb_engine::load::convert_1x(&parsed)
            .map_err(|e| format!("{label}: cannot load the song: {e}"))?;
        let source = crate::document::source_of_1x(&engine);
        return Ok(Opened { engine, source });
    }
    let source =
        rbs::Song::parse(data).map_err(|e| format!("{label}: not a valid .rbs song: {e}"))?;
    let engine = rb_engine::load::convert(&source)
        .map_err(|e| format!("{label}: cannot load the song: {e:?}"))?;
    Ok(Opened { engine, source })
}

/// The name of the demo song a mod carries (docs/rbm.md 13.1; `sub_417000` looks it up).
pub const DEMO: &str = "Demo.rbs";

/// Whether the mod file has an embedded `Demo.rbs`; false for a file that is no mod.
pub fn mod_has_demo(path: &Path) -> bool {
    std::fs::read(path)
        .ok()
        .and_then(|b| rbm::Rbm::parse(b).ok())
        .is_some_and(|r| r.find(DEMO).is_some())
}

/// The bytes of the `Demo.rbs` of a recrypted mod. An error says why there are none (no demo,
/// not a mod, not recrypted).
pub fn mod_demo(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: cannot read: {e}", path.display()))?;
    let rbm = rbm::Rbm::parse(bytes).map_err(|e| format!("{}: not a mod: {e}", path.display()))?;
    if rbm.find(DEMO).is_none() {
        return Err(format!("{}: the mod has no {DEMO}", path.display()));
    }
    let table = rbm::KeyTable::embedded(&rbm)
        .filter(|t| rbm.slots_covered(t))
        .ok_or_else(|| {
            format!(
                "{}: the mod is not recrypted for rb (`rb mod recrypt`); its {DEMO} cannot be read",
                path.display()
            )
        })?;
    rbm.file_by_name(DEMO, &table)
        .map_err(|e| format!("{}: {DEMO}: {e}", path.display()))
}

/// Reads and converts an `.rbs` song.
pub fn load_song(path: &Path) -> Result<Song, String> {
    open_song(path).map(|o| o.engine)
}

/// The INFO chunk of a mod file (its menu name and URLs).
pub fn mod_info(path: &Path) -> Result<rbm::Info, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: cannot read: {e}", path.display()))?;
    let rbm = rbm::Rbm::parse(bytes).map_err(|e| format!("{}: not a mod: {e}", path.display()))?;
    rbm.info()
        .cloned()
        .ok_or_else(|| format!("{}: not a mod: it has no INFO chunk", path.display()))
}

/// Writes `bytes` to `path` through a temporary file next to it, renamed over it, so a failed
/// write leaves the old file.
pub fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, bytes)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            format!("{}: cannot write: {e}", path.display())
        })
}

/// Where the standard mod is: `explicit`, `RB_STANDARD_MOD`, then `Standard.rbm` next to the
/// binary.
pub fn locate_standard_mod(explicit: Option<&Path>) -> Result<PathBuf, String> {
    let env = std::env::var_os("RB_STANDARD_MOD").map(PathBuf::from);
    if let Some((what, p)) = explicit
        .map(|p| ("--standard-mod", p.to_path_buf()))
        .or_else(|| env.map(|p| ("RB_STANDARD_MOD", p)))
    {
        return if p.is_file() {
            Ok(p)
        } else {
            Err(format!("{what} {}: no such file", p.display()))
        };
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("Standard.rbm")))
        .filter(|p| p.is_file())
        .ok_or_else(|| {
            "no standard mod found. Make it once with `rb mod pack-standard --rebirth-dll \
             ReBirth.dll -o Standard.rbm`, then pass --standard-mod FILE or set RB_STANDARD_MOD"
                .to_string()
        })
}

/// The built-in drum samples from the standard mod.
pub fn load_standard(explicit: Option<&Path>) -> Result<SampleSet, String> {
    let path = locate_standard_mod(explicit)?;
    let bytes =
        std::fs::read(&path).map_err(|e| format!("{}: cannot read: {e}", path.display()))?;
    let rbm = rbm::Rbm::parse(bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    if !rbm.is_standard() {
        return Err(format!("{}: not a standard mod", path.display()));
    }
    let table = rbm::KeyTable::embedded(&rbm)
        .filter(|t| rbm.slots_covered(t))
        .ok_or_else(|| {
            format!(
                "{}: the standard mod is not recrypted for rb; make it with `rb mod pack-standard`",
                path.display()
            )
        })?;
    rbm.load_all_samples(&table)
        .map_err(|e| format!("{}: {e}", path.display()))
}

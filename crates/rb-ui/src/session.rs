//! What the window needs to open: a song file and the built-in drum samples.
//!
//! The song loader is that of `rb play` (`crates/rb-cli/src/main.rs`): an RBS 4.x file or a
//! ReBirth 1.x MIDI-file song. The samples come from the standard mod (`rb standard-mod`,
//! docs/rbm.md section 13), found as `rb` finds it: `--standard-mod`, the `RB_STANDARD_MOD`
//! environment variable, or `Standard.rbm` next to the binary. A song's own mod is not applied
//! yet (task-047 adds the mod menu); every song plays with the built-in kit.

use std::path::{Path, PathBuf};

use rb_engine::samples::SampleSet;
use rb_engine::song::Song;

/// Reads and converts an `.rbs` song.
pub fn load_song(path: &Path) -> Result<Song, String> {
    let data = std::fs::read(path).map_err(|e| format!("{}: cannot read: {e}", path.display()))?;
    if rbs::rb1x::is_1x(&data) {
        let parsed = rbs::rb1x::Song1x::parse(&data)
            .map_err(|e| format!("{}: not a valid ReBirth 1.x song: {e}", path.display()))?;
        return rb_engine::load::convert_1x(&parsed)
            .map_err(|e| format!("{}: cannot load the song: {e}", path.display()));
    }
    let parsed = rbs::Song::parse(&data)
        .map_err(|e| format!("{}: not a valid .rbs song: {e}", path.display()))?;
    rb_engine::load::convert(&parsed)
        .map_err(|e| format!("{}: cannot load the song: {e:?}", path.display()))
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
            "no standard mod found. Make it once with `rb standard-mod --rebirth-dll \
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
                "{}: the standard mod is not recrypted for rb; make it with `rb standard-mod`",
                path.display()
            )
        })?;
    rbm.load_all_samples(&table)
        .map_err(|e| format!("{}: {e}", path.display()))
}

//! What the window needs to open: a song file and the built-in drum samples.
//!
//! The song loader is that of `orb play` (`crates/orb-cli/src/main.rs`): an RBS 4.x file or a
//! ReBirth 1.x MIDI-file song. The samples are the built-in kit orb embeds (the standard mod
//! `assets/Standard.rbm`, `rbm::standard::builtin_samples`; docs/rbm.md section 13). The
//! song's own mod is applied by the window (`App::open_song`, the Mods menu).

use std::path::{Path, PathBuf};

use orb_cli::info::SongFacts;
use orb_engine::song::Song;

/// A song file read: the engine's song and the `rbs` song it came from (the document's source).
pub struct Opened {
    pub engine: Song,
    pub source: rbs::Song,
    /// The facts of a ReBirth 1.x song, which has no `rbs` song of its own (`source` is a
    /// stand-in): File > Info shows them (task-123).
    pub facts_1x: Option<SongFacts>,
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
        let engine = orb_engine::load::convert_1x(&parsed)
            .map_err(|e| format!("{label}: cannot load the song: {e}"))?;
        let source = crate::document::source_of_1x(&engine, &parsed);
        let facts_1x = Some(SongFacts::of_1x(&parsed, &engine));
        return Ok(Opened {
            engine,
            source,
            facts_1x,
        });
    }
    let source =
        rbs::Song::parse(data).map_err(|e| format!("{label}: not a valid .rbs song: {e}"))?;
    let engine = orb_engine::load::convert(&source)
        .map_err(|e| format!("{label}: cannot load the song: {e:?}"))?;
    Ok(Opened {
        engine,
        source,
        facts_1x: None,
    })
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
    mod_file(path, DEMO)
}

/// The name of the mod's splash picture (`Splash.jpg`, id 12999; docs/rbm.md 12.1).
pub const SPLASH: &str = "splash.jpg";

/// The bytes of the embedded file `name` (case-insensitive, the first) of a recrypted mod. An
/// error says why there are none (no such file, not a mod, not recrypted).
pub fn mod_file(path: &Path, name: &str) -> Result<Vec<u8>, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: cannot read: {e}", path.display()))?;
    let rbm = rbm::Rbm::parse(bytes).map_err(|e| format!("{}: not a mod: {e}", path.display()))?;
    if rbm.find(name).is_none() {
        return Err(format!("{}: the mod has no {name}", path.display()));
    }
    let table = rbm::KeyTable::embedded(&rbm)
        .filter(|t| rbm.slots_covered(t))
        .ok_or_else(|| {
            format!(
                "{}: the mod is not recrypted for orb (`orb mod recrypt`); its {name} cannot be read",
                path.display()
            )
        })?;
    rbm.file_by_name(name, &table)
        .map_err(|e| format!("{}: {name}: {e}", path.display()))
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

/// The splash picture of the kit that plays, decoded (`App::mod_view`, Mods > Info); None when
/// the kit has none or it does not decode.
pub fn splash_image(kit: &orb_cli::mods::Kit) -> Option<eframe::egui::ColorImage> {
    let bytes = match kit {
        orb_cli::mods::Kit::Mod(p) => mod_file(p, SPLASH).ok()?,
        orb_cli::mods::Kit::Standard => rbm::standard::builtin_file(SPLASH)?,
    };
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg)
        .ok()?
        .to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Some(eframe::egui::ColorImage::from_rgba_unmultiplied(
        size,
        img.as_raw(),
    ))
}

/// What Mods > Info shows of a mod file besides its splash (task-106).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModDetails {
    pub name: String,
    /// `(major, minor, revision)`; None when the mod says "the app's version".
    pub version: Option<(u8, u8, u8)>,
    /// The description, with `\r` and `\r\n` turned into `\n`.
    pub text: String,
    pub web: String,
    pub ftp: String,
    pub file_name: String,
    pub folder: Option<String>,
    pub size: u64,
    /// The number of embedded files (EMBF chunks).
    pub files: usize,
}

impl ModDetails {
    /// The details of the file of a kit: the mod file, or the embedded standard mod.
    pub fn of(kit: &orb_cli::mods::Kit) -> Result<Self, String> {
        match kit {
            orb_cli::mods::Kit::Standard => Self::from_bytes(
                rbm::standard::EMBEDDED.to_vec(),
                "Standard.rbm (built in)".into(),
                None,
            ),
            orb_cli::mods::Kit::Mod(p) => {
                let bytes =
                    std::fs::read(p).map_err(|e| format!("{}: cannot read: {e}", p.display()))?;
                let name = p.file_name().map_or_else(
                    || p.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                let folder = p.parent().map(|d| d.display().to_string());
                Self::from_bytes(bytes, name, folder)
            }
        }
    }

    /// The details of the bytes of a mod file; `file_name` names the file in the errors.
    pub fn from_bytes(
        bytes: Vec<u8>,
        file_name: String,
        folder: Option<String>,
    ) -> Result<Self, String> {
        let size = bytes.len() as u64;
        let rbm = rbm::Rbm::parse(bytes).map_err(|e| format!("{file_name}: not a mod: {e}"))?;
        let info = rbm
            .info()
            .cloned()
            .ok_or_else(|| format!("{file_name}: not a mod: it has no INFO chunk"))?;
        Ok(Self {
            name: info.name,
            version: info.version,
            text: info.text.replace("\r\n", "\n").replace('\r', "\n"),
            web: info.web,
            ftp: info.ftp,
            file_name,
            folder,
            size,
            files: rbm.embedded().count(),
        })
    }
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

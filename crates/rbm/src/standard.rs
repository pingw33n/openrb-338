//! The standard mod (task-036, docs/rbm.md section 13): the original's built-in kit and panel
//! as a mod, built at run time from the user's own `ReBirth.dll` and `Rebirth.exe`.
//!
//! - The 46 samples are the DLL's `"AIFF"` resources 1000..=1045, embedded byte for byte under
//!   the 46 names the exe looks up (docs/rbm.md section 3).
//! - The panel is the exe's `"JPEG"` resources: the 58 numbered bitmaps 12015..=12571 a mod
//!   replaces, embedded as `<id>.jpg`, and resource 903, the built-in kit's splash, as
//!   `splash.jpg`. `sub_41703A` asks an open mod for `%d.jpg` (`Splash.jpg` for id 12999) and
//!   falls back to the exe's resource of that id (903 for the built-in splash, 905 for a mod
//!   without one).
//! - No `Demo.rbs`: the built-in kit has none, and the mod info dialog only enables its "save the
//!   demo song" button when a mod carries one (`sub_417000`).
//!
//! The mod is laid out in the clear and then passed through [`crate::recrypt`], so it is
//! scrambled against the embedded key window like every mod rb loads: rb reads it without
//! `RB20FUL.DAT`, and the original reads it with its own.
//!
//! rb's built-in kit is one such mod, committed as `assets/Standard.rbm` (task-067, built with
//! the CD's Rebirth.exe for the splash; `assets/README.md`) and embedded here as [`EMBEDDED`]:
//! [`builtin_samples`] and [`builtin_file`] read it, so no command needs the original's files.

use std::sync::{Arc, OnceLock};

use rb_engine::samples::{self, Sample, SampleSet, pe};

use crate::{KeyTable, Rbm, RbmError, pack, slot_name};

/// The standard mod's INFO name: "Standard ReBirth", the name songs give the built-in kit (their
/// GLOB mod name; the original's `sub_4166E5` compares with the string at 0x4BB2D0). The
/// embedded mod is rb's built-in kit, so a song made with the built-in kit and the mod name
/// the same kit (task-067; task-036's form, tried in the original, was "ReBirth Standard Kit").
pub const NAME: &str = "Standard ReBirth";

/// The numbered panel bitmaps a mod replaces: the exe's `"JPEG"` resources from this ID on (the
/// IDs below it, 900..=906, are the exe's own splash and about bitmaps).
pub const FIRST_PANEL_ID: u16 = 12000;

/// The exe's `"JPEG"` resource of the built-in kit's splash (`sub_41703A`, `push 387h` at
/// 0x417198).
pub const SPLASH_ID: u16 = 903;

/// The number of numbered panel bitmaps in Rebirth.exe 2.01 (and in every shipped mod).
pub const PANEL_COUNT: usize = 58;

// Changing the text changes the bytes of assets/Standard.rbm (docs/rbm.md 13.2).
const INFO_TEXT: &str = "Standard ReBirth\r\n\r\nThe built-in samples and panel of ReBirth RB-338 \
                         2.01, made into a mod by rb mod pack-standard from ReBirth.dll and \
                         Rebirth.exe.";
const INFO_WEB: &str = "http://www.propellerheads.se/";

/// What went into a standard mod.
#[derive(Debug, Clone)]
pub struct Standard {
    /// The mod file.
    pub bytes: Vec<u8>,
    /// The panel bitmaps embedded, `splash.jpg` included.
    pub jpegs: usize,
}

fn err(m: impl Into<String>) -> RbmError {
    RbmError::Standard(m.into())
}

/// The 1280-byte INFO of the standard mod (docs/rbm.md section 2).
fn info() -> [u8; 1280] {
    let mut b = [0u8; 1280];
    b[..INFO_TEXT.len()].copy_from_slice(INFO_TEXT.as_bytes());
    b[802..802 + INFO_WEB.len()].copy_from_slice(INFO_WEB.as_bytes());
    b[1003..1003 + NAME.len()].copy_from_slice(NAME.as_bytes());
    // version 1.0.0 as PBE2, MSM2 and Orbit2 carry it; the app compares the major (0x415178)
    b[1073..1077].copy_from_slice(&[0xBC, 0x01, 0x00, 0x00]);
    b
}

/// Builds the standard mod from the bytes of `ReBirth.dll` (the samples) and `Rebirth.exe`
/// (the panel).
pub fn build(dll: &[u8], exe: &[u8]) -> Result<Standard, RbmError> {
    let aiff =
        pe::named_type_resources(dll, "AIFF").map_err(|e| err(format!("ReBirth.dll: {e}")))?;
    let jpeg =
        pe::named_type_resources(exe, "JPEG").map_err(|e| err(format!("Rebirth.exe: {e}")))?;

    let mut panel: Vec<&pe::Resource> = jpeg.iter().filter(|r| r.id >= FIRST_PANEL_ID).collect();
    panel.sort_by_key(|r| r.id);
    panel.dedup_by_key(|r| r.id);
    if panel.len() != PANEL_COUNT {
        return Err(err(format!(
            "Rebirth.exe: {} panel bitmaps (JPEG resources from {FIRST_PANEL_ID}), not the \
             {PANEL_COUNT} of ReBirth 2.01",
            panel.len()
        )));
    }
    let splash = jpeg.iter().find(|r| r.id == SPLASH_ID).ok_or_else(|| {
        err(format!(
            "Rebirth.exe: no JPEG resource {SPLASH_ID} (the splash)"
        ))
    })?;
    for r in panel.iter().copied().chain(std::iter::once(splash)) {
        if !r.data.starts_with(&[0xFF, 0xD8]) {
            return Err(err(format!(
                "Rebirth.exe: JPEG resource {} is not a JPEG",
                r.id
            )));
        }
    }

    let names: Vec<String> = panel.iter().map(|r| format!("{}.jpg", r.id)).collect();
    let mut files: Vec<(&str, &[u8])> = names
        .iter()
        .zip(&panel)
        .map(|(n, r)| (n.as_str(), r.data))
        .collect();
    files.push(("splash.jpg", splash.data));
    // The samples in slot order (808, 909, the startup sound), as the DLL's table.
    let ids = (samples::FIRST_ID..samples::STARTUP_ID)
        .chain(samples::STARTUP_ID + 1..=samples::LAST_ID)
        .chain(std::iter::once(samples::STARTUP_ID));
    for id in ids {
        let r = aiff
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| err(format!("ReBirth.dll: the AIFF resource {id} is missing")))?;
        // the engine's reader must take it, as it takes the DLL's
        Sample::from_aiff(r.data).map_err(|e| err(format!("ReBirth.dll: AIFF {id}: {e}")))?;
        files.push((slot_name(id).expect("id in range"), r.data));
    }
    let bytes = pack::pack_parts(&pack::standard_head(), &files, &info())?;
    Ok(Standard {
        bytes,
        jpegs: PANEL_COUNT + 1,
    })
}

/// `assets/Standard.rbm`: the standard mod rb embeds as its built-in kit (task-067).
pub const EMBEDDED: &[u8] = include_bytes!("../../../assets/Standard.rbm");

/// The embedded standard mod, parsed once, and its key table.
fn embedded_mod() -> &'static (Rbm, KeyTable) {
    static M: OnceLock<(Rbm, KeyTable)> = OnceLock::new();
    M.get_or_init(|| {
        let r = Rbm::parse(EMBEDDED.to_vec()).expect("assets/Standard.rbm parses");
        let t = KeyTable::embedded(&r)
            .filter(|t| r.all_covered(t))
            .expect("assets/Standard.rbm is recrypted");
        (r, t)
    })
}

/// The built-in drum samples: the 46 samples of the embedded standard mod, loaded once.
pub fn builtin_samples() -> Arc<SampleSet> {
    static S: OnceLock<Arc<SampleSet>> = OnceLock::new();
    S.get_or_init(|| {
        let (r, t) = embedded_mod();
        Arc::new(
            r.load_all_samples(t)
                .expect("assets/Standard.rbm has every slot"),
        )
    })
    .clone()
}

/// The embedded file `name` of the built-in kit (case-insensitive, the first; `splash.jpg`,
/// `12015.jpg`, a sample), decrypted; `None` when the kit has no such file.
pub fn builtin_file(name: &str) -> Option<Vec<u8>> {
    let (r, t) = embedded_mod();
    r.find(name)?;
    r.file_by_name(name, t).ok()
}

impl Rbm {
    /// Whether this is a standard mod: its INFO name is [`NAME`].
    pub fn is_standard(&self) -> bool {
        self.info().is_some_and(|i| i.name == NAME)
    }

    /// The 46 samples of a mod that has every slot, such as the standard mod: the built-in kit.
    /// A missing slot is [`RbmError::NoSuchFile`]; the table must cover every slot.
    pub fn load_all_samples(&self, table: &KeyTable) -> Result<SampleSet, RbmError> {
        let mut out = Vec::with_capacity(samples::COUNT);
        for id in samples::FIRST_ID..=samples::LAST_ID {
            let name = slot_name(id).expect("id in range");
            let bytes = self.file_by_name(name, table)?;
            let s = Sample::from_aiff(&bytes)
                .map_err(|e| RbmError::Sample(name.to_string(), e.to_string()))?;
            out.push(Arc::new(s));
        }
        Ok(SampleSet::from_shared(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_fields_parse() {
        let m = pack::pack_parts(&pack::standard_head(), &[], &info()).unwrap();
        let r = Rbm::parse(m).unwrap();
        let i = r.info().unwrap();
        assert_eq!(i.name, NAME);
        assert_eq!(i.version, Some((1, 0, 0)));
        assert_eq!(i.web, INFO_WEB);
        assert_eq!(i.ftp, "");
        assert!(i.text.starts_with(NAME));
        assert!(r.is_standard());
        assert!(INFO_TEXT.len() < 600 && NAME.len() < 64);
    }

    #[test]
    fn the_embedded_kit_is_the_standard_mod() {
        let (r, _) = embedded_mod();
        assert!(r.is_standard());
        assert_eq!(r.info().unwrap().text, INFO_TEXT);
        assert_eq!(r.chunks().len(), 108);
        let s = builtin_samples();
        assert_eq!(s.get(samples::FIRST_ID).frames(), 72704);
        assert_eq!(s.get(samples::LAST_ID).frames(), 42458);
        assert!(
            builtin_file("Splash.jpg")
                .unwrap()
                .starts_with(&[0xFF, 0xD8])
        );
        assert!(builtin_file("Demo.rbs").is_none());
    }

    #[test]
    fn not_a_pe_is_an_error() {
        assert!(matches!(
            build(b"nope", b"nope"),
            Err(RbmError::Standard(_))
        ));
    }
}

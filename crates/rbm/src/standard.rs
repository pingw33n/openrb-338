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

use std::sync::Arc;

use rb_engine::samples::{self, Sample, SampleSet, pe};

use crate::{KeyTable, Rbm, RbmError, TABLE_LEN, recrypt, slot_name};

/// The standard mod's name in the Mods menu. "Standard ReBirth" is the name the original and rb
/// give the built-in kit itself (`sub_4166E5`), so the mod has a name of its own.
pub const NAME: &str = "ReBirth Standard Kit";

/// The numbered panel bitmaps a mod replaces: the exe's `"JPEG"` resources from this ID on (the
/// IDs below it, 900..=906, are the exe's own splash and about bitmaps).
pub const FIRST_PANEL_ID: u16 = 12000;

/// The exe's `"JPEG"` resource of the built-in kit's splash (`sub_41703A`, `push 387h` at
/// 0x417198).
pub const SPLASH_ID: u16 = 903;

/// The number of numbered panel bitmaps in Rebirth.exe 2.01 (and in every shipped mod).
pub const PANEL_COUNT: usize = 58;

/// The first bytes of `HEAD` in every shipped mod (nothing reads them; docs/rbm.md section 2),
/// then the copyright line they carry.
const HEAD_LEAD: [u8; 9] = [0x5B, 0x35, 0xB3, 0x5B, 0xBC, 0x01, 0x00, 0x00, 0x00];
const HEAD_TEXT: &str = "(c)1998 Propellerhead Software, all rights reserved.";

const INFO_TEXT: &str = "ReBirth Standard Kit\r\n\r\nThe built-in samples and panel of ReBirth RB-338 \
                         2.01, made into a mod by rb standard-mod from ReBirth.dll and \
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

fn push_chunk(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(tag);
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
}

fn embf(out: &mut Vec<u8>, name: &str, file: &[u8]) {
    let mut p = Vec::with_capacity(name.len() + 1 + file.len());
    p.extend_from_slice(name.as_bytes());
    p.push(0);
    p.extend_from_slice(file);
    push_chunk(out, b"EMBF", &p);
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

    let mut out = b"CAT \0\0\0\0PRBM".to_vec();
    let mut head = vec![0u8; 256];
    head[..HEAD_LEAD.len()].copy_from_slice(&HEAD_LEAD);
    head[HEAD_LEAD.len()..HEAD_LEAD.len() + HEAD_TEXT.len()].copy_from_slice(HEAD_TEXT.as_bytes());
    push_chunk(&mut out, b"HEAD", &head);
    for r in &panel {
        embf(&mut out, &format!("{}.jpg", r.id), r.data);
    }
    embf(&mut out, "splash.jpg", splash.data);
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
        embf(&mut out, slot_name(id).expect("id in range"), r.data);
    }
    push_chunk(&mut out, b"INFO", &info());
    let size = (out.len() - 8) as u32;
    out[4..8].copy_from_slice(&size.to_be_bytes());

    // In the clear: a zero table descrambles to the bytes as they are.
    let clear = Rbm::parse(out)?;
    let zero = KeyTable::from_bytes(&[0u8; TABLE_LEN])?;
    let bytes = recrypt(&clear, &zero)?;
    Ok(Standard {
        bytes,
        jpegs: PANEL_COUNT + 1,
    })
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
        let mut m = b"CAT \0\0\0\0PRBM".to_vec();
        push_chunk(&mut m, b"INFO", &info());
        let size = (m.len() - 8) as u32;
        m[4..8].copy_from_slice(&size.to_be_bytes());
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
    fn not_a_pe_is_an_error() {
        assert!(matches!(
            build(b"nope", b"nope"),
            Err(RbmError::Standard(_))
        ));
    }
}
